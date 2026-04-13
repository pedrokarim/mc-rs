# 09 — Système d'inventaire

> **STATUT : NON FONCTIONNEL.** Voir `INVENTORY-ITEMS-SYSTEM.md` pour le plan complet de portage.
>
> Ce document décrit le système cible (PMMP). L'implémentation actuelle dans `crates/mc-rs-server/src/inventory.rs` et `crates/mc-rs-server/src/connection/inventory.rs` est un squelette incomplet — elle DOIT être remplacée par un port intégral de l'`InventoryManager` PMMP.

---

## 1. Architecture cible (port intégral PMMP)

### 1.1 Hiérarchie des inventaires

```
Inventory (trait)
└── BaseInventory (struct générique)
    ├── PlayerInventory       — 36 slots (hotbar 0-8 + main 9-35)
    ├── PlayerOffHandInventory — 1 slot
    ├── ArmorInventory        — 4 slots
    ├── PlayerCursorInventory — 1 slot (item au curseur)
    ├── PlayerCraftingInventory — 4 slots (grille 2x2)
    ├── PlayerEnderInventory  — 27 slots
    ├── PlayerCreativeInventory — items créatifs filtrables
    ├── ChestInventory        — 27 slots
    ├── DoubleChestInventory  — 54 slots
    ├── FurnaceInventory      — 3 slots (input, fuel, output)
    ├── HopperInventory       — 5 slots
    ├── DropperInventory      — 9 slots
    ├── BrewingStandInventory — 5 slots
    ├── EnchantingTableInventory — 2 slots
    ├── AnvilInventory        — 3 slots
    ├── ShulkerBoxInventory   — 27 slots
    ├── BarrelInventory       — 27 slots
    ├── CampfireInventory     — 4 slots
    ├── BeaconInventory       — 1 slot
    ├── BlastFurnaceInventory — 3 slots
    ├── SmokerInventory       — 3 slots
    ├── StonecutterInventory  — 1 slot
    ├── LoomInventory         — 4 slots
    ├── GrindstoneInventory   — 3 slots
    ├── CartographyInventory  — 3 slots
    ├── SmithingTableInventory — 3 slots
    └── CrafterInventory      — 9 slots
```

### 1.2 Trait Inventory

Calque PMMP `Inventory.php` :

```rust
pub trait Inventory: Send + Sync {
    fn size(&self) -> usize;
    fn max_stack_size(&self) -> u8 { 64 }

    fn get_item(&self, slot: usize) -> &ItemStack;
    fn set_item(&mut self, slot: usize, item: ItemStack);
    fn get_contents(&self, include_empty: bool) -> Vec<ItemStack>;
    fn set_contents(&mut self, items: Vec<ItemStack>);

    fn add_item(&mut self, item: ItemStack) -> Vec<ItemStack>;     // retour : reste
    fn remove_item(&mut self, item: &ItemStack) -> Vec<ItemStack>; // retour : reste
    fn can_add_item(&self, item: &ItemStack) -> bool;
    fn contains(&self, item: &ItemStack) -> bool;
    fn first(&self, item: &ItemStack, exact_count: bool) -> Option<usize>;
    fn first_empty(&self) -> Option<usize>;
    fn count(&self, item: &ItemStack) -> u32;
    fn clear(&mut self, slot: usize);
    fn clear_all(&mut self);

    fn get_addable_item_quantity(&self, item: &ItemStack) -> u32;

    fn get_listeners(&mut self) -> &mut Vec<Box<dyn InventoryListener>>;
    fn slot_exists(&self, slot: usize) -> bool;
}
```

### 1.3 InventoryListener

```rust
pub trait InventoryListener: Send + Sync {
    fn on_slot_change(&mut self, inv: &dyn Inventory, slot: usize, old_item: &ItemStack);
    fn on_content_change(&mut self, inv: &dyn Inventory, old_contents: &[ItemStack]);
}
```

C'est par ce trait que l'`InventoryManager` (réseau) écoute les changements pour envoyer les mises à jour au client. Sans listener, **aucune mise à jour de slot ne sera envoyée au client après modification serveur.**

### 1.4 InventoryManager (per-Connection)

Voir `INVENTORY-ITEMS-SYSTEM.md` §3.1 et suivants pour le détail complet. Résumé :

```rust
pub struct InventoryManager {
    // Mapping inventaires <-> windowId
    inventories: HashMap<InventoryRef, InventoryManagerEntry>,
    network_id_to_inventory: HashMap<u8, InventoryRef>,
    complex_slot_to_inventory: HashMap<u32, ComplexWindowMapEntry>,

    // État de la session
    last_inventory_network_id: u8,    // commence à FIRST=1
    current_window_type: i32,         // -1 = INVENTORY au défaut

    // Stack ID tracking
    next_item_stack_id: i32,
    current_item_stack_request_id: Option<i32>,

    // Handshake d'ouverture/fermeture
    pending_close_window_id: Option<u8>,
    pending_open_window_callback: Option<Box<dyn FnOnce()>>,

    // Sync state
    full_sync_requested: bool,

    // Hotbar
    client_selected_hotbar_slot: i32, // -1 si pas encore reçu

    // Special inventories (UI)
    enchanting_table_options: Vec<(i32, i32)>,
    next_enchanting_table_option_id: i32,
}

pub struct InventoryManagerEntry {
    pub inventory: InventoryRef,
    pub item_stack_infos: HashMap<usize, ItemStackInfo>,
    pub predictions: HashMap<usize, ItemStack>,    // prédictions client
    pub pending_syncs: HashMap<usize, ItemStack>,  // sync différée
    pub complex_slot_map: Option<ComplexWindowMap>,
}

pub struct ItemStackInfo {
    pub request_id: i32,
    pub stack_id: i32,
}

pub struct ComplexWindowMap {
    pub slot_map: HashMap<u32, usize>,  // netSlot → coreSlot
    pub inventory: InventoryRef,
}
```

