//! Port fidèle de `PocketMine-MP/src/network/mcpe/InventoryManager.php`.
//!
//! Tracking par-session des inventaires permanents (main, offhand, armor, UI cursor,
//! UI crafting 2x2) plus les fenêtres dynamiquement ouvertes. Gère :
//! - l'attribution des stackId uniques (`next_item_stack_id`) indispensables aux
//!   ItemStackRequest côté client,
//! - le two-phase sync (clear-all-in-air puis vrai contenu) requis depuis Bedrock
//!   1.20.12 sinon le client ignore les changements de stackId → crash à l'ouverture,
//! - le handshake d'ouverture/fermeture (`pending_close_window_id` + callback différé)
//!   obligatoire sinon ouvrir une nouvelle fenêtre pendant qu'une autre se ferme
//!   provoque des bugs client.
//!
//! Les méthodes n'envoient rien directement : elles poussent des paires
//! `(packet_id, payload_non_compressé)` dans un `Vec` que la `Connection`
//! compresse avant envoi. Ça évite les conflits d'emprunt avec `Connection`.

use std::collections::HashMap;

use mc_rs_proto::packets::packet_id;
use mc_rs_proto::packets::player::{
    FullContainerName, InventoryContent, InventorySlot, ItemStack, ItemStackRequest,
    ItemStackWrapper, MobEquipment, SlotInfo, StackRequestAction,
};
use mc_rs_proto::packets::world::{
    ContainerClose, ContainerOpen, ItemStackResponse, ItemStackResponseContainer,
    ItemStackResponseSlot,
};

use crate::inventory::PlayerInventory;

// ── Constantes PMMP ──────────────────────────────────────────────────────────
// `vendor/pocketmine/bedrock-protocol/src/types/inventory/ContainerIds.php`
pub mod container_ids {
    pub const NONE: i8 = -1;
    pub const INVENTORY: u8 = 0;
    pub const FIRST: u8 = 1;
    pub const LAST: u8 = 100;
    pub const OFFHAND: u8 = 119;
    pub const ARMOR: u8 = 120;
    pub const UI: u8 = 124;
}

// `vendor/pocketmine/bedrock-protocol/src/types/inventory/WindowTypes.php`
pub mod window_types {
    pub const INVENTORY: i8 = -1;
    pub const CONTAINER: i8 = 0;
}

// `src/network/mcpe/handler/UIInventorySlotOffset.php`
pub mod ui_slot {
    pub const CURSOR: u32 = 0;
    // CRAFTING2X2_INPUT = [28 => 0, 29 => 1, 30 => 2, 31 => 3]
    pub const CRAFTING2X2_INPUT_START: u32 = 28;
    pub const CRAFTING_RESULT: u32 = 50;
}

// `vendor/.../ContainerUIIds.php` — IDs envoyés par le client dans
// `ItemStackRequestSlotInfo.containerName.containerId`.
pub mod ui_id {
    pub const ARMOR: u8 = 6;
    pub const COMBINED_HOTBAR_AND_INVENTORY: u8 = 12;
    pub const HOTBAR: u8 = 28;
    pub const INVENTORY: u8 = 29;
    pub const OFFHAND: u8 = 34;
    pub const CURSOR: u8 = 59;
    pub const CRAFTING_INPUT: u8 = 13;
    pub const CREATED_OUTPUT: u8 = 60;
}

// ── Types internes ───────────────────────────────────────────────────────────

/// Identifie chaque inventaire logique trackable. Remplace le `spl_object_id` PHP.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InvKey {
    Main,
    Offhand,
    Armor,
    Cursor,
    Craft2x2,
    CraftResult,
}

/// Équivalent de `ItemStackInfo` PMMP — lie un slot à un stackId unique et au
/// `requestId` qui a causé la modification (si applicable).
#[derive(Debug, Clone, Copy)]
pub struct ItemStackInfo {
    pub request_id: Option<i32>,
    pub stack_id: i32,
}

/// Équivalent de `InventoryManagerEntry` PMMP.
#[derive(Default)]
pub struct InvEntry {
    pub item_stack_infos: HashMap<usize, ItemStackInfo>,
    pub predictions: HashMap<usize, ItemStack>,
    pub pending_syncs: HashMap<usize, ItemStack>,
    /// `Some((net→core, core→net))` si l'inventaire est branché sur l'UI(124).
    pub complex_slot_map: Option<(HashMap<u32, usize>, HashMap<usize, u32>)>,
}

pub type PacketOut = Vec<(u32, Vec<u8>)>;

pub struct InventoryManager {
    pub inventories: HashMap<InvKey, InvEntry>,
    pub network_id_to_key: HashMap<u8, InvKey>,
    pub complex_slot_to_key: HashMap<u32, InvKey>,

    pub last_inventory_network_id: u8,
    pub current_window_type: i8,

    pub next_item_stack_id: i32,
    pub current_item_stack_request_id: Option<i32>,

    pub pending_close_window_id: Option<u8>,
    /// Simplification du `Closure`-based deferred call PMMP : on ne supporte
    /// pour l'instant qu'un seul type de deferred (ouverture main inv).
    pub pending_open_main_inventory: Option<i64>,

    pub client_selected_hotbar_slot: i32,
    pub full_sync_requested: bool,
}

