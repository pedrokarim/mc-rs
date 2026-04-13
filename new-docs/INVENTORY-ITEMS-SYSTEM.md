# Système Inventaire / Items / Items au sol — État réel et plan de portage complet

> **Document de référence ABSOLUE pour la prochaine session de développement.**
> Tous les chemins de fichiers, lignes, formats wire et constantes sont vérifiés.
> Référence unique : **PocketMine-MP** (`.reference/PocketMine-MP/`). Pas d'autres références.

---

## 1. Constat brutal

Ce système ne fonctionne pas et n'a JAMAIS fonctionné de bout en bout depuis le début du projet.

**Trois bugs distincts confirmés en testant avec un vrai client Bedrock 1.26.10 (protocol 944) :**

1. **Ouverture inventaire (touche E) → crash client instant**
   - Format PMMP `entityInv(windowId, -1, player.id)` → crash
   - Format dragonfly testé → crash aussi
   - Test isolé : envoyer un `TextPacket` à la place ne crashe pas
   - **Conclusion** : le ContainerOpen lui-même n'est pas le seul problème ; il manque tout l'écosystème inventaire derrière (UI inventory tracking, stack IDs uniques, ItemStackRequest predictions, etc.)

2. **Block break → drop d'item → ombre invisible puis disparition, parfois crash**
   - L'AddItemActor est envoyé avec un format vérifié byte-par-byte contre PMMP
   - Le client reçoit l'item mais le rend "transparent" (silhouette uniquement)
   - Après plusieurs items spawnés, le client crash sans raison apparente côté serveur
   - **Conclusion** : metadata/format incomplet ; il manque probablement des propriétés de rendu et la gestion correcte du `coreItemStackToNet`

3. **Inventaire pas affiché correctement même quand il ne crashe pas**
   - Les InventoryContent envoyés au spawn arrivent
   - Mais le client n'affiche pas les items dans la hotbar comme attendu
   - Le held_item n'est pas montré non plus

**Diagnostic global** : on a ~30 % du système PMMP. On bricole les 30 % en envoyant des paquets à la main, sans l'infrastructure derrière. **Il faut porter l'`InventoryManager` PMMP entièrement.**

---

## 2. Code mc-rs actuel — inventaire de l'existant

### 2.1 Fichiers concernés

| Fichier | Rôle | État |
|---|---|---|
| `crates/mc-rs-server/src/inventory.rs` | `PlayerInventory` struct, helpers `block_drop` / `item_to_block` | ⚠ Squelette, pas d'event listener, pas de stack ID tracking |
| `crates/mc-rs-server/src/connection/inventory.rs` | Handlers : `handle_interact`, `handle_container_close`, `handle_mob_equipment`, `handle_inventory_transaction`, `handle_item_stack_request`, `push_inventory_sync` | ⚠ Logique inline, pas de séparation manager/handler |
| `crates/mc-rs-server/src/connection/movement.rs` | Block break (action_type=26) → spawn item entity | ✅ Spawn fonctionnel, drop déclenché |
| `crates/mc-rs-server/src/item_entities.rs` | `ItemEntityManager`, physique, scatter/throw | ⚠ Items rendus en ombre côté client |
| `crates/mc-rs-server/src/entity.rs` | `item_metadata()` pour AddItemActor | ⚠ Probablement incomplet |
| `crates/mc-rs-server/src/item_registry.rs` | Registre items (string ↔ network_id) | ✅ Marche, `required_item_list.json` à jour pour 1.26.10 |
| `crates/mc-rs-proto/src/packets/player.rs` | `ItemStack`, `ItemStackWrapper`, `FullContainerName`, `InventoryContent`, `InventorySlot`, `MobEquipment` | ⚠ Encodage byte-correct mais legacy stack_id=1 hardcodé |
| `crates/mc-rs-proto/src/packets/world.rs` | `ContainerOpen`, `ContainerClose` | ⚠ Format byte-correct mais flow d'ouverture incomplet |

### 2.2 Champs `Connection` liés à l'inventaire

```rust
// crates/mc-rs-server/src/connection/mod.rs
pub struct Connection {
    // ...
    pub inventory: PlayerInventory,
    pub player_inventory_window_id: u8,    // démarre à 1, cycle 1-99
    pub player_inventory_open: bool,
    pub pending_item_spawns: Vec<PendingItemEntitySpawn>,
    // ...
}
```

### 2.3 PlayerInventory

```rust
// crates/mc-rs-server/src/inventory.rs:140-241
pub struct PlayerInventory {
    pub slots: Vec<ItemStackWrapper>,    // 36 slots (hotbar 0-8, main 9-35)
    pub armor: Vec<ItemStackWrapper>,    // 4 slots
    pub offhand: ItemStackWrapper,       // 1 slot
    pub held_slot: u8,                   // 0-8
    next_stack_id: i32,                  // incrémente, jamais reset
}
```

