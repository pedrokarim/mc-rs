# 09 - Inventory System

## PocketMine : Système d'inventaire

### Hiérarchie

```
Inventory (interface)
└── BaseInventory (abstract)
    └── SimpleInventory
        ├── PlayerInventory (36 slots : 27 main + 9 hotbar)
        ├── ArmorInventory (4 slots)
        ├── PlayerOffHandInventory (1 slot)
        ├── PlayerEnderInventory (27 slots)
        ├── CraftingGrid (abstract, 4 ou 9 slots)
        ├── CreativeInventory
        └── [Block inventories]
            ├── ChestInventory (27 slots)
            ├── DoubleChestInventory (54 slots)
            ├── FurnaceInventory (3 slots)
            ├── HopperInventory (5 slots)
            ├── DropperInventory (9 slots)
            ├── BrewingStandInventory (5 slots)
            ├── EnchantingTableInventory (2 slots)
            ├── AnvilInventory (3 slots)
            ├── ShulkerBoxInventory (27 slots)
            ├── BarrelInventory (27 slots)
            └── CampfireInventory (4 slots)
```

### Inventory Interface

```php
interface Inventory {
    const MAX_STACK = 64;

    getSize() → int
    getMaxStackSize() → int

    getItem(index) → Item
    setItem(index, Item) → void
    getContents() → Item[]
    setContents(Item[]) → void

    addItem(Item...) → Item[]        // retourne ce qui n'a pas rentré
    removeItem(Item...) → Item[]     // retourne ce qui n'a pas été retiré
    canAddItem(Item) → bool
    contains(Item) → bool
    first(Item) → int                // premier slot avec cet item
    firstEmpty() → int

    getViewers() → Player[]
    getListeners() → InventoryListener[]
}
```

### PlayerInventory (36 slots)

```
Slots 0-8   : Hotbar (barre rapide)
Slots 9-35  : Main inventory (grille 3x9)

getHeldItemIndex() → 0-8 (slot sélectionné dans la hotbar)
getItemInHand() → Item du slot sélectionné
```

### Système de transactions

Toute modification multi-slot est une **transaction atomique** :

```
InventoryTransaction
├── source: Player
├── actions: InventoryAction[]
│   ├── SlotChangeAction (slot → nouvel item)
│   ├── CreateItemAction (création d'item, ex: résultat craft)
│   ├── DestroyItemAction (destruction d'item)
│   └── DropItemAction (drop au sol)
└── validate() + execute()
```

**Validation :**
- Total items entrants = Total items sortants (conservation)
- Chaque slot source correspond à l'état actuel
- Stack size respecté

**Flux :**
1. Client envoie `ItemStackRequest`
2. Serveur décode en `InventoryTransaction`
3. Validation (événement `InventoryTransactionEvent`)
4. Exécution atomique
5. Réponse `ItemStackResponse` au client

### Listeners

```php
interface InventoryListener {
    onSlotChange(Inventory, slot, oldItem) → void
    onContentChange(Inventory, oldContents) → void
}
```

Les listeners sont notifiés à chaque changement de slot. Le `InventoryManager` (réseau) écoute pour envoyer les mises à jour au client.

### Fichiers PocketMine de référence

```
src/inventory/Inventory.php
src/inventory/BaseInventory.php
src/inventory/SimpleInventory.php
src/inventory/PlayerInventory.php
src/inventory/ArmorInventory.php
src/inventory/transaction/InventoryTransaction.php
src/inventory/transaction/action/*.php
src/network/mcpe/InventoryManager.php
src/network/mcpe/handler/ItemStackRequestExecutor.php
```

---

## Équivalent Rust

### Crate : `mc-rs-inventory`

```rust
/// Inventaire générique de taille fixe
pub struct Inventory<const N: usize> {
    slots: [ItemStack; N],
    max_stack_size: u8,
    listeners: Vec<Box<dyn InventoryListener>>,
}

impl<const N: usize> Inventory<N> {
    pub fn new() -> Self {
        Self {
            slots: [ItemStack::EMPTY; N],
            max_stack_size: 64,
            listeners: Vec::new(),
        }
    }

    pub fn size(&self) -> usize { N }

    pub fn get(&self, slot: usize) -> &ItemStack {
        &self.slots[slot]
    }

    pub fn set(&mut self, slot: usize, item: ItemStack) {
        let old = std::mem::replace(&mut self.slots[slot], item);
        self.notify_slot_change(slot, &old);
    }

    pub fn add_item(&mut self, item: ItemStack) -> ItemStack {
        // Essayer de stack avec les items existants, puis les slots vides
        // Retourne le reste
        todo!()
    }

    pub fn remove_item(&mut self, item: &ItemStack) -> u32 {
        // Retourne le nombre effectivement retiré
        todo!()
    }

    pub fn first_empty(&self) -> Option<usize> {
        self.slots.iter().position(|s| s.is_empty())
    }

    pub fn contains(&self, item: &ItemStack) -> bool {
        self.slots.iter().any(|s| s.matches(item))
    }

    fn notify_slot_change(&self, slot: usize, old: &ItemStack) {
        for listener in &self.listeners {
            listener.on_slot_change(slot, old);
        }
    }
}

/// Inventaire joueur
pub struct PlayerInventory {
    pub main: Inventory<36>,      // 9 hotbar + 27 main
    pub armor: Inventory<4>,
    pub off_hand: Inventory<1>,
    pub cursor: Inventory<1>,     // item sur le curseur
    held_slot: u8,                // 0-8
}

impl PlayerInventory {
    pub fn held_item(&self) -> &ItemStack {
        self.main.get(self.held_slot as usize)
    }

    pub fn set_held_slot(&mut self, slot: u8) {
        assert!(slot < 9);
        self.held_slot = slot;
    }
}

/// Listener d'inventaire
pub trait InventoryListener: Send + Sync {
    fn on_slot_change(&self, slot: usize, old_item: &ItemStack);
    fn on_content_change(&self, old_contents: &[ItemStack]);
}

/// Transaction d'inventaire (atomique)
pub struct InventoryTransaction {
    actions: Vec<InventoryAction>,
}

pub enum InventoryAction {
    SlotChange {
        inventory_id: InventoryId,
        slot: usize,
        source_item: ItemStack,
        target_item: ItemStack,
    },
    CreateItem(ItemStack),
    DestroyItem(ItemStack),
    DropItem(ItemStack),
}

impl InventoryTransaction {
    pub fn validate(&self) -> Result<()> {
        // Vérifier la conservation des items
        // Vérifier que les source_items correspondent à l'état actuel
        todo!()
    }

    pub fn execute(&self, inventories: &mut InventoryManager) -> Result<()> {
        self.validate()?;
        // Appliquer toutes les actions atomiquement
        for action in &self.actions {
            match action {
                InventoryAction::SlotChange { inventory_id, slot, target_item, .. } => {
                    inventories.get_mut(*inventory_id)?.set(*slot, target_item.clone());
                }
                InventoryAction::DropItem(item) => {
                    // Créer ItemEntity dans le monde
                }
                _ => {}
            }
        }
        Ok(())
    }
}
```