impl InventoryManager {
    pub fn new() -> Self {
        let mut mgr = Self {
            inventories: HashMap::new(),
            network_id_to_key: HashMap::new(),
            complex_slot_to_key: HashMap::new(),
            last_inventory_network_id: container_ids::FIRST,
            current_window_type: window_types::CONTAINER,
            next_item_stack_id: 1,
            current_item_stack_request_id: None,
            pending_close_window_id: None,
            pending_open_main_inventory: None,
            client_selected_hotbar_slot: -1,
            full_sync_requested: false,
        };
        // PMMP `__construct` :
        //   add(INVENTORY, main); add(OFFHAND, offhand); add(ARMOR, armor);
        //   addComplex(UIInventorySlotOffset::CURSOR, cursor);
        //   addComplex(UIInventorySlotOffset::CRAFTING2X2_INPUT, craftGrid);
        mgr.add(container_ids::INVENTORY, InvKey::Main);
        mgr.add(container_ids::OFFHAND, InvKey::Offhand);
        mgr.add(container_ids::ARMOR, InvKey::Armor);
        mgr.add_complex(vec![(ui_slot::CURSOR, 0)], InvKey::Cursor);
        mgr.add_complex(
            (0..4)
                .map(|i| (ui_slot::CRAFTING2X2_INPUT_START + i, i as usize))
                .collect(),
            InvKey::Craft2x2,
        );
        mgr
    }

    fn new_item_stack_id(&mut self) -> i32 {
        let id = self.next_item_stack_id;
        self.next_item_stack_id += 1;
        id
    }

    fn associate_id_with_key(&mut self, id: u8, key: InvKey) {
        self.network_id_to_key.insert(id, key);
    }

    fn add(&mut self, id: u8, key: InvKey) {
        self.inventories.entry(key).or_default();
        self.associate_id_with_key(id, key);
    }

    fn add_complex(&mut self, slot_map_pairs: Vec<(u32, usize)>, key: InvKey) {
        let mut net_to_core = HashMap::new();
        let mut core_to_net = HashMap::new();
        for (net, core) in &slot_map_pairs {
            net_to_core.insert(*net, *core);
            core_to_net.insert(*core, *net);
            self.complex_slot_to_key.insert(*net, key);
        }
        let entry = InvEntry {
            complex_slot_map: Some((net_to_core, core_to_net)),
            ..Default::default()
        };
        self.inventories.insert(key, entry);
    }

    /// PMMP `getNewWindowId()` : `max(FIRST, (last + 1) % LAST)`.
    fn get_new_window_id(&mut self) -> u8 {
        let next = (self.last_inventory_network_id as u16 + 1) % container_ids::LAST as u16;
        self.last_inventory_network_id = (next as u8).max(container_ids::FIRST);
        self.last_inventory_network_id
    }

    fn full_container_name(&self) -> FullContainerName {
        // PMMP InventoryManager.php:524,533,554,558 — passe toujours
        // `lastInventoryNetworkId`. Au spawn avant toute ouverture c'est
        // `ContainerIds::FIRST = 1`. Après ouverture d'une fenêtre c'est
        // l'ID dynamique attribué.
        FullContainerName::new(self.last_inventory_network_id)
    }

    /// `sendInventoryContentPackets()` — PMMP two-phase (InventoryManager.php:541-558).
    ///
    /// Depuis Bedrock 1.20.12, le client ignore le changement de stackId quand
    /// l'item de surface est identique. Il faut d'abord envoyer tout en air pour
    /// forcer le client à oublier les anciens stackIds, puis le vrai contenu avec
    /// les nouveaux stackIds. Sans ça → désynchro silencieuse → crash à l'ouverture.
    /// (Le commentaire précédent prétendait que dragonfly single-phase fonctionnait
    /// sur 944, mais en pratique ça crash le client.)
    fn send_inventory_content_packets(
        &self,
        window_id: u32,
        wrappers: &[ItemStackWrapper],
        out: &mut PacketOut,
    ) {
        let fcn = self.full_container_name();
        // Phase 1 : tout en air pour forcer l'oubli des anciens stackIds.
        let air_wrappers: Vec<ItemStackWrapper> =
            (0..wrappers.len()).map(|_| ItemStackWrapper::air()).collect();
        out.push((
            packet_id::INVENTORY_CONTENT,
            InventoryContent::encode_items(window_id, &air_wrappers, &fcn),
        ));
        // Phase 2 : vrai contenu avec les nouveaux stackIds.
        out.push((
            packet_id::INVENTORY_CONTENT,
            InventoryContent::encode_items(window_id, wrappers, &fcn),
        ));
    }

    /// `sendInventorySlotPackets()` — PMMP two-phase (InventoryManager.php:510-537).
    /// Le bug 1.20.12+ touche armor, offhand, enchanting — si `stackId != 0`, on
    /// clear le slot d'abord puis on envoie le vrai contenu.
    fn send_inventory_slot_packets(
        &self,
        window_id: u32,
        net_slot: u32,
        wrapper: &ItemStackWrapper,
        out: &mut PacketOut,
    ) {
        let fcn = self.full_container_name();
        if wrapper.stack_id != 0 {
            let air = ItemStackWrapper::air();
            out.push((
                packet_id::INVENTORY_SLOT,
                InventorySlot::encode(window_id, net_slot, &air, &fcn),
            ));
        }
        out.push((
            packet_id::INVENTORY_SLOT,
            InventorySlot::encode(window_id, net_slot, wrapper, &fcn),
        ));
    }

    fn track_item_stack(
        &mut self,
        key: InvKey,
        slot: usize,
        is_air: bool,
        request_id: Option<i32>,
    ) -> ItemStackInfo {
        let stack_id = if is_air { 0 } else { self.new_item_stack_id() };
        let info = ItemStackInfo {
            request_id,
            stack_id,
        };
        self.inventories
            .entry(key)
            .or_default()
            .item_stack_infos
            .insert(slot, info);
        info
    }