**Manque :** cursor inventory (1 slot), crafting grid 2x2 (4 slots), crafting output (1 slot), enderchest (27 slots), event listeners, dirty tracking.

### 2.4 Flow actuel d'ouverture E key

```rust
// crates/mc-rs-server/src/connection/inventory.rs:handle_interact (action=6)
1. Si player_inventory_open → return (anti double-send)
2. Marque player_inventory_open = true
3. window_id = advance_player_inventory_window_id()    // 1→2→...→99→1
4. ContainerOpen::entity_inventory(window_id, entity_runtime_id as i64)
5. Encode + envoi au client
// Pas de re-sync inventory après → bug
```

### 2.5 Flow actuel de sync au spawn

```rust
// crates/mc-rs-server/src/connection/inventory.rs:push_inventory_sync (lignes 216-258)
// Envoie 5 paquets compressés :
1. InventoryContent(window_id=0,   slots=36, FullContainerName(0))   // main
2. InventoryContent(window_id=124, slots=54, FullContainerName(0))   // UI (cursor + craft grid)
3. InventoryContent(window_id=119, slots=1,  FullContainerName(0))   // offhand
4. InventoryContent(window_id=120, slots=4,  FullContainerName(0))   // armor
5. MobEquipment(entity_runtime_id, held_item, held_slot)
```

**Manque :** two-phase sync (clear puis real), stack IDs uniques par item, ItemStackInfo tracking côté serveur.

### 2.6 ItemStackWrapper encoding actuel

```rust
// crates/mc-rs-proto/src/packets/player.rs:1705-1725
1. write_var_i32(item.id)        // 0 = air → return immédiat
2. write_u16_le(count)
3. write_var_u32(meta)
4. has_net_id = !is_air            // legacy mode
5. write_bool(has_net_id)
6. if has_net_id: write_var_i32(1)  // ← LEGACY STACK ID hardcodé à 1 !
7. write_var_i32(block_runtime_id)
8. write_byte_array(extra_data)    // VarU32 length + bytes
```

**Le hardcode `stack_id=1` est le problème central.** PMMP assigne un stackId unique incrémental par instance d'item via `nextItemStackId`. Le client utilise ce stackId pour les ItemStackRequest. Sans IDs uniques, toute interaction inventaire est cassée.

---

## 3. PocketMine-MP — Système de référence (à porter intégralement)

### 3.1 Architecture cible