---

## 2. Constantes obligatoires

### ContainerIds (vendor/.../ContainerIds.php)
```rust
pub mod container_ids {
    pub const NONE: i8 = -1;
    pub const INVENTORY: u8 = 0;
    pub const FIRST: u8 = 1;
    pub const LAST: u8 = 100;
    pub const OFFHAND: u8 = 119;
    pub const ARMOR: u8 = 120;
    pub const HOTBAR: u8 = 122;
    pub const FIXED_INVENTORY: u8 = 123;
    pub const UI: u8 = 124;
    pub const CONTAINER_ID_REGISTRY: u8 = 125;
}
```

### WindowTypes (vendor/.../WindowTypes.php)
```rust
pub mod window_types {
    pub const NONE: i8 = -9;
    pub const INVENTORY: i8 = -1;
    pub const CONTAINER: i8 = 0;
    pub const WORKBENCH: i8 = 1;
    pub const FURNACE: i8 = 2;
    pub const ENCHANTMENT: i8 = 3;
    pub const BREWING_STAND: i8 = 4;
    pub const ANVIL: i8 = 5;
    pub const HOPPER: i8 = 8;
    pub const BEACON: i8 = 13;
    pub const TRADING: i8 = 15;
    pub const HUD: i8 = 31;
    pub const SMITHING_TABLE: i8 = 33;
    pub const CRAFTER: i8 = 36;
    // ... voir vendor/.../WindowTypes.php pour la liste complète
}
```

### UIInventorySlotOffset (src/network/mcpe/handler/UIInventorySlotOffset.php)
Indices des slots UI à mapper vers les inventaires logiques :
```rust
pub mod ui_slot {
    pub const CURSOR: usize = 0;
    pub const CRAFTING2X2_INPUT_START: usize = 28; // 28..32 (4 slots)
    pub const CRAFTING_RESULT: usize = 50;
    pub const ANVIL_INPUT: usize = 1;
    pub const ANVIL_MATERIAL: usize = 2;
    pub const ENCHANTING_INPUT: usize = 14;
    pub const ENCHANTING_LAPIS: usize = 15;
    pub const STONECUTTER_INPUT: usize = 3;
    pub const TRADE_INPUT_1: usize = 4;
    pub const TRADE_INPUT_2: usize = 5;
    pub const LOOM_INPUT: usize = 9;
    pub const LOOM_DYE: usize = 10;
    pub const LOOM_PATTERN: usize = 11;
    pub const CARTOGRAPHY_INPUT: usize = 12;
    pub const CARTOGRAPHY_ADDITIONAL: usize = 13;
    pub const GRINDSTONE_INPUT: usize = 16;
    pub const GRINDSTONE_ADDITIONAL: usize = 17;
    pub const COMPOUND_CREATOR_INPUT_START: usize = 27;
    pub const SMITHING_INPUT: usize = 51;
    pub const SMITHING_MATERIAL: usize = 52;
    pub const SMITHING_TEMPLATE: usize = 53;
}
```

---

## 3. Format wire (rappel — voir INVENTORY-ITEMS-SYSTEM.md §3.7 pour le détail)

| Packet | ID | Contenu (résumé) |
|---|---|---|
| ContainerOpen | 0x2E | `Byte windowId, Byte windowType, BlockPos blockPos, VarI64 actorId` |
| ContainerClose | 0x2F | `Byte windowId, Byte windowType, Bool server` |
| InventoryContent | 0x31 | `VarU32 windowId, VarU32 count, ItemStackWrapper[N], FullContainerName name, ItemStackWrapper storage` |
| InventorySlot | 0x32 | `VarU32 windowId, VarU32 slot, FullContainerName name, ItemStackWrapper storage, ItemStackWrapper item` |
| MobEquipment | 0x1F | `VarU64 entityId, ItemStackWrapper item, Byte invSlot, Byte hotbarSlot, Byte windowId` |
| ItemStackRequest | 0x93 (C→S) | Liste de requests, chacune avec actions (Take/Place/Swap/Drop/Destroy/Craft/...) |
| ItemStackResponse | 0x94 | Liste de responses (status, requestId, slot changes par container) |
| AddItemActor | 0x0F | `VarI64 uid, VarU64 rid, ItemStackWrapper item, Vec3 pos, Vec3 motion, EntityMetadata meta, Bool fromFishing` |
| TakeItemActor | 0x11 | `VarU64 itemRid, VarU64 takerRid` |

---

## 4. Two-phase sync (RAPPEL CRITIQUE)

```
Pour TOUT envoi InventoryContentPacket (sauf cas trivial slot 0=air) :

PHASE 1 (clear) :
  InventoryContentPacket {
    windowId = X,
    items = [air × N],
    containerName = FullContainerName(lastInventoryNetworkId),
    storage = air,
  }

PHASE 2 (real) :
  InventoryContentPacket {
    windowId = X,
    items = [wrappers avec stackIds],
    containerName = FullContainerName(lastInventoryNetworkId),
    storage = air,
  }
```

Sans la phase 1 → **crash client à l'ouverture inventaire**.

---

## 5. Files PMMP de référence

Voir `INVENTORY-ITEMS-SYSTEM.md` §6 pour la liste exhaustive avec chemins absolus.

---

## 6. Plan d'implémentation

Voir `INVENTORY-ITEMS-SYSTEM.md` §4 pour le plan détaillé en 6 phases (A → F).

---

## 7. Tests E2E à passer

Voir `INVENTORY-ITEMS-SYSTEM.md` §4 Phase F pour la liste des 10 tests E2E à valider avant de considérer le système terminé.
