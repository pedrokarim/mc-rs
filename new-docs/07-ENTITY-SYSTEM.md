# 07 - Entity System

## PocketMine : Système d'entités

### Hiérarchie des classes

```
Entity (abstract)
├── Living (a de la vie, armure, effets)
│   ├── Human (inventaire, XP, faim, skin)
│   │   └── Player (connecté au réseau)
│   ├── Squid
│   ├── Villager
│   ├── Zombie, Skeleton, Creeper, Spider, ...
│   ├── Cow, Pig, Sheep, Chicken, ...
│   └── Wolf, Cat, Horse, ...
├── Projectile (abstract)
│   ├── Arrow
│   ├── Snowball
│   ├── Egg
│   ├── EnderPearl
│   ├── SplashPotion
│   └── Trident
└── Object
    ├── ItemEntity (item droppé)
    ├── ExperienceOrb
    ├── FallingBlock
    ├── PrimedTNT
    └── Painting
```

### Entity (classe de base)

**Propriétés principales :**

| Propriété | Type | Description |
|---|---|---|
| `id` | int | Runtime ID auto-incrémenté |
| `location` | Location | Position + monde + rotation |
| `motion` | Vector3 | Vélocité |
| `boundingBox` | AxisAlignedBB | Hitbox |
| `health` | float | Points de vie actuels |
| `maxHealth` | int | Points de vie max |
| `ySize` | float | Taille Y (offset) |
| `stepHeight` | float | Hauteur de marche |
| `keepMovement` | bool | Garder le mouvement |
| `fallDistance` | float | Distance de chute |
| `ticksLived` | int | Âge en ticks |
| `fireTicks` | int | Ticks de feu restants |
| `noDamageTicks` | int | Invincibilité |
| `nameTag` | string | Nom affiché |
| `nameTagVisible` | bool | Nom visible |
| `nameTagAlwaysVisible` | bool | Toujours visible |
| `scale` | float | Échelle (1.0 = normal) |
| `invisible` | bool | Invisible |
| `silent` | bool | Pas de sons |
| `gravity` | float | Gravité (0.04 défaut) |
| `drag` | float | Résistance air (0.02) |
| `canClimb` | bool | Peut grimper |

**Tick d'une entité :**
```
1. entityBaseTick()
   - Vérifier position valide (Void damage si Y < -64)
   - Mettre à jour fire ticks
   - Mettre à jour air supply (noyade)
   - Mettre à jour effets de potion
2. Mouvement / physique
   - Appliquer gravité
   - Appliquer drag
   - Vérifier collisions
   - Mettre à jour position
3. updateMovement()
   - Envoyer les changements de position aux viewers
```

### Living

Ajoute à Entity :
- `ArmorInventory` (4 slots)
- `EffectManager` (effets de potion)
- `AttributeMap` (attributs : santé, vitesse, dégâts)
- Knockback
- Respiration / noyade

**Attributs :**

| Attribut | ID | Défaut | Min | Max |
|---|---|---|---|---|
| Health | `minecraft:health` | 20.0 | 0.0 | 20.0 |
| Absorption | `minecraft:absorption` | 0.0 | 0.0 | 16.0 |
| Movement Speed | `minecraft:movement` | 0.1 | 0.0 | 340282346638528859811704183484516925440.0 |
| Attack Damage | `minecraft:attack_damage` | 1.0 | 0.0 | ∞ |
| Knockback Resistance | `minecraft:knockback_resistance` | 0.0 | 0.0 | 1.0 |
| Follow Range | `minecraft:follow_range` | 16.0 | 0.0 | 2048.0 |

### Human

Ajoute à Living :
- `PlayerInventory` (36 slots)
- `EnderChestInventory` (27 slots)
- `ExperienceManager`
- `HungerManager`
- `Skin`
- `UUID`

**HungerManager :**
- `food` : 0-20
- `saturation` : 0.0-20.0
- `exhaustion` : 0.0-4.0
- Tick : exhaustion → saturation → food → dégâts

**ExperienceManager :**
- `currentLevel` : 0-∞
- `currentProgress` : 0.0-1.0
- XP total calculé à partir du niveau
- Enchanting cost basé sur le niveau

### Player

Ajoute à Human :
- `NetworkSession` (connexion réseau)
- `username`, `uuid`, `xboxUserId`
- `gamemode` (Survival, Creative, Adventure, Spectator)
- `viewDistance` (distance de vue en chunks)
- `usedChunks` : set des chunks envoyés
- `ChunkSelector` : algorithme de sélection de chunks

**Chunk loading du joueur :**
1. Calculer les chunks dans le rayon de vue
2. Trier par distance (plus proche en premier)
3. Envoyer `LevelChunkPacket` pour chaque nouveau chunk
4. Décharger les chunks hors de portée

### EntityFactory

Registre de types d'entités → factory functions.

```php
EntityFactory::register(class, function(World, CompoundTag) → Entity)
```

### Entity Metadata (données réseau)

Les entités ont des métadonnées synchronisées avec le client via `SetEntityDataPacket` :