```
┌─────────────────────────────────────────────────────────────┐
│ InventoryManager (par session/connexion)                    │
│   - HashMap<InventoryRef, InventoryManagerEntry>            │
│   - HashMap<windowId, InventoryRef>                         │
│   - HashMap<netSlotId, ComplexWindowMapEntry>  (UI slots)   │
│   - lastInventoryNetworkId: u32  (dynamique 1-99)           │
│   - currentWindowType: i32       (-1 = INVENTORY)           │
│   - nextItemStackId: i32         (assigne unique par item)  │
│   - currentItemStackRequestId: Option<i32>                  │
│   - pendingCloseWindowId: Option<u8>  (handshake close ack) │
│   - pendingOpenWindowCallback: Option<Closure>              │
│   - fullSyncRequested: bool                                 │
│   - clientSelectedHotbarSlot: i32 (-1 = none)               │
└─────────────────────────────────────────────────────────────┘
            │
            ▼
┌─────────────────────────────────────────────────────────────┐
│ Inventories permanents (joueur)                             │
│   - main      → ContainerIds::INVENTORY (0)   36 slots      │
│   - offHand   → ContainerIds::OFFHAND   (119) 1 slot        │
│   - armor     → ContainerIds::ARMOR     (120) 4 slots       │
│   - cursor    → ContainerIds::UI        (124) slot 0        │
│   - craftingGrid 2x2 → ContainerIds::UI (124) slots 28-31   │
│   - craftingResult   → ContainerIds::UI (124) slot 50       │
└─────────────────────────────────────────────────────────────┘
            │
            ▼
┌─────────────────────────────────────────────────────────────┐
│ Inventories ouverts dynamiquement (chests, etc.)            │
│   - Chaque ouverture : getNewWindowId() + associate         │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Constantes obligatoires

#### ContainerIds (`vendor/pocketmine/bedrock-protocol/src/types/inventory/ContainerIds.php`)
```
NONE                  = -1
INVENTORY             = 0
FIRST                 = 1
LAST                  = 100
OFFHAND               = 119
ARMOR                 = 120
HOTBAR                = 122
FIXED_INVENTORY       = 123
UI                    = 124
CONTAINER_ID_REGISTRY = 125
```

#### WindowTypes (`vendor/pocketmine/bedrock-protocol/src/types/inventory/WindowTypes.php`)
```
INVENTORY     = -1   // Ecran principal joueur (E key)
CONTAINER     = 0    // Coffre, dispenser, dropper
WORKBENCH     = 1
FURNACE       = 2
ENCHANTMENT   = 3
BREWING_STAND = 4
ANVIL         = 5
HOPPER        = 8
BEACON        = 13
SMOKER        = 28
STONECUTTER   = 29
SMITHING_TABLE= 33
// ... (jusqu'à CRAFTER = 36)
```

#### UIInventorySlotOffset (`src/network/mcpe/handler/UIInventorySlotOffset.php`)
```
CURSOR              = 0
CRAFTING2X2_INPUT   = 28..32
CRAFTING_RESULT     = 50
ANVIL_INPUT         = 1, 2
ANVIL_RESULT        = 50
ENCHANTING_INPUT    = 14
ENCHANTING_LAPIS    = 15
LOOM_INPUT          = 9
LOOM_DYE            = 10
LOOM_PATTERN        = 11
LOOM_RESULT         = 50
STONE_CUTTER_INPUT  = 3
STONE_CUTTER_RESULT = 50
TRADE_INPUT_1       = 4
TRADE_INPUT_2       = 5
TRADE_RESULT        = 50
GRINDSTONE_INPUT    = 16
GRINDSTONE_ADDITIONAL = 17
GRINDSTONE_RESULT   = 50
SMITHING_TEMPLATE   = 53
SMITHING_INPUT      = 51
SMITHING_MATERIAL   = 52
SMITHING_RESULT     = 50
COMPOUND_CREATOR_INPUT = 27 + (rangée*3)+colonne
COMPOUND_CREATOR_OUTPUT = 50
ELEMENT_CONSTRUCTOR_OUTPUT = 50
MATERIAL_REDUCER_INPUT = 35
MATERIAL_REDUCER_OUTPUT = 35..43
LAB_TABLE_INPUT     = 32, 33, 34
CARTOGRAPHY_INPUT   = 12
CARTOGRAPHY_ADDITIONAL = 13
CARTOGRAPHY_RESULT  = 50
```

### 3.3 Two-phase sync (BUG CLIENT BEDROCK 1.20.12+)

`InventoryManager.php:sendInventoryContentPackets()` lignes 542-559 :

```php
// Phase 1 : tout en air (force le client à oublier les anciens stackIds)
$session->sendDataPacket(InventoryContentPacket::create(
    $windowId,
    array_fill_keys(array_keys($wrappers), new ItemStackWrapper(0, ItemStack::null())),
    new FullContainerName($this->lastInventoryNetworkId),
    new ItemStackWrapper(0, ItemStack::null())
));

// Phase 2 : vrai contenu avec les nouveaux stackIds
$session->sendDataPacket(InventoryContentPacket::create(
    $windowId,
    $wrappers,
    new FullContainerName($this->lastInventoryNetworkId),
    new ItemStackWrapper(0, ItemStack::null())
));
```

**Sans la phase 1, le client ignore les changements de stackId quand l'item de surface est identique → désynchro silencieuse → crash à l'ouverture inventaire.**

Pareil pour `sendInventorySlotPackets()` (lignes 511-537) sur les slots individuels avec stackId != 0.

### 3.4 Gestion des stackIds (CRITIQUE)

```php
// InventoryManager.php
private int $nextItemStackId = 1;

private function newItemStackId() : int {
    return $this->nextItemStackId++;
}

class ItemStackInfo {
    public function __construct(
        private int $requestId,
        private int $stackId,
    ) {}
}

