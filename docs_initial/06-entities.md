# 06 - Système d'entités et ECS

## Pourquoi un ECS ?

Un Entity Component System est le pattern idéal pour un serveur Minecraft :

| Approche OOP traditionnelle | Approche ECS |
|----------------------------|--------------|
| `class Zombie extends HostileMob extends Mob extends Entity` | Entity + `Health` + `Position` + `AI` + `MeleeAttack` |
| Hiérarchie rigide, problème du diamant | Composition flexible |
| Cache-unfriendly (données dispersées) | Cache-friendly (données groupées par type) |
| Difficile à paralléliser | Systèmes parallélisables automatiquement |

### ECS choisi : `bevy_ecs`

`bevy_ecs` peut être utilisé **sans le moteur Bevy complet** :

```toml
[dependencies]
bevy_ecs = "0.15"  # Standalone, pas besoin de bevy complet
```

Fonctionnalités clés :
- **Systèmes parallèles** automatiques
- **Change detection** — savoir quels composants ont changé (utile pour envoyer les updates réseau)
- **Events** — système d'événements intégré
- **Resources** — singletons globaux (config serveur, seed, etc.)
- **Queries** — requêtes ergonomiques sur les entités

## Composants (Components)

### Composants de base

```rust
// Position dans le monde
#[derive(Component)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

// Rotation
#[derive(Component)]
pub struct Rotation {
    pub yaw: f32,
    pub pitch: f32,
    pub head_yaw: f32,
}

// Vélocité
#[derive(Component)]
pub struct Velocity {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

// Bounding box (AABB)
#[derive(Component)]
pub struct BoundingBox {
    pub width: f32,
    pub height: f32,
}

// IDs
#[derive(Component)]
pub struct EntityIds {
    pub unique_id: i64,     // Persistant, unique par monde
    pub runtime_id: u64,    // Par session, pour le réseau
}

// Santé
#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

// Monde/dimension
#[derive(Component)]
pub struct InWorld {
    pub world_id: WorldId,
    pub dimension: Dimension,
}

// On-ground flag
#[derive(Component)]
pub struct OnGround(pub bool);
```

### Composants spécifiques aux joueurs

```rust
// Marqueur joueur
#[derive(Component)]
pub struct Player;

// Connexion réseau
#[derive(Component)]
pub struct NetworkSession {
    pub connection_id: ConnectionId,
    pub encryption_enabled: bool,
    pub compression_enabled: bool,
}

// Identité Xbox
#[derive(Component)]
pub struct XboxIdentity {
    pub xuid: String,
    pub gamertag: String,
    pub uuid: Uuid,
}

// Inventaire joueur
#[derive(Component)]
pub struct PlayerInventory {
    pub main: [ItemStack; 36],
    pub armor: [ItemStack; 4],
    pub offhand: ItemStack,
    pub cursor: ItemStack,
    pub selected_slot: u8,
}

// Gamemode
#[derive(Component)]
pub struct GameMode(pub GameModeType);

// Permissions
#[derive(Component)]
pub struct PermissionLevel(pub u8);

// Chunk loading
#[derive(Component)]
pub struct ChunkLoader {
    pub view_distance: u32,
    pub loaded_chunks: HashSet<ChunkPos>,
    pub pending_chunks: VecDeque<ChunkPos>,
}

// Skin
#[derive(Component)]
pub struct SkinData {
    pub skin_id: String,
    pub skin_data: Vec<u8>,
    pub cape_data: Vec<u8>,
    pub geometry_data: String,
    pub animation_data: String,
}

// Abilités
#[derive(Component)]
pub struct Abilities {
    pub fly: bool,
    pub no_clip: bool,
    pub operator: bool,
    pub teleport: bool,
    pub invulnerable: bool,
    pub build: bool,
    pub mine: bool,
    pub doors_and_switches: bool,
    pub open_containers: bool,
    pub attack_players: bool,
    pub attack_mobs: bool,
    pub fly_speed: f32,
    pub walk_speed: f32,
}

// Faim
#[derive(Component)]
pub struct Hunger {
    pub food_level: i32,       // 0-20
    pub saturation: f32,       // 0.0-20.0
    pub exhaustion: f32,       // Accumulateur, reset quand >= 4.0
    pub food_tick_timer: u32,  // Timer pour la régénération
}

// Expérience
#[derive(Component)]
pub struct Experience {
    pub level: i32,
    pub points: i32,
}
```