    /// Équivalent de `syncContents()` — envoie le contenu complet d'un inventaire.
    pub fn sync_contents(&mut self, inv: &PlayerInventory, key: InvKey, out: &mut PacketOut) {
        let size = PlayerInventory::inventory_size(key);

        // Reset predictions & pending_syncs AVANT de re-tracker (PMMP le fait aussi).
        if let Some(e) = self.inventories.get_mut(&key) {
            e.predictions.clear();
            e.pending_syncs.clear();
        }

        // Collecte les wrappers avec des stackId fraîchement générés.
        let mut contents: Vec<ItemStackWrapper> = Vec::with_capacity(size);
        for slot in 0..size {
            let item_wrapper = inv
                .slot_ref(key, slot)
                .cloned()
                .unwrap_or_else(ItemStackWrapper::air);
            let info = self.track_item_stack(key, slot, item_wrapper.item.is_air(), None);
            contents.push(ItemStackWrapper {
                stack_id: info.stack_id,
                item: item_wrapper.item,
            });
        }

        // Complex vs simple.
        let (is_complex, complex_map) = {
            let e = self.inventories.get(&key).unwrap();
            (
                e.complex_slot_map.is_some(),
                e.complex_slot_map.clone(),
            )
        };

        if is_complex {
            // PMMP envoie *par slot individuel* pour les complex maps.
            let (_net_to_core, core_to_net) = complex_map.unwrap();
            for (core_slot, wrapper) in contents.iter().enumerate() {
                if let Some(&net_slot) = core_to_net.get(&core_slot) {
                    self.send_inventory_slot_packets(
                        container_ids::UI as u32,
                        net_slot,
                        wrapper,
                        out,
                    );
                }
            }
        } else {
            // Trouver le windowId associé à la key (la plus basse).
            if let Some(window_id) = self.window_id_for(key) {
                self.send_inventory_content_packets(window_id as u32, &contents, out);
            }
        }
    }

    fn window_id_for(&self, key: InvKey) -> Option<u8> {
        self.network_id_to_key
            .iter()
            .find(|(_, k)| **k == key)
            .map(|(id, _)| *id)
    }

    /// `syncAll()` — resync tous les inventaires enregistrés.
    ///
    /// Protocol 944 (dragonfly) : les inventaires non-UI (Main, Offhand, Armor)
    /// reçoivent chacun un InventoryContent. La fenêtre UI (124) reçoit
    /// **un seul** InventoryContent de 54 slots où cursor (0), craft grid
    /// (28..32) et craft result (50) sont aux bonnes positions, le reste en air.
    /// Sans ces 54 slots le client 1.26.10 crash à l'ouverture de l'inventaire.
    pub fn sync_all(&mut self, inv: &PlayerInventory, out: &mut PacketOut) {
        // Ordre exact dragonfly (session.go:255-258) :
        //   sendInv(inv, WindowIDInventory=0)
        //   sendInv(ui,  WindowIDUI=124)
        //   sendInv(offHand, WindowIDOffHand=119)
        //   sendInv(armour, WindowIDArmour=120)
        // Pas de MobEquipment au spawn (dragonfly ne l'envoie qu'au changement).
        self.sync_contents_non_ui(inv, InvKey::Main, out);
        self.sync_ui_inventory(inv, out);
        self.sync_contents_non_ui(inv, InvKey::Offhand, out);
        self.sync_contents_non_ui(inv, InvKey::Armor, out);
    }

    /// Sync un inventaire "simple" (non mappé sur UI). Envoie un InventoryContent
    /// complet sur le windowId associé (0, 119 ou 120).
    fn sync_contents_non_ui(
        &mut self,
        inv: &PlayerInventory,
        key: InvKey,
        out: &mut PacketOut,
    ) {
        let size = PlayerInventory::inventory_size(key);
        if let Some(e) = self.inventories.get_mut(&key) {
            e.predictions.clear();
            e.pending_syncs.clear();
        }
        let mut contents = Vec::with_capacity(size);
        for slot in 0..size {
            let item_wrapper = inv
                .slot_ref(key, slot)
                .cloned()
                .unwrap_or_else(ItemStackWrapper::air);
            let info = self.track_item_stack(key, slot, item_wrapper.item.is_air(), None);
            contents.push(ItemStackWrapper {
                stack_id: info.stack_id,
                item: item_wrapper.item,
            });
        }
        if let Some(window_id) = self.window_id_for(key) {
            self.send_inventory_content_packets(window_id as u32, &contents, out);
        }
    }

    /// Sync de la fenêtre UI (124) en un seul paquet de 54 slots.
    /// Layout (dragonfly/gophertunnel 944) :
    ///   slot 0       → cursor
    ///   slots 28..31 → crafting grid 2x2
    ///   slot 50      → crafting result
    ///   tous les autres → air
    fn sync_ui_inventory(&mut self, inv: &PlayerInventory, out: &mut PacketOut) {
        const UI_SIZE: usize = 54;
        // Nettoie les prédictions/pending_syncs des inventaires UI-backed.
        for key in [InvKey::Cursor, InvKey::Craft2x2, InvKey::CraftResult] {
            if let Some(e) = self.inventories.get_mut(&key) {
                e.predictions.clear();
                e.pending_syncs.clear();
            }
        }

        // Tous les slots à air par défaut.
        let mut contents: Vec<ItemStackWrapper> = (0..UI_SIZE)
            .map(|_| ItemStackWrapper::air())
            .collect();

        // Place cursor en slot 0.
        let cursor_item = inv
            .slot_ref(InvKey::Cursor, 0)
            .cloned()
            .unwrap_or_else(ItemStackWrapper::air);
        let info = self.track_item_stack(InvKey::Cursor, 0, cursor_item.item.is_air(), None);
        contents[ui_slot::CURSOR as usize] = ItemStackWrapper {
            stack_id: info.stack_id,
            item: cursor_item.item,
        };

        // Craft grid 2x2 (slots 28..32).
        for core in 0..4 {
            let item = inv
                .slot_ref(InvKey::Craft2x2, core)
                .cloned()
                .unwrap_or_else(ItemStackWrapper::air);
            let info = self.track_item_stack(InvKey::Craft2x2, core, item.item.is_air(), None);
            let net_slot = ui_slot::CRAFTING2X2_INPUT_START as usize + core;
            contents[net_slot] = ItemStackWrapper {
                stack_id: info.stack_id,
                item: item.item,
            };
        }

        // Craft result (slot 50) — généralement air au spawn.
        let result = inv
            .slot_ref(InvKey::CraftResult, 0)
            .cloned()
            .unwrap_or_else(ItemStackWrapper::air);
        let info = self.track_item_stack(InvKey::CraftResult, 0, result.item.is_air(), None);
        contents[ui_slot::CRAFTING_RESULT as usize] = ItemStackWrapper {
            stack_id: info.stack_id,
            item: result.item,
        };

        self.send_inventory_content_packets(container_ids::UI as u32, &contents, out);
    }