class InventoryManagerEntry {
    public array $itemStackInfos = [];           // slot → ItemStackInfo
    public array $predictions = [];              // slot → Item (prédit côté client)
    public array $pendingSyncs = [];             // slot → Item (à resync)
    public ?ComplexWindowMapEntry $complexSlotMap = null;
}
```

**Sans ce tracking, ItemStackRequestPacket envoyé par le client est ingérable.**

### 3.5 Flow d'ouverture inventaire principal (E key)

`InventoryManager.php:onClientOpenMainInventory()` lignes 394-408 :

```php
public function onClientOpenMainInventory() : void {
    $this->onCurrentWindowRemove();    // ferme l'ancienne fenêtre

    $this->openWindowDeferred(function() : void {
        $windowId = $this->getNewWindowId();
        $this->associateIdWithInventory($windowId, $this->player->getInventory());
        $this->currentWindowType = WindowTypes::INVENTORY;

        $this->session->sendDataPacket(ContainerOpenPacket::entityInv(
            $windowId,
            WindowTypes::INVENTORY,                  // -1
            $this->player->getId()
        ));
    });
    // Note : pas de syncContents() ici. La sync est faite ailleurs via le listener.
    // Mais en pratique le client REQUIERT un sync après l'ouverture.
}
```

`onCurrentWindowRemove()` lignes 410-420 :

```php
public function onCurrentWindowRemove() : void {
    if (isset($this->networkIdToInventoryMap[$this->lastInventoryNetworkId])) {
        $this->remove($this->lastInventoryNetworkId);
        $this->session->sendDataPacket(ContainerClosePacket::create(
            $this->lastInventoryNetworkId,
            $this->currentWindowType,
            true                                     // serverInitiated
        ));
        if ($this->pendingCloseWindowId !== null) {
            throw new AssumptionFailedError("Window close already pending");
        }
        $this->pendingCloseWindowId = $this->lastInventoryNetworkId;
        $this->enchantingTableOptions = [];
    }
}
```

`openWindowDeferred()` lignes 302-309 :

```php
private function openWindowDeferred(\Closure $func) : void {
    if ($this->pendingCloseWindowId !== null) {
        // Attendre l'ACK du close client
        $this->pendingOpenWindowCallback = $func;
    } else {
        $func();
    }
}

// Quand le client ACK le close (handle_container_close) :
public function onClientRemoveWindow(int $id) : void {
    if ($id === $this->pendingCloseWindowId) {
        $this->pendingCloseWindowId = null;
        if ($this->pendingOpenWindowCallback !== null) {
            ($this->pendingOpenWindowCallback)();
            $this->pendingOpenWindowCallback = null;
        }
    }
}
```

### 3.6 Sync au spawn — `syncAll()`

`InventoryManager.php:syncAll()` :

```php
public function syncAll() : void {
    foreach ($this->inventories as $entry) {
        $this->syncContents($entry->inventory);
    }
}
```

Itère TOUS les inventaires enregistrés au `__construct` :
1. Main → window_id=0
2. Offhand → window_id=119
3. Armor → window_id=120
4. UI cursor → window_id=124, slot 0
5. UI crafting grid → window_id=124, slots 28-31

Chaque `syncContents()` appelle `sendInventoryContentPackets()` avec les wrappers + le `lastInventoryNetworkId` actuel dans le FullContainerName.

### 3.7 Format des packets (vérifié 924, identique 944 sauf SetActorMotion)

#### ContainerOpenPacket (0x2E)
```
Byte    windowId             // dynamique 1-99 pour main inv
Byte    windowType           // 0xFF (-1) pour INVENTORY
BlockPosition blockPosition  // VarI32 x, VarU32 y (Binary::unsignInt), VarI32 z
                             // [0,0,0] pour entityInv