### Composants pour les mobs

```rust
// Marqueur mob
#[derive(Component)]
pub struct Mob;

// Type d'entité
#[derive(Component)]
pub struct EntityType(pub String);  // "minecraft:zombie", "minecraft:creeper", etc.

// AI
#[derive(Component)]
pub struct AiGoals {
    pub goals: Vec<AiGoal>,
}

// Pathfinding
#[derive(Component)]
pub struct Pathfinder {
    pub current_path: Option<Vec<BlockPos>>,
    pub target: Option<Vec3>,
    pub speed: f32,
    pub navigation_type: NavigationType,
}

pub enum NavigationType {
    Walk,
    Fly,
    Swim,
    Climb,
    Hover,
}

// Attributs
#[derive(Component)]
pub struct Attributes {
    pub entries: HashMap<String, Attribute>,
}

pub struct Attribute {
    pub base: f32,
    pub current: f32,
    pub min: f32,
    pub max: f32,
    pub modifiers: Vec<AttributeModifier>,
}

// Loot table
#[derive(Component)]
pub struct LootTable(pub String);

// Spawn source
#[derive(Component)]
pub struct SpawnSource(pub SpawnReason);

pub enum SpawnReason {
    Natural,
    Spawner,
    Command,
    SpawnEgg,
    Jockey,
    Event,
}

// Despawn timer
#[derive(Component)]
pub struct DespawnTimer {
    pub no_player_ticks: u32,  // Ticks sans joueur à proximité
}

// Aggro
#[derive(Component)]
pub struct AggroTarget(pub Option<Entity>);
```

### Composants pour les items au sol

```rust
#[derive(Component)]
pub struct DroppedItem {
    pub item: ItemStack,
    pub pickup_delay: u32,   // Ticks avant ramassage possible
    pub age: u32,            // Ticks depuis le drop (despawn à 6000)
    pub thrower: Option<Entity>,
}
```

### Composants pour les projectiles

```rust
#[derive(Component)]
pub struct Projectile {
    pub shooter: Option<Entity>,
    pub damage: f32,
    pub gravity: f32,
}
```

## Entity Metadata (synchronisation réseau)

Bedrock utilise un système de metadata (Synced Data) pour synchroniser l'état des entités :

### Metadata Keys principales

| Key | Nom | Type | Description |
|-----|-----|------|-------------|
| 0 | FLAGS | Int64 | Bitfield d'états (sneaking, sprinting, etc.) |
| 1 | HEALTH | Int | Points de vie |
| 2 | VARIANT | Int | Variante (type de cheval, couleur de chat...) |
| 3 | COLOR | Byte | Couleur (mouton, collier de loup...) |
| 4 | NAMETAG | String | Nom affiché au-dessus |
| 5 | OWNER_EID | Int64 | ID du propriétaire (apprivoisé) |
| 6 | TARGET_EID | Int64 | ID de la cible |
| 7 | AIR_SUPPLY | Short | Air restant sous l'eau |
| 8 | EFFECT_COLOR | Int | Couleur des effets de potion |
| 9 | EFFECT_AMBIENT | Byte | Effets ambiants |
| 14 | PLAYER_FLAGS | Byte | Flags joueur |
| 15 | PLAYER_INDEX | Int | Index joueur |
| 23 | SCALE | Float | Échelle de l'entité |
| 38 | BOUNDING_BOX_WIDTH | Float | Largeur AABB |
| 39 | BOUNDING_BOX_HEIGHT | Float | Hauteur AABB |
| 40 | FUSE_LENGTH | Int | Longueur fusible TNT |
| 45 | MAX_AIR_SUPPLY | Short | Air maximum |
| 53 | FLAGS2 | Int64 | Bitfield d'états étendu |
| 54 | NAMETAG_ALWAYS_SHOW | Byte | Toujours afficher le nom |
| 57 | MAX_AIR_SUPPLY | Short | Air max (doublon) |
| 70 | SKIN_ID | Int | ID de skin |
| 75 | PLAYER_LAST_DEATH_POS | Vec3i | Dernière position de mort |
| 76 | PLAYER_LAST_DEATH_DIM | Int | Dernière dimension de mort |
| 77 | PLAYER_HAS_DIED | Byte | A déjà eu une mort |