    /// `syncSelectedHotbarSlot()` — MobEquipment avec le stackId tracké du slot main.
    pub fn sync_selected_hotbar_slot(
        &mut self,
        inv: &PlayerInventory,
        entity_runtime_id: u64,
        out: &mut PacketOut,
    ) {
        let selected = inv.held_slot as i32;
        if selected == self.client_selected_hotbar_slot {
            return;
        }
        let stack_id = self
            .inventories
            .get(&InvKey::Main)
            .and_then(|e| e.item_stack_infos.get(&(selected as usize)))
            .map(|i| i.stack_id)
            .unwrap_or(0);
        let held = inv
            .slot_ref(InvKey::Main, selected as usize)
            .cloned()
            .unwrap_or_else(ItemStackWrapper::air);
        let wrapper = ItemStackWrapper {
            stack_id,
            item: held.item,
        };
        out.push((
            packet_id::MOB_EQUIPMENT,
            MobEquipment::encode_item(entity_runtime_id, &wrapper, selected as u8),
        ));
        self.client_selected_hotbar_slot = selected;
    }

    /// `onClientSelectHotbarSlot()` — le client vient de sélectionner un slot hotbar.
    pub fn on_client_select_hotbar_slot(&mut self, slot: i32) {
        self.client_selected_hotbar_slot = slot;
    }

    /// `onClientOpenMainInventory()` — touche E côté client.
    /// PMMP `InventoryManager.php:394-408` :
    ///   1. `onCurrentWindowRemove()` — ferme la fenêtre courante si besoin
    ///   2. `openWindowDeferred(callback)` — si close pending, store callback
    ///   3. callback exécuté : `windowId = getNewWindowId()`,
    ///      `associateIdWithInventory`, `currentWindowType = INVENTORY (-1)`,
    ///      envoie `entityInv(windowId, -1, player.getId())` avec pos=(0,0,0)
    pub fn on_client_open_main_inventory(
        &mut self,
        player_entity_id: i64,
        out: &mut PacketOut,
    ) {
        self.on_current_window_remove(out);
        if self.pending_close_window_id.is_some() {
            // ACK d'une fermeture en attente : diffère l'ouverture.
            self.pending_open_main_inventory = Some(player_entity_id);
            return;
        }
        self.do_open_main_inventory(player_entity_id, out);
    }

    /// Partie « callback » de `onClientOpenMainInventory` — exécutée soit
    /// immédiatement soit après l'ACK de close.
    fn do_open_main_inventory(&mut self, player_entity_id: i64, out: &mut PacketOut) {
        let window_id = self.get_new_window_id();
        self.associate_id_with_key(window_id, InvKey::Main);
        self.current_window_type = window_types::INVENTORY;
        out.push((
            packet_id::CONTAINER_OPEN,
            ContainerOpen {
                window_id,
                window_type: 0xFF, // -1 = INVENTORY
                position: [0, 0, 0],
                actor_unique_id: player_entity_id,
            }
            .encode(),
        ));
    }

    /// `onCurrentWindowRemove()` — le serveur ferme la fenêtre courante.
    /// Si une fenêtre dynamique était ouverte, émet ContainerClose server-initiated
    /// et passe en attente du ack client.
    pub fn on_current_window_remove(&mut self, out: &mut PacketOut) {
        let id = self.last_inventory_network_id;
        // PMMP check: "si networkIdToInventoryMap[last] existe". On reproduit :
        // uniquement si `id` correspond à une fenêtre dynamique (≥ FIRST, ≤ LAST)
        // distincte des windowIds permanents (0/119/120/124).
        if id >= container_ids::FIRST
            && id <= container_ids::LAST
            && self.network_id_to_key.contains_key(&id)
        {
            // remove
            if let Some(key) = self.network_id_to_key.remove(&id) {
                // Si l'inventaire n'a plus aucun windowId associé, le retirer du tracking.
                // Mais main/offhand/armor/cursor/craft sont permanents — on NE les retire pas.
                let still_referenced = self.network_id_to_key.values().any(|k| *k == key);
                if !still_referenced && !matches!(
                    key,
                    InvKey::Main | InvKey::Offhand | InvKey::Armor | InvKey::Cursor | InvKey::Craft2x2
                ) {
                    self.inventories.remove(&key);
                }
            }
            out.push((
                packet_id::CONTAINER_CLOSE,
                ContainerClose {
                    window_id: id,
                    window_type: self.current_window_type as u8,
                    server: true,
                }
                .encode(),
            ));
            self.pending_close_window_id = Some(id);
        }
    }

    /// `onClientRemoveWindow()` — le client a fermé une fenêtre (ou ACK une fermeture serveur).
    pub fn on_client_remove_window(&mut self, raw_id: u8, out: &mut PacketOut) {
        // HACK PMMP (1.21.100+): le client envoie parfois -1 (0xFF) pour rejeter l'ouverture
        // d'une fenêtre. Dans ce cas on considère que c'est `lastInventoryNetworkId`.
        let id = if raw_id as i8 == container_ids::NONE {
            self.last_inventory_network_id
        } else {
            raw_id
        };

        if id == self.last_inventory_network_id
            && self.network_id_to_key.contains_key(&id)
            && Some(id) != self.pending_close_window_id
        {
            self.network_id_to_key.remove(&id);
        }

        // Toujours renvoyer un ContainerClose (echo, server=false).
        out.push((
            packet_id::CONTAINER_CLOSE,
            ContainerClose {
                window_id: id,
                window_type: self.current_window_type as u8,
                server: false,
            }
            .encode(),
        ));

        if self.pending_close_window_id == Some(id) {
            self.pending_close_window_id = None;
            // PMMP `onClientRemoveWindow` lignes 445-449 : exécuter le callback
            // d'ouverture différée si présent.
            if let Some(player_entity_id) = self.pending_open_main_inventory.take() {
                self.do_open_main_inventory(player_entity_id, out);
            }
        }
    }