VarI64  actorUniqueId        // player entity ID pour entityInv
```

#### ContainerClosePacket (0x2F)
```
Byte    windowId
Byte    windowType
Bool    server               // true si initié par serveur
```

#### InventoryContentPacket (0x31)
```
VarU32  windowId             // 0, 119, 120, 124, ou ID dynamique
VarU32  itemCount
{ItemStackWrapper}[itemCount]
FullContainerName containerName
ItemStackWrapper  storage    // toujours air
```

#### InventorySlotPacket (0x32)
```
VarU32  windowId
VarU32  slot
FullContainerName containerName
ItemStackWrapper  storage    // toujours air
ItemStackWrapper  item       // le vrai item
```

#### MobEquipmentPacket (0x1F)
```
VarU64  actorRuntimeId
ItemStackWrapper item
Byte    inventorySlot        // 0-35
Byte    hotbarSlot           // 0-8
Byte    windowId             // 0=inv, 119=offhand
```

#### FullContainerName
```
Byte    containerId          // = lastInventoryNetworkId (PAS le windowId !)
Bool    hasDynamicId
[U32LE  dynamicId]           // si hasDynamicId
```

#### ItemStackWrapper (full)
```
VarI32  itemId               // 0 = air → STOP
U16LE   count
VarU32  meta
Bool    hasNetId             // true si stackId != 0
[VarI32 stackId]             // si hasNetId, stackId UNIQUE par instance
VarI32  blockRuntimeId
String  rawExtraData         // VarU32 length + bytes
```

#### ItemStackExtraData (rawExtraData content, item non-shield)
```
I16LE   nbtLength            // 0 = pas de NBT, 0xFFFF = NBT compound suit
[U8     nbtVersion = 1]      // si nbtLength == -1
[NBT    compound]            // si nbtLength == -1
U32LE   canPlaceOnCount
{ U16LE strLen + bytes }[canPlaceOnCount]
U32LE   canDestroyCount
{ U16LE strLen + bytes }[canDestroyCount]
[I64LE  shieldBlockingTick]  // si item == minecraft:shield (network_id=387 en 944)
```

### 3.8 ItemStackRequest — flow client→serveur

Le client envoie `ItemStackRequestPacket` (0x93) pour TOUTE interaction inventaire (drag, drop, swap, take, place). Format dans `vendor/.../ItemStackRequestPacket.php`. PMMP route vers `ItemStackRequestExecutor` qui :

1. Génère une `InventoryTransaction` à partir des actions
2. Valide (conservation des items)
3. Applique la transaction
4. Construit un `ItemStackResponseBuilder`
5. Le serveur envoie `ItemStackResponsePacket` (0x94) avec le résultat

Actions supportées :
- `Take` (count, source, destination)
- `Place` (count, source, destination)
- `Swap` (source, destination)
- `Drop` (count, source, randomly)
- `Destroy` (count, source)
- `CraftRecipe` (recipeNetworkId)
- `CraftRecipeAuto` (recipeNetworkId, count, ingredients)
- `CraftCreative` (creativeNetworkId)
- `CraftRecipeOptional` (recipeNetworkId, recipeIndex)
- `CraftGrindstone` (recipeNetworkId, repetitions)
- `CraftLoom` (patternId)
- `BeaconPayment` (primaryEffectId, secondaryEffectId)
- `MineBlock` (predictedDurability, networkStackId)
- `ConsumeAction` (count, source)
- `CreateAction` (count, source)
- ... (et d'autres)

`ItemStackResponseBuilder` track les changements par slot et les renvoie au client pour qu'il valide visuellement. **Sans cette réponse, le client annule les changements et resync.**

### 3.9 Block Break → Drop

`World::useBreakOn()` ligne 2136 :

```php
$drops = $block->getDrops($item);             // dépend de l'outil
$dropPos = $vector->add(0.5, 0.5, 0.5);       // CENTRE du bloc

// Pour CHAQUE drop
$this->dropItem($dropPos, $drop);
```

`World::dropItem($source, $item, $motion=null, $delay=10)` :

```php
$itemEntity = new ItemEntity(
    Location::fromObject($source, $world, rand(0,360), 0),  // yaw aléatoire
    $item
);
$itemEntity->setPickupDelay(10);  // 10 ticks (0.5s) à 20 TPS
$itemEntity->setMotion($motion ?? new Vector3(
    rand() * 0.2 - 0.1,
    0.2,                                       // saute toujours vers le haut
    rand() * 0.2 - 0.1
));
$itemEntity->spawnToAll();
```

`ItemEntity::sendSpawnPacket()` ligne 293 :

```php
AddItemActorPacket::create(
    $this->getId(),
    $this->getId(),
    ItemStackWrapper::legacy(coreItemStackToNet($this->getItem())),
    $this->location->asVector3(),
    $this->getMotion(),
    $this->getAllNetworkData(),                // metadata complète
    false                                       // isFromFishing
)
```

`getAllNetworkData()` retourne TOUTE la metadata d'entité, dont `Entity::syncNetworkData()` :
```
ALWAYS_SHOW_NAMETAG (81)  → byte 0
BOUNDING_BOX_HEIGHT (54)   → 0.25 / scale = 0.25
BOUNDING_BOX_WIDTH (53)    → 0.25
SCALE (38)                 → 1.0
LEAD_HOLDER_EID (37)       → -1
OWNER_EID (5)              → -1
TARGET_EID (6)             → 0
NAMETAG (4)                → ""
SCORE_TAG (84)             → ""
COLOR (3)                  → 0

// Flags (key 0, type long)
HAS_COLLISION (bit 48)         → true
AFFECTED_BY_GRAVITY (bit 49)   → true
NO_AI (bit 16)                 → false
INVISIBLE (bit 5)              → false
SILENT (bit 17)                → false
ONFIRE (bit 0)                 → false
WALLCLIMBING (bit 18)          → false
CAN_CLIMB (bit 19)             → false
CAN_SHOW_NAMETAG (bit 14)      → false
```

### 3.10 ItemEntity tick (physique)

`ItemEntity::entityBaseTick()` ligne 104 :
- Décrément pickupDelay
- Tente de merge avec items voisins (si ground + age multiple de 2)
- Décrément despawnDelay (default 6000 ticks = 5 min)

`Entity::tryChangeMovement()` (parent class) :
- Applique drag puis gravité
- Détecte collision ground
- Broadcast `MoveActorAbsolutePacket` + `SetActorMotionPacket` aux viewers

Constantes ItemEntity :
```
gravity         = 0.04 / tick²  (à 20 TPS)
drag            = 0.02
boundingBox     = 0.25 × 0.25
maxHealth       = 5
defaultPickupDelay   = 10 ticks
defaultDespawnDelay  = 6000 ticks
mergeCheckPeriod     = 2 ticks
yOffset (rendu) = 0.125
```

### 3.11 Pickup

`ItemEntity::onCollideWithPlayer($player)` ligne 318 :

```php
if ($this->getPickupDelay() !== 0) return;