### Entity Flags (Key 0 — FLAGS)

| Bit | Nom | Description |
|-----|-----|-------------|
| 0 | ON_FIRE | En feu |
| 1 | SNEAKING | Accroupi |
| 2 | RIDING | Sur une monture |
| 3 | SPRINTING | En sprint |
| 4 | USING_ITEM | Utilise un item |
| 5 | INVISIBLE | Invisible |
| 7 | TEMPTED | Attiré (animal) |
| 8 | IN_LOVE | Mode amour (animal) |
| 9 | SADDLED | Avec selle |
| 10 | POWERED | Chargé (creeper) |
| 11 | IGNITED | Allumé (TNT) |
| 12 | BABY | Bébé |
| 13 | CONVERTING | En conversion |
| 14 | CRITICAL | Coup critique |
| 15 | CAN_SHOW_NAME | Peut afficher le nom |
| 16 | ALWAYS_SHOW_NAME | Nom toujours affiché |
| 17 | NO_AI | Pas d'IA |
| 18 | SILENT | Silencieux |
| 19 | WALL_CLIMBING | Escalade mur (araignée) |
| 20 | CAN_CLIMB | Peut grimper |
| 21 | CAN_SWIM | Peut nager |
| 22 | CAN_FLY | Peut voler |
| 23 | CAN_WALK | Peut marcher |
| 24 | RESTING | Au repos (chauve-souris) |
| 25 | SITTING | Assis |
| 26 | ANGRY | En colère |
| 27 | INTERESTED | Intéressé |
| 28 | CHARGED | Chargé (wither) |
| 29 | TAMED | Apprivoisé |
| 30 | ORPHANED | Orphelin |
| 31 | LEASHED | En laisse |
| 32 | SHEARED | Tondu |
| 33 | GLIDING | En vol plané (élytra) |
| 34 | ELDER | Elder (guardian) |
| 35 | MOVING | En mouvement |
| 36 | BREATHING | Respire |
| 37 | CHESTED | Avec coffre (lama, mule) |
| 38 | STACKABLE | Empilable |
| 39 | SHOW_BOTTOM | Afficher le bas |
| 47 | SWIMMING | Nage |
| 48 | SPIN_ATTACK | Attaque tournante (trident) |
| 55 | TRADE_INTEREST | Intérêt commercial |
| 56 | DOOR_BREAKER | Brise les portes |
| 57 | BREAKING_OBSTRUCTION | Casse obstruction |
| 58 | DOOR_OPENER | Ouvre les portes |
| 60 | HAS_DASH_COOLDOWN | Cooldown de dash |
| 71 | CRAWLING | Rampe |

## Attributs d'entités

### Attributs par défaut

| Attribut | Min | Max | Défaut | Description |
|----------|-----|-----|--------|-------------|
| `minecraft:health` | 0 | varies | 20 | Points de vie |
| `minecraft:follow_range` | 0 | 2048 | 16 | Distance de suivi |
| `minecraft:knockback_resistance` | 0 | 1 | 0 | Résistance au knockback |
| `minecraft:movement` | 0 | 3.4e38 | 0.1 | Vitesse de déplacement |
| `minecraft:underwater_movement` | 0 | 3.4e38 | 0.02 | Vitesse sous l'eau |
| `minecraft:lava_movement` | 0 | 3.4e38 | 0.02 | Vitesse dans la lave |
| `minecraft:attack_damage` | 0 | 3.4e38 | 1 | Dégâts d'attaque |
| `minecraft:absorption` | 0 | 3.4e38 | 0 | Absorption |
| `minecraft:luck` | -1024 | 1024 | 0 | Chance |
| `minecraft:player.hunger` | 0 | 20 | 20 | Faim (joueur) |
| `minecraft:player.saturation` | 0 | 20 | 5 | Saturation (joueur) |
| `minecraft:player.exhaustion` | 0 | 5 | 0 | Épuisement (joueur) |
| `minecraft:player.level` | 0 | 24791 | 0 | Niveau XP (joueur) |
| `minecraft:player.experience` | 0 | 1 | 0 | Barre XP % (joueur) |

