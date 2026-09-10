# 08 - Item System

## PocketMine : Système d'items

### Structure

```
Item
├── identifier: ItemIdentifier (type_id)
├── count: int (1-64)
├── nbt: CompoundTag? (données NBT custom)
├── customName: string?
├── lore: string[]
├── enchantments: EnchantmentInstance[]
├── canPlaceOn: string[]
├── canDestroy: string[]
└── blockData: int (pour les blocs en tant qu'items)
```

### Types d'items (~130 classes)

**Armes :**
- Sword (Wood, Stone, Iron, Gold, Diamond, Netherite)
- Bow, Crossbow, Trident

**Outils :**
- Pickaxe, Axe, Shovel, Hoe (5 matériaux chacun)
- Shears, FishingRod, FlintAndSteel

**Armure :**
- Helmet, Chestplate, Leggings, Boots (Leather, Chain, Iron, Gold, Diamond, Netherite)

**Nourriture :**
- Apple, GoldenApple, Bread, Steak, Porkchop, Chicken, Fish, Cake, Cookie, Melon, Carrot, Potato, Beetroot, Mushroom Stew, etc.

**Projectiles :**
- Egg, Snowball, EnderPearl, SplashPotion, LingeringPotion, ExperienceBottle

**Blocs en items :**
- `ItemBlock` : wrapper qui transforme un Block en Item

**Divers :**
- Bucket, Compass, Clock, Map, Book, NameTag, String, Redstone, Coal, Diamond, Emerald, etc.

### Item durabilité

```php
class Durable extends Item {
    maxDurability: int       // max avant cassure
    damage: int              // dégâts actuels
    unbreakable: bool

    applyDamage(amount) → void
    isBroken() → bool       // damage >= maxDurability
}
```

**Durabilités par matériau :**

| Matériau | Durabilité |
|---|---|
| Gold | 32 |
| Wood | 59 |
| Stone | 131 |
| Iron | 250 |
| Diamond | 1561 |
| Netherite | 2031 |

### Enchantements

```php
class EnchantmentInstance {
    type: Enchantment      // type d'enchantement
    level: int             // niveau (1-5 typiquement)
}
```

**Enchantements principaux :**

| Enchantement | Max Level | Applicable à |
|---|---|---|
| Protection | 4 | Armure |
| Sharpness | 5 | Épée |
| Efficiency | 5 | Outils de minage |
| Unbreaking | 3 | Tous durables |
| Fortune | 3 | Pickaxe, Shovel, Hoe |
| Silk Touch | 1 | Pickaxe, Shovel, Axe |
| Looting | 3 | Épée |
| Power | 5 | Arc |
| Punch | 2 | Arc |
| Infinity | 1 | Arc |
| Mending | 1 | Tous durables |
| Fire Aspect | 2 | Épée |
| Knockback | 2 | Épée |
| Thorns | 3 | Armure |

### Sérialisation réseau des items

```
Pour un ItemStack réseau :
  [VarInt32 network_id]          → 0 = air/vide
  Si network_id != 0 :
    [u16_le count]
    [VarUInt32 aux_value]        → damage/metadata
    [bool has_nbt]
    Si has_nbt :
      [i16_le nbt_version]       → -1
      [u8 nbt_count]             → 1
      [NBT compound]
    [VarUInt32 can_place_on_count]
    [String can_place_on[]]
    [VarUInt32 can_destroy_count]
    [String can_destroy[]]
    Si network_id == shield :
      [VarInt64 blocking_tick]
```

### StringToItemParser

Permet de parser des items depuis des strings :
- `"stone"` → Stone block item
- `"diamond_sword"` → Diamond Sword
- `"potion:8"` → Potion with aux value 8

### Fichiers PocketMine de référence

```
src/item/Item.php                  → Base
src/item/ItemIdentifier.php        → Identifiant
src/item/ItemBlock.php             → Bloc en item
src/item/Durable.php               → Items durables
src/item/Tool.php                  → Outils
src/item/TieredTool.php            → Outils par tier
src/item/Sword.php, Pickaxe.php... → Types spécifiques
src/item/Armor.php                 → Armure
src/item/Food.php                  → Nourriture
src/item/enchantment/              → Enchantements
generated/item/VanillaItems.php    → Registre
src/item/StringToItemParser.php    → Parser
```

---

## Équivalent Rust

### Crate : `mc-rs-item`

```rust
/// Identifiant de type d'item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemTypeId(pub u32);

/// Stack d'items
#[derive(Debug, Clone)]
pub struct ItemStack {
    pub type_id: ItemTypeId,
    pub count: u8,           // 0 = vide, 1-64
    pub damage: u16,         // durabilité / metadata
    pub nbt: Option<NbtCompound>,
    pub custom_name: Option<String>,
    pub lore: Vec<String>,
    pub enchantments: Vec<EnchantmentInstance>,
    pub can_place_on: Vec<String>,
    pub can_destroy: Vec<String>,
}

impl ItemStack {
    pub const EMPTY: Self = Self {
        type_id: ItemTypeId(0),
        count: 0,
        damage: 0,
        nbt: None,
        custom_name: None,
        lore: Vec::new(),
        enchantments: Vec::new(),
        can_place_on: Vec::new(),
        can_destroy: Vec::new(),
    };

    pub fn is_empty(&self) -> bool {
        self.count == 0 || self.type_id.0 == 0
    }

    pub fn max_stack_size(&self) -> u8 {
        // Lookup dans le registre
        ItemRegistry::global().max_stack_size(self.type_id)
    }
}

/// Enchantement appliqué
#[derive(Debug, Clone)]
pub struct EnchantmentInstance {
    pub enchantment: EnchantmentType,
    pub level: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EnchantmentType {
    Protection,
    FireProtection,
    FeatherFalling,
    BlastProtection,
    ProjectileProtection,
    Sharpness,
    Smite,
    BaneOfArthropods,
    Knockback,
    FireAspect,
    Looting,
    Efficiency,
    SilkTouch,
    Unbreaking,
    Fortune,
    Power,
    Punch,
    Flame,
    Infinity,
    LuckOfTheSea,
    Lure,
    Mending,
    Thorns,
    // ... etc
}

/// Trait pour le comportement des items
pub trait ItemBehavior: Send + Sync {
    fn max_stack_size(&self) -> u8 { 64 }
    fn max_durability(&self) -> Option<u16> { None }
    fn on_use(&self, ctx: &mut ItemUseContext) -> bool { false }
    fn on_attack(&self, ctx: &mut ItemAttackContext) -> bool { false }
    fn tool_type(&self) -> ToolType { ToolType::None }
    fn mining_speed(&self, block: BlockState) -> f32 { 1.0 }
    fn attack_damage(&self) -> f32 { 1.0 }
    fn is_food(&self) -> bool { false }
    fn food_properties(&self) -> Option<FoodProperties> { None }
    fn armor_properties(&self) -> Option<ArmorProperties> { None }
}

pub struct FoodProperties {
    pub nutrition: u32,
    pub saturation: f32,
    pub can_always_eat: bool,
}

pub struct ArmorProperties {
    pub slot: ArmorSlot,
    pub defense: u32,
    pub toughness: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum ArmorSlot {
    Helmet,
    Chestplate,
    Leggings,
    Boots,
}

/// Registre global des items
pub struct ItemRegistry {
    behaviors: HashMap<ItemTypeId, Box<dyn ItemBehavior>>,
    string_to_id: HashMap<String, ItemTypeId>,
    id_to_string: HashMap<ItemTypeId, String>,
    id_to_network: HashMap<ItemTypeId, i32>,
}
```