// Choisir l'inventaire cible (offhand prioritaire si stackable)
$inventory = match(true) {
    $player->getOffHandInventory()->getItem(0)->canStackWith($item)
        && $player->getOffHandInventory()->getAddableItemQuantity($item) > 0
            => $player->getOffHandInventory(),
    $player->getInventory()->getAddableItemQuantity($item) > 0
            => $player->getInventory(),
    default => null,
};

$ev = new EntityItemPickupEvent($player, $this, $item, $inventory);
$ev->call();
if ($ev->isCancelled()) return;

NetworkBroadcastUtils::broadcastEntityEvent(
    $this->getViewers(),
    fn($broadcaster, $recipients) => $broadcaster->onPickUpItem($recipients, $player, $this)
);

if ($inventory !== null) {
    foreach ($inventory->addItem($item) as $remains) {
        $world->dropItem($this->location, $remains, new Vector3(0,0,0));
    }
}
$this->flagForDespawn();
```

`onPickUpItem()` envoie `TakeItemActorPacket(itemRuntimeId, takerRuntimeId)` aux viewers.

---

## 4. Plan de portage complet

### Phase A — Infrastructure (sans casser l'existant)

1. **Créer `crates/mc-rs-server/src/inventory_manager.rs`**
   - Struct `InventoryManager` per-Connection
   - HashMap<windowId, InventoryRef>
   - HashMap<slotIndex, ComplexWindowMapEntry>
   - Tracking : `last_inventory_network_id`, `current_window_type`, `next_item_stack_id`, `current_item_stack_request_id`, `pending_close_window_id`, `pending_open_callback`
   - Constants from PMMP : `INVENTORY=0`, `OFFHAND=119`, `ARMOR=120`, `UI=124`, `FIRST=1`, `LAST=100`

2. **Étendre `PlayerInventory`** dans `inventory.rs` :
   - Ajouter `cursor: ItemStack`, `crafting_grid_2x2: [ItemStack; 4]`, `crafting_result: ItemStack`
   - Ajouter event listener system (callback sur `set_slot`)
   - Méthodes `add_item`, `remove_item`, `can_add_item`, `first_empty`, `contains`, `count`

3. **Refactor `ItemStackWrapper`** dans `crates/mc-rs-proto/src/packets/player.rs` :
   - Retirer le hardcode `stack_id=1`
   - Accepter le stack_id en paramètre lors de la construction
   - `ItemStackWrapper::air()` reste avec stack_id=0

### Phase B — Port InventoryManager

1. **`InventoryManager::register_inventories()`** appelé à la création de la Connection :
   - Associer windowId 0 → main inventory
   - Associer windowId 119 → offhand
   - Associer windowId 120 → armor
   - Associer windowId 124 → UI (avec ComplexWindowMap pour cursor[0], craft[28-31], result[50])

2. **`InventoryManager::sync_all()`** :
   - Itère tous les inventaires enregistrés
   - Pour chaque : `sync_contents(inventory)` → `send_inventory_content_packets()` two-phase

3. **`InventoryManager::sync_contents(inventory)`** :
   - Récupère windowId associé (ou UI=124 si complex)
   - Track les ItemStackInfo (stackId, requestId)
   - Appelle `send_inventory_content_packets(windowId, wrappers)`

4. **`InventoryManager::send_inventory_content_packets(windowId, wrappers)`** :
   - **Phase 1** : envoie `InventoryContent(windowId, [air×N], FullContainerName(last_id), air)`
   - **Phase 2** : envoie `InventoryContent(windowId, wrappers, FullContainerName(last_id), air)`

5. **`InventoryManager::send_inventory_slot_packets(windowId, slot, wrapper)`** :
   - Si `wrapper.stack_id != 0` : envoie d'abord un slot vide (clear)
   - Puis envoie le vrai slot

6. **`InventoryManager::sync_slot(inventory, slot, item_stack)`** :
   - Récupère windowId + ItemStackInfo
   - Si OFFHAND : utilise `send_inventory_content_packets` (bug client connu)
   - Sinon : `send_inventory_slot_packets`

7. **`InventoryManager::on_client_open_main_inventory()`** :
   - `on_current_window_remove()` (envoie ContainerClose si fenêtre ouverte + set pending_close)
   - `open_window_deferred(callback)` :
     - Si pending_close → store callback
     - Sinon : exécute immédiatement
   - Callback :
     - `windowId = get_new_window_id()`
     - `associate_id_with_inventory(windowId, main_inventory)`
     - `current_window_type = INVENTORY (-1)`
     - Send `ContainerOpen::entity_inventory(windowId, INVENTORY, player.entity_id)`

8. **`InventoryManager::on_client_remove_window(windowId)`** :
   - Si `windowId == pending_close_window_id` :
     - `pending_close_window_id = None`
     - Si `pending_open_callback` : exécuter + clear

9. **`InventoryManager::handle_item_stack_request(request)`** :
   - Pour chaque action : valider et appliquer
   - Construire un `ItemStackResponseBuilder` avec les slots changés
   - Envoyer `ItemStackResponsePacket`

### Phase C — Brancher dans Connection

1. Remplacer le code dans `connection/inventory.rs` :
   - `handle_interact(action=6)` → `inventory_manager.on_client_open_main_inventory()`
   - `handle_container_close()` → `inventory_manager.on_client_remove_window(windowId)`
   - `handle_item_stack_request()` → `inventory_manager.handle_item_stack_request(request)`
   - `handle_inventory_transaction()` → router selon type
   - `push_inventory_sync()` → `inventory_manager.sync_all()`

2. Au spawn (`connection/spawn.rs`) :
   - Après `SetActorData` : `inventory_manager.register_inventories()`
   - Puis `inventory_manager.sync_all()`

### Phase D — Items au sol

1. **Vérifier `entity::item_metadata()`** complète :
   - Tous les flags PMMP listés en §3.9
   - Bounding box 0.25×0.25
   - Scale 1.0

2. **Vérifier `ItemStackWrapper::legacy()` pour AddItemActor** :
   - Doit utiliser stack_id=1 (legacy mode pour ItemEntity uniquement)
   - Les autres usages doivent utiliser le stackId du tracking

3. **Tester** : casser un bloc → item visible (pas une ombre) → tombe → ramassable.

### Phase E — Drop depuis l'inventaire

1. **`ItemStackRequest::Drop`** dans `inventory_manager.handle_item_stack_request()` :
   - Décrémente le slot source
   - Spawn item entity à `player.position + (0, 1.3, 0)` avec motion `direction * 0.4`
   - Build response avec le slot changé

2. **Legacy `InventoryTransaction::Normal` avec SOURCE_WORLD** :
   - Identifier les actions de drop
   - Spawn item entity équivalent
   - Resync inventory

### Phase F — Tests E2E manuels

Tests à passer avant de considérer la phase terminée :

1. ✅ Connexion → spawn → voir le monde
2. ✅ Casser un bloc → voir l'item au sol (PAS une ombre)
3. ✅ Marcher sur l'item → ramassé → animation TakeItemActor → item dans hotbar
4. ✅ Touche E → inventaire s'ouvre SANS crash
5. ✅ Drag d'un item dans l'inventaire → l'item bouge
6. ✅ Drop d'un item (touche Q ou drag hors UI) → item au sol
7. ✅ Fermer l'inventaire → ContainerClose → re-ouvrir → re-marche
8. ✅ Casser plusieurs blocs rapidement → tous les items spawnent → pas de crash
9. ✅ Empiler items dans la hotbar (count > 1) → count visible
10. ✅ Reconnexion → l'inventaire est sauvegardé (déjà fait via `player_data.rs`)

---

## 5. Pièges connus et règles absolues

### Règle 1 : PMMP est la SEULE référence
- Pas dragonfly, pas gophertunnel, pas bedrock-rs.
- PMMP est en protocol 924, on cible 944. **Différences à connaître** :
  - `SetActorMotionPacket` ajoute un champ `tick: VarU64` à la fin (déjà fixé dans `crates/mc-rs-proto/src/packets/player.rs`)
  - `BlockPosition.y` reste en `VarUInt32` (PMMP comportement, vérifié)
  - Pour le reste de l'inventaire : aucune différence connue entre 924 et 944.

### Règle 2 : Two-phase sync est OBLIGATOIRE
Sans la phase 1 "tout en air", le client BEDROCK 1.20.12+ ignore les changements de stackId et crash à l'ouverture inventaire.

### Règle 3 : `lastInventoryNetworkId` ≠ `windowId`
- `windowId` = fixé par container type (0=inv, 119=offhand, 120=armor, 124=UI, ou dynamique 1-99 pour fenêtres)
- `lastInventoryNetworkId` = compteur dynamique incrémental utilisé dans `FullContainerName` pour tracker la "version" de la fenêtre côté client

### Règle 4 : ContainerOpen DOIT être précédé d'un ContainerClose si une fenêtre est ouverte
Sinon le client crash.

### Règle 5 : Le client renvoie un ContainerClose après son propre ACK
Il faut attendre cet ACK (`pending_close_window_id`) avant de pouvoir ouvrir une nouvelle fenêtre dynamique.

### Règle 6 : Stack IDs uniques per item instance
Hardcoder `stack_id=1` casse tout. Chaque item dans un slot doit avoir un stackId unique (`nextItemStackId++`).

### Règle 7 : `coreItemStackToNet` est crucial
Le wrapper doit avoir le bon `block_runtime_id` quand l'item est un bloc (sinon le client n'affiche rien). PMMP fait : `Item → ItemStack(net_id, count, meta, block_rid_si_block, raw_extra_data)`.

### Règle 8 : Ne JAMAIS bricoler les paquets pour faire marcher ; porter le système
Les paquets sont la **conséquence** d'un système cohérent. Sans le système (InventoryManager), les paquets ne tiennent pas debout.

---

## 6. Fichiers PMMP à lire INTÉGRALEMENT

```
.reference/PocketMine-MP/src/network/mcpe/InventoryManager.php
.reference/PocketMine-MP/src/network/mcpe/handler/InGamePacketHandler.php
  → handleInteract, handleContainerClose, handleMobEquipment,
    handleItemStackRequest, handleInventoryTransaction