### Modificateurs d'attributs

```rust
pub struct AttributeModifier {
    pub id: Uuid,
    pub name: String,
    pub amount: f64,
    pub operation: ModifierOperation,
    pub operand: ModifierOperand,
}

pub enum ModifierOperation {
    Add,
    MultiplyBase,
    MultiplyTotal,
}
```

Calcul : `final = (base + add_modifiers) * (1 + multiply_base_sum) * multiply_total_product`

## Système d'IA (Behaviors)

### Architecture des behaviors

Bedrock utilise un système de **priorité** pour les comportements d'IA :

```rust
pub struct AiGoal {
    pub priority: i32,        // Plus petit = plus prioritaire
    pub behavior: Box<dyn AiBehavior>,
}

pub trait AiBehavior: Send + Sync {
    /// Peut-on commencer ce behavior ?
    fn can_start(&self, entity: &EntityRef, world: &World) -> bool;
    /// Peut-on continuer ?
    fn can_continue(&self, entity: &EntityRef, world: &World) -> bool;
    /// Démarrer le behavior
    fn start(&mut self, entity: &mut EntityMut, world: &mut World);
    /// Tick du behavior
    fn tick(&mut self, entity: &mut EntityMut, world: &mut World);
    /// Arrêter le behavior
    fn stop(&mut self, entity: &mut EntityMut, world: &mut World);
}
```

### Behaviors courants

| Behavior | Priorité typique | Description |
|----------|-------------------|-------------|
| `Float` | 0 | Flotter sur l'eau |
| `Panic` | 1 | Fuir quand blessé (animaux passifs) |
| `MeleeAttack` | 2 | Attaque au corps à corps |
| `RangedAttack` | 2 | Attaque à distance |
| `AvoidMobType` | 3 | Fuir un type de mob |
| `MoveTowardsTarget` | 4 | Se déplacer vers la cible |
| `RandomStroll` | 5 | Marche aléatoire |
| `LookAtPlayer` | 6 | Regarder le joueur le plus proche |
| `RandomLookAround` | 7 | Regarder aléatoirement |
| `TemptGoal` | 3 | Suivre un joueur avec un item |
| `BreedGoal` | 2 | Reproduction |
| `FollowParent` | 4 | Suivre le parent (bébé) |
| `EatBlock` | 5 | Manger de l'herbe (mouton) |
| `HurtByTarget` | 1 | Cibler l'attaquant |
| `NearestAttackableTarget` | 2 | Cibler l'entité la plus proche |

### Pathfinding

```rust
pub enum NavigationType {
    Walk,     // Navigation au sol (A* sur grille 3D)
    Fly,      // Navigation aérienne (A* en 3D)
    Swim,     // Navigation aquatique
    Climb,    // Navigation avec escalade (araignée)
    Hover,    // Survol (abeille)
}

// Algorithme A* avec évaluation des blocs
pub struct PathNode {
    pub pos: BlockPos,
    pub g_cost: f64,      // Coût depuis le départ
    pub h_cost: f64,      // Heuristique vers la destination
    pub f_cost: f64,      // g + h
    pub parent: Option<BlockPos>,
    pub node_type: PathNodeType,
}

pub enum PathNodeType {
    Walkable,
    OpenDoor,
    Water,
    Lava,
    Danger,    // À éviter mais traversable
    Blocked,   // Infranchissable
}
```

## Spawn de mobs

### Règles de spawn naturel

```
Conditions générales :
- Distance du joueur : 24-128 blocs
- Caps par catégorie (par monde) :
  - Hostiles : 70
  - Créatures : 10
  - Aquatiques : 5
  - Ambiants : 15

Conditions par catégorie :
- Hostiles : Light level ≤ 0 (surface) ou partout (grottes), bloc solide en dessous
- Créatures : Light level ≥ 9, bloc herbe en dessous, premier spawn uniquement
- Aquatiques : Dans l'eau, biome approprié
```

### Despawn

```
- Distance > 128 blocs d'un joueur → despawn immédiat
- Distance > 32 blocs pendant 30+ secondes → chance de despawn
- Entités nommées (name tag) ne despawnent jamais
- Entités persistantes (apprivoisées, etc.) ne despawnent jamais
```