    pub fn set_current_item_stack_request_id(&mut self, id: Option<i32>) {
        self.current_item_stack_request_id = id;
    }

    /// Retourne le `stackId` tracké pour `(key, slot)`, 0 si non tracké ou air.
    pub fn stack_id_of(&self, key: InvKey, slot: usize) -> i32 {
        self.inventories
            .get(&key)
            .and_then(|e| e.item_stack_infos.get(&slot))
            .map(|i| i.stack_id)
            .unwrap_or(0)
    }

    // ── Listener / predictions / pending sync (PMMP: InventoryListener +
    //    onSlotChange + addPredictedSlotChange + flushPendingUpdates) ─────

    /// Mutation server-side d'un slot. Met à jour l'inventaire ET déclenche
    /// la logique `onSlotChange` PMMP : si une prédiction client matche on
    /// associe au requestId courant, sinon on queue un `pending_sync` qui sera
    /// envoyé lors du prochain `flush_pending_updates`.
    pub fn set_slot(
        &mut self,
        inv: &mut PlayerInventory,
        key: InvKey,
        core_slot: usize,
        new_item: ItemStack,
    ) {
        let new_air = new_item.is_air();
        if let Some(slot) = inv.slot_mut(key, core_slot) {
            slot.item = new_item.clone();
            // Le stack_id réel sera réécrit par on_slot_change/track_item_stack.
        }
        self.on_slot_change(inv, key, core_slot, new_item, new_air);
    }

    /// Prédit un changement client. Quand `set_slot` arrive ensuite avec le
    /// même item, on associe au requestId courant (pas de pending_sync). Sinon
    /// on resync. PMMP `addPredictedSlotChange`.
    pub fn add_predicted_slot_change(
        &mut self,
        key: InvKey,
        core_slot: usize,
        item: ItemStack,
    ) {
        self.inventories
            .entry(key)
            .or_default()
            .predictions
            .insert(core_slot, item);
    }

    fn on_slot_change(
        &mut self,
        _inv: &PlayerInventory,
        key: InvKey,
        core_slot: usize,
        current_item: ItemStack,
        is_air: bool,
    ) {
        let current_request = self.current_item_stack_request_id;
        let entry = self.inventories.entry(key).or_default();

        let predicted = entry.predictions.remove(&core_slot);
        let matches = predicted
            .as_ref()
            .map(|p| item_stack_equals(p, &current_item))
            .unwrap_or(false);

        let request_id = if matches { current_request } else { None };
        let stack_id = if is_air { 0 } else { self.next_item_stack_id };
        if !is_air {
            self.next_item_stack_id += 1;
        }
        let info = ItemStackInfo {
            request_id,
            stack_id,
        };
        let entry = self.inventories.get_mut(&key).unwrap();
        entry.item_stack_infos.insert(core_slot, info);
        if !matches {
            entry.pending_syncs.insert(core_slot, current_item);
        }
    }

    /// PMMP `flushPendingUpdates()` — à appeler à chaque fin de tick. Si un
    /// `request_sync_all` est en attente → full sync. Sinon → envoie chaque
    /// `pending_sync` slot par slot.
    pub fn flush_pending_updates(&mut self, inv: &PlayerInventory) -> PacketOut {
        let mut out = PacketOut::new();
        if self.full_sync_requested {
            self.full_sync_requested = false;
            self.sync_all(inv, &mut out);
            return out;
        }

        // Snapshot keys+slots à flush avant emprunt mutable.
        let mut to_flush: Vec<(InvKey, usize)> = Vec::new();
        for (key, entry) in &self.inventories {
            for slot in entry.pending_syncs.keys() {
                to_flush.push((*key, *slot));
            }
        }

        for (key, core_slot) in to_flush {
            // Re-écrit le stack_id sur le wrapper inv pour cohérence.
            let stack_id = self
                .inventories
                .get(&key)
                .and_then(|e| e.item_stack_infos.get(&core_slot))
                .map(|i| i.stack_id)
                .unwrap_or(0);
            let wrapper = inv
                .slot_ref(key, core_slot)
                .cloned()
                .map(|w| ItemStackWrapper {
                    stack_id,
                    item: w.item,
                })
                .unwrap_or_else(ItemStackWrapper::air);
            self.emit_slot_sync(key, core_slot, &wrapper, &mut out);
            // Mise à jour mémorisée du stack_id côté inv (read-only ici, on le
            // retouche dans `set_slot` quand l'appelant a un &mut).
            // On vide pending_syncs après émission.
            if let Some(entry) = self.inventories.get_mut(&key) {
                entry.pending_syncs.remove(&core_slot);
            }
        }
        out
    }

    pub fn request_sync_all(&mut self) {
        self.full_sync_requested = true;
    }

    /// PMMP `addRawPredictedSlotChanges` : pour chaque action legacy avec
    /// `source_type == SOURCE_CONTAINER` (0), enregistre la prédiction
    /// `new_item` côté manager, ce qui permet à `on_slot_change` ultérieur de
    /// matcher et éviter un re-sync inutile.
    pub fn add_raw_predicted_slot_changes(
        &mut self,
        actions: &[mc_rs_proto::packets::player::NetworkInventoryAction],
    ) {
        for action in actions {
            if action.source_type != 0 {
                continue;
            }
            let Some(window_id) = action.window_id else { continue };
            // Seules INVENTORY/OFFHAND/ARMOR sont autorisées en legacy (PMMP).
            let key = match window_id {
                0 => InvKey::Main,
                119 => InvKey::Offhand,
                120 => InvKey::Armor,
                _ => continue,
            };
            let core_slot = action.inventory_slot as usize;
            self.add_predicted_slot_change(key, core_slot, action.new_item.item.clone());
        }
    }