.reference/PocketMine-MP/src/network/mcpe/handler/ItemStackRequestExecutor.php
.reference/PocketMine-MP/src/network/mcpe/handler/ItemStackResponseBuilder.php
.reference/PocketMine-MP/src/network/mcpe/handler/UIInventorySlotOffset.php
.reference/PocketMine-MP/src/inventory/Inventory.php
.reference/PocketMine-MP/src/inventory/BaseInventory.php
.reference/PocketMine-MP/src/inventory/SimpleInventory.php
.reference/PocketMine-MP/src/inventory/PlayerInventory.php
.reference/PocketMine-MP/src/inventory/PlayerOffHandInventory.php
.reference/PocketMine-MP/src/inventory/PlayerCursorInventory.php
.reference/PocketMine-MP/src/inventory/ArmorInventory.php
.reference/PocketMine-MP/src/inventory/transaction/InventoryTransaction.php
.reference/PocketMine-MP/src/inventory/transaction/action/SlotChangeAction.php
.reference/PocketMine-MP/src/inventory/transaction/action/CreateItemAction.php
.reference/PocketMine-MP/src/inventory/transaction/action/DestroyItemAction.php
.reference/PocketMine-MP/src/inventory/transaction/action/DropItemAction.php
.reference/PocketMine-MP/src/entity/object/ItemEntity.php
.reference/PocketMine-MP/src/world/World.php
  → useBreakOn, dropItem