## Systèmes ECS

### Exemples de systèmes

```rust
// Système de physique — applique la gravité et les collisions
fn physics_system(
    mut query: Query<(&mut Position, &mut Velocity, &BoundingBox, &mut OnGround)>,
    world_data: Res<WorldData>,
) {
    for (mut pos, mut vel, bbox, mut on_ground) in query.iter_mut() {
        // Gravité
        vel.y -= 0.08;  // blocs/tick²
        vel.y *= 0.98;  // drag aérien

        // Collision et mouvement
        let movement = resolve_collisions(&pos, &vel, bbox, &world_data);
        pos.x += movement.x;
        pos.y += movement.y;
        pos.z += movement.z;

        on_ground.0 = movement.y != vel.y && vel.y < 0.0;

        if on_ground.0 {
            vel.y = 0.0;
            // Friction au sol
            vel.x *= 0.6;
            vel.z *= 0.6;
        }
    }
}

// Système d'envoi des mises à jour de position
fn send_movement_updates(
    query: Query<(&EntityIds, &Position, &Rotation, &OnGround), Changed<Position>>,
    players: Query<(&NetworkSession, &ChunkLoader)>,
) {
    for (ids, pos, rot, on_ground) in query.iter() {
        let packet = MoveEntityAbsolute {
            runtime_id: ids.runtime_id,
            position: pos.into(),
            rotation: rot.into(),
            on_ground: on_ground.0,
        };
        // Envoyer à tous les joueurs qui ont cette entité dans leur view distance
        broadcast_to_viewers(&packet, pos, &players);
    }
}

// Système d'IA
fn ai_system(
    mut query: Query<(&mut AiGoals, &Position, &AggroTarget, &EntityType)>,
    world_data: Res<WorldData>,
) {
    for (mut goals, pos, target, entity_type) in query.iter_mut() {
        // Évaluer les behaviors par priorité
        goals.tick(pos, target, &world_data);
    }
}

// Système de despawn
fn despawn_system(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DespawnTimer, &Position), With<Mob>>,
    players: Query<&Position, With<Player>>,
) {
    for (entity, mut timer, mob_pos) in query.iter_mut() {
        let nearest_player_dist = players.iter()
            .map(|p| p.distance_to(mob_pos))
            .min_by(|a, b| a.partial_cmp(b).unwrap());

        match nearest_player_dist {
            Some(d) if d > 128.0 => { commands.entity(entity).despawn(); }
            Some(d) if d > 32.0 => {
                timer.no_player_ticks += 1;
                if timer.no_player_ticks > 600 { // 30 secondes
                    // Chance de despawn
                }
            }
            _ => { timer.no_player_ticks = 0; }
        }
    }
}
```

## Paquets réseau pour les entités

### Spawn d'entité — AddEntity (0x0D)

```
EntityUniqueID    : VarLong signé
EntityRuntimeID   : VarULong
EntityType        : String ("minecraft:zombie")
Position          : Vec3
Velocity          : Vec3
Rotation          : Vec2 (pitch, yaw)
HeadYaw           : float32_le
BodyYaw           : float32_le
Attributes        : Attribute[]
Metadata          : EntityMetadata
EntityLinks       : EntityLink[]
```

### Spawn de joueur — AddPlayer (0x0C)

```
UUID              : UUID
Username          : String
EntityRuntimeID   : VarULong
PlatformChatId    : String
Position          : Vec3
Velocity          : Vec3
Rotation          : Vec3 (pitch, yaw, head_yaw)
HeldItem          : ItemStack
GameType          : VarInt
Metadata          : EntityMetadata
AbilityData       : UpdateAbilitiesData
EntityLinks       : EntityLink[]
DeviceId          : String
DeviceOS          : int32_le
```

### Runtime ID vs Unique ID

| Type | Format | Persistance | Usage |
|------|--------|-------------|-------|
| **UniqueID** | int64 signé | Persistant (sauvé dans LevelDB) | Identification cross-session |
| **RuntimeID** | VarULong | Par session uniquement | Utilisé dans la majorité des paquets réseau |

Le serveur maintient un mapping bidirectionnel entre les deux.