    /// PMMP `syncMismatchedPredictedSlotChanges` : à appeler après l'exécution
    /// d'une transaction legacy. Toute prédiction restante (donc non matchée
    /// par on_slot_change) est marquée pending_sync pour resync.
    pub fn sync_mismatched_predicted_slot_changes(&mut self, inv: &PlayerInventory) {
        let mut to_resync: Vec<(InvKey, usize)> = Vec::new();
        for (key, entry) in &self.inventories {
            for slot in entry.predictions.keys() {
                to_resync.push((*key, *slot));
            }
        }
        for (key, slot) in to_resync {
            if let Some(entry) = self.inventories.get_mut(&key) {
                entry.predictions.remove(&slot);
                if let Some(item) = inv.slot_ref(key, slot) {
                    entry.pending_syncs.insert(slot, item.item.clone());
                }
            }
        }
    }

    /// Helper haut-niveau type PMMP `Inventory::addItem`. Tente de stacker sur
    /// un slot existant compatible, sinon premier slot vide. Retourne true si
    /// l'item a été ajouté entièrement. Met à jour le tracking.
    pub fn add_item_to_main(&mut self, inv: &mut PlayerInventory, item: ItemStack) -> bool {
        if item.is_air() || item.count == 0 {
            return false;
        }
        let max_stack = 64u16;

        // 1) Stack sur slot existant compatible.
        for i in 0..36 {
            let slot = &inv.slots[i];
            if !slot.item.is_air()
                && slot.item.id == item.id
                && slot.item.meta == item.meta
                && slot.item.block_runtime_id == item.block_runtime_id
                && slot.item.count < max_stack
            {
                let space = max_stack - slot.item.count;
                let add = item.count.min(space);
                let mut new_item = slot.item.clone();
                new_item.count += add;
                self.set_slot(inv, InvKey::Main, i, new_item);
                if add >= item.count {
                    return true;
                }
                // Reste à placer : tronque puis cherche slot vide.
                let mut remaining = item.clone();
                remaining.count -= add;
                return self.add_item_to_main(inv, remaining);
            }
        }
        // 2) Premier slot vide.
        for i in 0..36 {
            if inv.slots[i].item.is_air() {
                self.set_slot(inv, InvKey::Main, i, item);
                return true;
            }
        }
        false
    }

    /// Localise un couple (windowId, netSlot) dans un (key, coreSlot) logique.
    /// PMMP `locateWindowAndSlot()`.
    pub fn locate_window_and_slot(&self, window_id: u8, net_slot: u32) -> Option<(InvKey, usize)> {
        if window_id == container_ids::UI {
            let key = *self.complex_slot_to_key.get(&net_slot)?;
            let (net_to_core, _) = self.inventories.get(&key)?.complex_slot_map.as_ref()?;
            let core = *net_to_core.get(&net_slot)?;
            Some((key, core))
        } else {
            let key = *self.network_id_to_key.get(&window_id)?;
            Some((key, net_slot as usize))
        }
    }
}