.reference/PocketMine-MP/src/player/Player.php
  → breakBlock, dropItem
.reference/PocketMine-MP/vendor/pocketmine/bedrock-protocol/src/types/inventory/
  → ContainerIds.php, WindowTypes.php, FullContainerName.php,
    ItemStack.php, ItemStackWrapper.php, ItemStackExtraData.php
.reference/PocketMine-MP/vendor/pocketmine/bedrock-protocol/src/
  → ContainerOpenPacket.php, ContainerClosePacket.php,
    InventoryContentPacket.php, InventorySlotPacket.php,
    MobEquipmentPacket.php, AddItemActorPacket.php, TakeItemActorPacket.php,
    ItemStackRequestPacket.php, ItemStackResponsePacket.php
```

---

## 7. État du code après préparation (avant le démarrage de l'implémentation)

- `connection/movement.rs` : drops d'items réactivés
- `connection/inventory.rs` : E key envoie ContainerOpen format PMMP entityInv (mais flow incomplet)
- `connection/inventory.rs:push_inventory_sync` : envoie main + UI(124) + offhand + armor + MobEquipment (single-phase, FullContainerName(0))
- Tous les autres handlers existants restent en place
- Tests `cargo test` passent (les tests existants n'évaluent pas le crash client)

**Bugs à fixer dans la nouvelle session (ordre logique)** :
1. AddItemActor → items rendus en ombre + crash après plusieurs spawn
2. ContainerOpen → crash instant à l'ouverture inventaire
3. Inventory sync → items pas affichés correctement dans la hotbar
4. ItemStackRequest → drag/drop dans l'inventaire ne fonctionne pas

**Cause racine commune** : pas d'`InventoryManager` PMMP. Tout le système doit être porté.