| Flag | Bit | Description |
|---|---|---|
| ON_FIRE | 0 | En feu |
| SNEAKING | 1 | Accroupi |
| RIDING | 2 | Monte une entité |
| SPRINTING | 3 | Sprint |
| USING_ITEM | 4 | Utilise un item |
| INVISIBLE | 5 | Invisible |
| BABY | 8 | Bébé |
| CAN_CLIMB | 16 | Peut grimper |
| SWIMMER | 17 | Nage |
| CAN_FLY | 21 | Peut voler |
| SLEEPING | 26 | Dort |
| NO_AI | 15 | Pas d'IA |
| BREATHING | 35 | Respire |

### Fichiers PocketMine de référence

```
src/entity/Entity.php              → Base
src/entity/Living.php              → Living
src/entity/Human.php               → Human
src/entity/Location.php            → Location
src/entity/Attribute.php           → Attribut
src/entity/AttributeMap.php        → Map d'attributs
src/entity/AttributeFactory.php    → Factory
src/entity/EntityFactory.php       → Registre
src/entity/Skin.php                → Skin
src/entity/effect/                 → Effets de potion
src/entity/projectile/             → Projectiles
src/entity/object/                 → Objets (items, XP, TNT)
src/player/Player.php              → Joueur
src/player/GameMode.php            → Mode de jeu
src/player/ChunkSelector.php       → Sélection chunks
```

---

## Équivalent Rust

### Crate : `mc-rs-entity`

```rust
use std::collections::HashMap;

/// ID unique d'entité (auto-incrémenté)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EntityId(pub u64);

/// Position + rotation + monde
#[derive(Debug, Clone)]
pub struct Location {
    pub world: String,  // nom du monde
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
}

/// Données communes à toutes les entités
#[derive(Debug)]
pub struct EntityBase {
    pub id: EntityId,
    pub location: Location,
    pub motion: Vec3f64,
    pub bounding_box: Aabb,
    pub on_ground: bool,
    pub health: f32,
    pub max_health: f32,
    pub fire_ticks: i32,
    pub fall_distance: f32,
    pub ticks_lived: u64,
    pub no_damage_ticks: i32,
    pub name_tag: String,
    pub name_tag_visible: bool,
    pub scale: f32,
    pub invisible: bool,
    pub gravity: f32,
    pub drag: f32,
    pub metadata: EntityMetadata,
}

/// Trait pour tous les types d'entités
pub trait Entity: Send + Sync {
    fn base(&self) -> &EntityBase;
    fn base_mut(&mut self) -> &mut EntityBase;
    fn entity_type(&self) -> &str; // "minecraft:zombie", etc.

    fn tick(&mut self, world: &mut WorldAccess) {
        self.base_tick(world);
    }

    fn base_tick(&mut self, world: &mut WorldAccess) {
        let base = self.base_mut();
        base.ticks_lived += 1;
        // Fire, void damage, etc.
    }

    fn on_spawn(&mut self) {}
    fn on_despawn(&mut self) {}
    fn save_nbt(&self) -> NbtCompound;
    fn load_nbt(&mut self, nbt: &NbtCompound);
}

/// Entité vivante (a de la vie, des attributs)
pub struct LivingEntity {
    pub base: EntityBase,
    pub attributes: AttributeMap,
    pub effects: EffectManager,
    pub armor: ArmorInventory,
    pub air_supply: i16,
    pub max_air_supply: i16,
}

/// Humain (inventaire, XP, faim)
pub struct HumanEntity {
    pub living: LivingEntity,
    pub inventory: PlayerInventory,
    pub ender_inventory: EnderChestInventory,
    pub experience: ExperienceManager,
    pub hunger: HungerManager,
    pub skin: Skin,
    pub uuid: Uuid,
}

/// Joueur connecté
pub struct Player {
    pub human: HumanEntity,
    pub username: String,
    pub xbox_user_id: String,
    pub gamemode: GameMode,
    pub view_distance: u32,
    pub session: SessionHandle,  // handle vers NetworkSession
    // Chunk management
    pub loaded_chunks: HashSet<ChunkPos>,
    pub chunk_send_queue: VecDeque<ChunkPos>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Survival = 0,
    Creative = 1,
    Adventure = 2,
    Spectator = 6,
}

/// Attributs d'entité
pub struct AttributeMap {
    attributes: HashMap<String, Attribute>,
}

pub struct Attribute {
    pub id: String,
    pub value: f32,
    pub default: f32,
    pub min: f32,
    pub max: f32,
    pub dirty: bool,
}

/// Gestionnaire de faim
pub struct HungerManager {
    pub food: u32,        // 0-20
    pub saturation: f32,  // 0.0-20.0
    pub exhaustion: f32,  // 0.0-4.0
    pub tick_timer: u32,
}

/// Gestionnaire d'XP
pub struct ExperienceManager {
    pub level: u32,
    pub progress: f32,  // 0.0-1.0
}

/// Factory d'entités
pub struct EntityFactory {
    creators: HashMap<String, Box<dyn Fn(Location, &NbtCompound) -> Box<dyn Entity>>>,
}
```