impl Default for InventoryManager {
    fn default() -> Self {
        Self::new()
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Item equality (PMMP `itemStacksEqual`) — pour matcher prédiction vs réalité.
// ────────────────────────────────────────────────────────────────────────────

fn item_stack_equals(a: &ItemStack, b: &ItemStack) -> bool {
    a.id == b.id
        && a.meta == b.meta
        && a.block_runtime_id == b.block_runtime_id
        && a.count == b.count
        && a.extra_data == b.extra_data
}

// ────────────────────────────────────────────────────────────────────────────
// ItemStackContainerIdTranslator + ItemStackRequestExecutor + ResponseBuilder
// ────────────────────────────────────────────────────────────────────────────

/// Port de `ItemStackContainerIdTranslator::translate()`.
/// (containerInterfaceId, currentWindowId, slotId) → (windowId, slotId).
fn translate_container_ui(
    interface_id: u8,
    _current_window_id: u8,
    slot_id: u8,
) -> Option<(u8, u32)> {
    Some(match interface_id {
        ui_id::ARMOR => (container_ids::ARMOR, slot_id as u32),
        ui_id::HOTBAR | ui_id::INVENTORY | ui_id::COMBINED_HOTBAR_AND_INVENTORY => {
            (container_ids::INVENTORY, slot_id as u32)
        }
        // PMMP HACK : le client envoie un mauvais slotId pour offhand → on force 0.
        ui_id::OFFHAND => (container_ids::OFFHAND, 0),
        ui_id::CURSOR | ui_id::CRAFTING_INPUT => (container_ids::UI, slot_id as u32),
        _ => return None,
    })
}

/// Résultat d'un `process_item_stack_request` : paquets à envoyer + items à
/// faire spawner dans le monde.
pub struct ProcessOutcome {
    pub packets: PacketOut,
    pub drops: Vec<ItemStack>,
}

impl InventoryManager {
    /// Exposé publiquement pour permettre aux callers (block-break, pickup) de
    /// regénérer un stackId frais et garder le tracking cohérent.
    pub fn new_item_stack_id_pub(&mut self) -> i32 {
        self.new_item_stack_id()
    }

    /// Port de `ItemStackRequestExecutor::generateInventoryTransaction()` +
    /// `ItemStackResponseBuilder::build()`. Applique chaque action sur
    /// `inv`, re-track les slots modifiés avec un stackId frais associé au
    /// `request.request_id`, génère les packets de sync (slot par slot), et
    /// retourne aussi la liste des items à drop physiquement dans le monde.
    pub fn process_item_stack_request(
        &mut self,
        inv: &mut PlayerInventory,
        request: &ItemStackRequest,
    ) -> ProcessOutcome {
        self.set_current_item_stack_request_id(Some(request.request_id));
        let mut outcome = ProcessOutcome {
            packets: Vec::new(),
            drops: Vec::new(),
        };

        // Slots touchés (déduplication par (interface_id, slot_id)) — sert à
        // construire la réponse client. PMMP `requestSlotInfos`.
        let mut touched: Vec<(u8, u8)> = Vec::new();
        let mut touched_logical: Vec<(InvKey, usize)> = Vec::new();
        let mut had_error = false;

        let record = |touched: &mut Vec<(u8, u8)>,
                          touched_logical: &mut Vec<(InvKey, usize)>,
                          info: &SlotInfo,
                          key: InvKey,
                          core: usize| {
            if !touched.iter().any(|(c, s)| *c == info.container_id && *s == info.slot_id) {
                touched.push((info.container_id, info.slot_id));
            }
            if !touched_logical.iter().any(|(k, s)| *k == key && *s == core) {
                touched_logical.push((key, core));
            }
        };

        for action in &request.actions {
            match action {
                StackRequestAction::Take {
                    count,
                    source,
                    destination,
                }
                | StackRequestAction::Place {
                    count,
                    source,
                    destination,
                } => {
                    let src = match self.resolve_slot_info(source) {
                        Some(v) => v,
                        None => {
                            had_error = true;
                            continue;
                        }
                    };
                    let dst = match self.resolve_slot_info(destination) {
                        Some(v) => v,
                        None => {
                            had_error = true;
                            continue;
                        }
                    };
                    record(&mut touched, &mut touched_logical, source, src.0, src.1);
                    record(&mut touched, &mut touched_logical, destination, dst.0, dst.1);

                    if !self.transfer_items(inv, src, dst, *count as u16) {
                        had_error = true;
                    }
                }
                StackRequestAction::Swap {
                    source,
                    destination,
                    ..
                } => {
                    let src = match self.resolve_slot_info(source) {
                        Some(v) => v,
                        None => {
                            had_error = true;
                            continue;
                        }
                    };
                    let dst = match self.resolve_slot_info(destination) {
                        Some(v) => v,
                        None => {
                            had_error = true;
                            continue;
                        }
                    };
                    record(&mut touched, &mut touched_logical, source, src.0, src.1);
                    record(&mut touched, &mut touched_logical, destination, dst.0, dst.1);
                    self.swap_items(inv, src, dst);
                }
                StackRequestAction::Drop { count, source } => {
                    let src = match self.resolve_slot_info(source) {
                        Some(v) => v,
                        None => {
                            had_error = true;
                            continue;
                        }
                    };
                    record(&mut touched, &mut touched_logical, source, src.0, src.1);
                    if let Some(removed) = self.remove_items(inv, src, *count as u16) {
                        outcome.drops.push(removed);
                    } else {
                        had_error = true;
                    }
                }
                StackRequestAction::Destroy { count, source } => {
                    let src = match self.resolve_slot_info(source) {
                        Some(v) => v,
                        None => {
                            had_error = true;
                            continue;
                        }
                    };
                    record(&mut touched, &mut touched_logical, source, src.0, src.1);
                    let _ = self.remove_items(inv, src, *count as u16);
                }
                StackRequestAction::MineBlock {
                    hotbar_slot,
                    predicted_durability: _,
                    network_stack_id: _,
                } => {
                    // PMMP : MineBlockStackRequestAction met à jour la
                    // durabilité de l'outil. Sans système Durable, on se
                    // contente d'ack le slot pour éviter un re-sync.
                    let info = SlotInfo {
                        container_id: ui_id::HOTBAR,
                        slot_id: *hotbar_slot as u8,
                        stack_id: 0,
                    };
                    if let Some(target) = self.resolve_slot_info(&info) {
                        record(&mut touched, &mut touched_logical, &info, target.0, target.1);
                    }
                }
                StackRequestAction::CraftCreative {
                    creative_item_network_id: _,
                } => {
                    // En créatif, la prochaine action Take/Place portera sur
                    // le « créé » : sans CreativeInventoryCache on ne peut pas
                    // matérialiser l'item. On marque l'erreur mais sans full
                    // resync (les Take/Place suivants seront ignorés naturellement).
                    had_error = true;
                }
                StackRequestAction::Unknown(code) => {
                    // PMMP throw `ItemStackRequestProcessException`. Côté Rust,
                    // on garde la session vivante mais on demande un full
                    // resync au prochain flush, sinon on accumule de la
                    // désynchro qui finit par crasher le client.
                    tracing::debug!(
                        "ItemStackRequest: action type {} non gérée — request_sync_all queued",
                        code
                    );
                    self.full_sync_requested = true;
                    had_error = true;
                }
            }
        }

        // Re-track les slots modifiés (assigne de nouveaux stackIds liés à ce
        // requestId) avant de construire la réponse, comme PMMP fait via le
        // listener `onSlotChange` qui passe par `trackItemStack`.
        for (key, core) in &touched_logical {
            let item_air = inv
                .slot_ref(*key, *core)
                .map(|w| w.item.is_air())
                .unwrap_or(true);
            let info = self.track_item_stack(*key, *core, item_air, Some(request.request_id));
            // Sync wire-side : envoie le slot avec son nouveau stackId.
            if let Some(wrapper) = inv.slot_ref(*key, *core).cloned() {
                let new_wrapper = ItemStackWrapper {
                    stack_id: info.stack_id,
                    item: wrapper.item,
                };
                // Met à jour l'inventaire pour refléter le stackId tracké.
                if let Some(slot) = inv.slot_mut(*key, *core) {
                    slot.stack_id = info.stack_id;
                }
                self.emit_slot_sync(*key, *core, &new_wrapper, &mut outcome.packets);
            }
        }

        // Construction de la réponse.
        let response = if had_error {
            ItemStackResponse::error(request.request_id)
        } else {
            self.build_response(inv, request.request_id, &touched)
        };
        outcome.packets.push((
            packet_id::ITEM_STACK_RESPONSE,
            response.encode(),
        ));

        self.set_current_item_stack_request_id(None);
        outcome
    }

    fn resolve_slot_info(&self, info: &SlotInfo) -> Option<(InvKey, usize)> {
        let (window_id, net_slot) = translate_container_ui(
            info.container_id,
            self.last_inventory_network_id,
            info.slot_id,
        )?;
        self.locate_window_and_slot(window_id, net_slot)
    }

    fn emit_slot_sync(
        &self,
        key: InvKey,
        core_slot: usize,
        wrapper: &ItemStackWrapper,
        out: &mut PacketOut,
    ) {
        // Détermine windowId + netSlot pour le sync.
        let entry = match self.inventories.get(&key) {
            Some(e) => e,
            None => return,
        };
        if let Some((_, core_to_net)) = &entry.complex_slot_map {
            if let Some(&net_slot) = core_to_net.get(&core_slot) {
                self.send_inventory_slot_packets(
                    container_ids::UI as u32,
                    net_slot,
                    wrapper,
                    out,
                );
            }
        } else if let Some(window_id) = self.window_id_for(key) {
            if key == InvKey::Offhand {
                // PMMP HACK : pour l'offhand on envoie InventoryContent au lieu de slot.
                self.send_inventory_content_packets(
                    window_id as u32,
                    std::slice::from_ref(wrapper),
                    out,
                );
            } else {
                self.send_inventory_slot_packets(
                    window_id as u32,
                    core_slot as u32,
                    wrapper,
                    out,
                );
            }
        }
    }

    fn build_response(
        &self,
        inv: &PlayerInventory,
        request_id: i32,
        touched: &[(u8, u8)],
    ) -> ItemStackResponse {
        let mut by_container: Vec<ItemStackResponseContainer> = Vec::new();
        for (interface_id, slot_id) in touched {
            if *interface_id == ui_id::CREATED_OUTPUT {
                continue;
            }
            let Some((window_id, net_slot)) = translate_container_ui(
                *interface_id,
                self.last_inventory_network_id,
                *slot_id,
            ) else {
                continue;
            };
            let Some((key, core_slot)) = self.locate_window_and_slot(window_id, net_slot) else {
                continue;
            };
            let Some(wrapper) = inv.slot_ref(key, core_slot) else {
                continue;
            };
            let stack_id = self.stack_id_of(key, core_slot);
            let resp_slot = ItemStackResponseSlot {
                slot: *slot_id,
                hotbar_slot: *slot_id,
                count: wrapper.item.count as u8,
                stack_id,
                custom_name: String::new(),
                filtered_custom_name: String::new(),
                durability_correction: 0,
            };
            if let Some(c) = by_container
                .iter_mut()
                .find(|c| c.container_id == *interface_id)
            {
                c.slots.push(resp_slot);
            } else {
                by_container.push(ItemStackResponseContainer {
                    container_id: *interface_id,
                    slots: vec![resp_slot],
                });
            }
        }
        ItemStackResponse::ok(request_id, by_container)
    }

    /// Décrémente `count` d'un slot logique. Retourne le stack retiré, ou None
    /// si insuffisant / vide.
    fn remove_items(
        &self,
        inv: &mut PlayerInventory,
        target: (InvKey, usize),
        count: u16,
    ) -> Option<ItemStack> {
        let (key, core) = target;
        let slot = inv.slot_mut(key, core)?;
        if slot.item.is_air() || slot.item.count < count || count == 0 {
            return None;
        }
        let mut removed = slot.item.clone();
        removed.count = count;
        if slot.item.count == count {
            *slot = ItemStackWrapper::air();
        } else {
            slot.item.count -= count;
        }
        Some(removed)
    }

    /// Ajoute `count` items dans `target`. Le slot doit être vide ou contenir
    /// le même item (mêmes id+meta+block_runtime_id). Retourne true si ok.
    fn add_items(
        &self,
        inv: &mut PlayerInventory,
        target: (InvKey, usize),
        item: &ItemStack,
        count: u16,
    ) -> bool {
        let (key, core) = target;
        let Some(slot) = inv.slot_mut(key, core) else {
            return false;
        };
        if slot.item.is_air() {
            let mut new_item = item.clone();
            new_item.count = count;
            slot.item = new_item;
            return true;
        }
        if slot.item.id == item.id
            && slot.item.meta == item.meta
            && slot.item.block_runtime_id == item.block_runtime_id
        {
            slot.item.count = slot.item.count.saturating_add(count);
            return true;
        }
        false
    }

    fn transfer_items(
        &self,
        inv: &mut PlayerInventory,
        src: (InvKey, usize),
        dst: (InvKey, usize),
        count: u16,
    ) -> bool {
        let Some(removed) = self.remove_items(inv, src, count) else {
            return false;
        };
        if !self.add_items(inv, dst, &removed, count) {
            // rollback
            let _ = self.add_items(inv, src, &removed, count);
            return false;
        }
        true
    }

    fn swap_items(&self, inv: &mut PlayerInventory, a: (InvKey, usize), b: (InvKey, usize)) {
        let item_a = inv
            .slot_ref(a.0, a.1)
            .cloned()
            .unwrap_or_else(ItemStackWrapper::air);
        let item_b = inv
            .slot_ref(b.0, b.1)
            .cloned()
            .unwrap_or_else(ItemStackWrapper::air);
        if let Some(s) = inv.slot_mut(a.0, a.1) {
            *s = item_b;
        }
        if let Some(s) = inv.slot_mut(b.0, b.1) {
            *s = item_a;
        }
    }
}
