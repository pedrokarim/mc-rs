# 08 - Système de plugins / mods

## Philosophie

MC-RS adopte une approche **hybride multi-couches** pour les extensions :

```
┌─────────────────────────────────────────────────────┐
│  Couche 3 : Behavior Packs (JSON, Bedrock natif)    │  ← Addons Bedrock
├─────────────────────────────────────────────────────┤
│  Couche 2 : Scripts Lua (mlua, hot-reload)          │  ← Scripts légers
├─────────────────────────────────────────────────────┤
│  Couche 1 : Plugins WASM (wasmtime, sandboxés)      │  ← Plugins tiers
├─────────────────────────────────────────────────────┤
│  Couche 0 : API Rust (crate mc-rs-plugin-api)       │  ← Core/Framework
└─────────────────────────────────────────────────────┘
```

| Couche | Langage | Performance | Sécurité | Hot-reload | Cas d'usage |
|--------|---------|-------------|----------|------------|-------------|
| **Rust API** | Rust | Native | Totale | Non (recompile) | Core, systèmes internes |
| **WASM** | Any→WASM | ~0.8× native | Sandbox | Oui | Plugins communautaires |
| **Lua** | Lua/LuaJIT | ~0.3-0.5× native | Partiel | Oui | Scripts simples, prototypage |
| **Behavior Packs** | JSON | N/A | Totale | Oui | Mods Bedrock natifs |

## Couche 0 : API Rust (mc-rs-plugin-api)

### Trait Plugin

```rust
/// Trait principal qu'un plugin Rust doit implémenter
pub trait Plugin: Send + Sync {
    /// Nom du plugin
    fn name(&self) -> &str;

    /// Version du plugin
    fn version(&self) -> &str;

    /// Appelé au chargement du plugin
    fn on_enable(&mut self, api: &PluginApi) -> Result<()>;

    /// Appelé au déchargement du plugin
    fn on_disable(&mut self) -> Result<()>;
}
```

### Système d'événements

```rust
/// Priorité d'exécution des handlers
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventPriority {
    Lowest = 0,   // Exécuté en premier
    Low = 1,
    Normal = 2,
    High = 3,
    Highest = 4,
    Monitor = 5,  // Exécuté en dernier, lecture seule
}

/// Trait pour les événements cancellables
pub trait Cancellable {
    fn is_cancelled(&self) -> bool;
    fn set_cancelled(&mut self, cancelled: bool);
}

/// Événement de base
pub trait Event: Send + Sync + 'static {
    fn event_name(&self) -> &'static str;
}

/// Registre d'événements
pub struct EventBus {
    handlers: HashMap<TypeId, Vec<HandlerEntry>>,
}

impl EventBus {
    /// Enregistrer un handler
    pub fn subscribe<E: Event>(
        &mut self,
        priority: EventPriority,
        handler: impl Fn(&mut E) + Send + Sync + 'static,
    );

    /// Émettre un événement
    pub fn emit<E: Event>(&self, event: &mut E);
}
```

### Événements disponibles

#### Événements joueur

```rust
pub struct PlayerJoinEvent {
    pub player: PlayerId,
    pub join_message: String,
    cancelled: bool,
}

pub struct PlayerQuitEvent {
    pub player: PlayerId,
    pub quit_message: String,
}

pub struct PlayerChatEvent {
    pub player: PlayerId,
    pub message: String,
    pub format: String,       // "{player}: {message}"
    pub recipients: Vec<PlayerId>,
    cancelled: bool,
}

pub struct PlayerMoveEvent {
    pub player: PlayerId,
    pub from: Position,
    pub to: Position,
    cancelled: bool,          // Cancel = téléporter au from
}

pub struct PlayerBreakBlockEvent {
    pub player: PlayerId,
    pub position: BlockPos,
    pub block: BlockState,
    pub drops: Vec<ItemStack>,
    cancelled: bool,
}

pub struct PlayerPlaceBlockEvent {
    pub player: PlayerId,
    pub position: BlockPos,
    pub block: BlockState,
    pub against: BlockPos,    // Bloc sur lequel on place
    cancelled: bool,
}

pub struct PlayerInteractEvent {
    pub player: PlayerId,
    pub action: InteractAction,
    pub target: Option<EntityId>,
    pub item: Option<ItemStack>,
    cancelled: bool,
}

pub struct PlayerCommandEvent {
    pub player: PlayerId,
    pub command: String,
    pub args: Vec<String>,
    cancelled: bool,
}

pub struct PlayerDeathEvent {
    pub player: PlayerId,
    pub cause: DamageCause,
    pub killer: Option<EntityId>,
    pub death_message: String,
    pub keep_inventory: bool,
}

pub struct PlayerRespawnEvent {
    pub player: PlayerId,
    pub respawn_position: Position,
}

pub struct PlayerChangeDimensionEvent {
    pub player: PlayerId,
    pub from: Dimension,
    pub to: Dimension,
    cancelled: bool,
}
```

#### Événements entité

```rust
pub struct EntityDamageEvent {
    pub entity: EntityId,
    pub damage: f32,
    pub cause: DamageCause,
    pub attacker: Option<EntityId>,
    cancelled: bool,
}

pub struct EntityDeathEvent {
    pub entity: EntityId,
    pub cause: DamageCause,
    pub drops: Vec<ItemStack>,
    pub xp: u32,
}

pub struct EntitySpawnEvent {
    pub entity: EntityId,
    pub position: Position,
    pub spawn_reason: SpawnReason,
    cancelled: bool,
}

pub struct EntityExplodeEvent {
    pub entity: EntityId,
    pub position: Position,
    pub power: f32,
    pub affected_blocks: Vec<BlockPos>,
    cancelled: bool,
}
```

#### Événements monde

```rust
pub struct BlockUpdateEvent {
    pub position: BlockPos,
    pub old_block: BlockState,
    pub new_block: BlockState,
    pub cause: BlockUpdateCause,
    cancelled: bool,
}

pub struct ChunkLoadEvent {
    pub chunk: ChunkPos,
    pub is_new: bool,
}

pub struct ChunkUnloadEvent {
    pub chunk: ChunkPos,
    cancelled: bool,
}

pub struct WeatherChangeEvent {
    pub new_weather: Weather,
    cancelled: bool,
}
```

#### Événements serveur

```rust
pub struct ServerTickEvent {
    pub tick: u64,
}

pub struct ServerStartEvent;
pub struct ServerStopEvent;
```

### API Plugin

```rust
/// API exposée aux plugins
pub struct PluginApi {
    // Joueurs
    pub fn get_player(&self, id: PlayerId) -> Option<PlayerRef>;
    pub fn get_online_players(&self) -> Vec<PlayerRef>;
    pub fn broadcast_message(&self, message: &str);

    // Monde
    pub fn get_world(&self, id: WorldId) -> Option<WorldRef>;
    pub fn get_block(&self, world: WorldId, pos: BlockPos) -> BlockState;
    pub fn set_block(&self, world: WorldId, pos: BlockPos, block: BlockState);

    // Entités
    pub fn spawn_entity(&self, world: WorldId, entity_type: &str, pos: Position) -> EntityId;
    pub fn remove_entity(&self, id: EntityId);

    // Commandes
    pub fn register_command(&self, command: CommandDefinition);

    // Scheduler
    pub fn schedule_task(&self, delay_ticks: u64, task: Box<dyn FnOnce() + Send>);
    pub fn schedule_repeating(&self, period_ticks: u64, task: Box<dyn Fn() + Send>) -> TaskId;
    pub fn cancel_task(&self, id: TaskId);

    // Configuration
    pub fn get_data_folder(&self) -> PathBuf;
    pub fn get_config<T: DeserializeOwned>(&self, name: &str) -> Result<T>;
    pub fn save_config<T: Serialize>(&self, name: &str, config: &T) -> Result<()>;

    // Événements
    pub fn subscribe<E: Event>(&self, priority: EventPriority, handler: impl Fn(&mut E));

    // Logging
    pub fn logger(&self) -> &Logger;

    // Forms
    pub fn send_form(&self, player: PlayerId, form: Form) -> FormResponseFuture;
}
```

### PlayerRef API

```rust
pub struct PlayerRef<'a> { /* ... */ }

impl<'a> PlayerRef<'a> {
    // Identité
    pub fn name(&self) -> &str;
    pub fn uuid(&self) -> Uuid;
    pub fn xuid(&self) -> &str;

    // Position
    pub fn position(&self) -> Position;
    pub fn teleport(&self, pos: Position);
    pub fn dimension(&self) -> Dimension;

    // État
    pub fn health(&self) -> f32;
    pub fn set_health(&self, health: f32);
    pub fn game_mode(&self) -> GameMode;
    pub fn set_game_mode(&self, mode: GameMode);
    pub fn is_op(&self) -> bool;

    // Inventaire
    pub fn inventory(&self) -> &PlayerInventory;
    pub fn give_item(&self, item: ItemStack);
    pub fn clear_inventory(&self);

    // Communication
    pub fn send_message(&self, message: &str);
    pub fn send_title(&self, title: &str, subtitle: &str, fade_in: u32, stay: u32, fade_out: u32);
    pub fn send_action_bar(&self, message: &str);
    pub fn send_toast(&self, title: &str, content: &str);
    pub fn send_form(&self, form: Form) -> FormResponseFuture;

    // Effets
    pub fn add_effect(&self, effect: Effect, duration: u32, amplifier: u8);
    pub fn remove_effect(&self, effect_type: EffectType);

    // Réseau
    pub fn kick(&self, reason: &str);
    pub fn transfer(&self, address: &str, port: u16);
    pub fn ping(&self) -> u32;
}
```

## Couche 1 : Plugins WASM (wasmtime)

### Pourquoi WASM ?

- **Sandbox** : Le plugin ne peut accéder qu'aux APIs explicitement exposées
- **Language-agnostic** : Peut être écrit en Rust, C, Go, AssemblyScript, etc.
- **Hot-reload** : Rechargement sans redémarrage du serveur
- **Sécurité** : Pas de crash du serveur, limites CPU/mémoire
- **Portabilité** : Le même .wasm tourne partout

### Architecture

```
┌──────────────────────────────────────────────┐
│               MC-RS Server (Host)             │
│                                               │
│  ┌─────────────────────────────────────────┐  │
│  │         WASM Host Functions              │  │
│  │  (exportées vers les plugins)            │  │
│  │                                          │  │
│  │  mc_log(level, msg)                      │  │
│  │  mc_get_player_name(player_id) -> str    │  │
│  │  mc_send_message(player_id, msg)         │  │
│  │  mc_get_block(x, y, z) -> block_id       │  │
│  │  mc_set_block(x, y, z, block_id)         │  │
│  │  mc_teleport(player_id, x, y, z)         │  │
│  │  mc_register_command(name, desc, perm)    │  │
│  │  mc_schedule_task(delay_ticks, cb_id)     │  │
│  │  ...                                      │  │
│  └──────────────┬──────────────────────────┘  │
│                 │ Host-Guest boundary          │
│  ┌──────────────┴──────────────────────────┐  │
│  │         WASM Plugin Instance             │  │
│  │  (sandboxé, limites mémoire/CPU)         │  │
│  │                                          │  │
│  │  Exports:                                │  │
│  │    on_enable()                           │  │
│  │    on_disable()                          │  │
│  │    on_player_join(player_id)             │  │
│  │    on_player_chat(player_id, msg) -> bool│  │
│  │    on_command(name, player_id, args)     │  │
│  │    on_tick(tick_number)                  │  │
│  │    ...                                   │  │
│  └──────────────────────────────────────────┘  │
│                                               │
└──────────────────────────────────────────────┘
```

### Configuration du runtime

```rust
use wasmtime::*;

pub struct WasmPluginRuntime {
    engine: Engine,
    plugins: Vec<WasmPlugin>,
}

impl WasmPluginRuntime {
    pub fn new() -> Self {
        let mut config = Config::new();
        config.consume_fuel(true);           // Limite CPU
        config.epoch_interruption(true);     // Interruption par epoch

        let engine = Engine::new(&config).unwrap();
        Self { engine, plugins: vec![] }
    }

    pub fn load_plugin(&mut self, path: &Path) -> Result<()> {
        let module = Module::from_file(&self.engine, path)?;
        let mut store = Store::new(&self.engine, PluginState::new());

        // Limiter la consommation CPU
        store.set_fuel(1_000_000)?; // Fuel initial

        // Limiter la mémoire
        let memory_type = MemoryType::new(1, Some(256)); // 1-256 pages (64KB-16MB)

        let mut linker = Linker::new(&self.engine);
        self.register_host_functions(&mut linker)?;

        let instance = linker.instantiate(&mut store, &module)?;
        // ...
    }
}
```

### Manifest du plugin WASM

```toml
# plugin.toml
[plugin]
name = "MonPlugin"
version = "1.0.0"
author = "Dev"
description = "Un super plugin"
main = "plugin.wasm"

[permissions]
# Quelles APIs le plugin peut utiliser
player = true
world = true
commands = true
scheduler = true
network = false     # Pas d'accès réseau direct
filesystem = false  # Pas d'accès au système de fichiers

[resources]
# Limites
max_memory_mb = 16
max_fuel_per_tick = 100000
```

### Écrire un plugin WASM en Rust

```rust
// Côté plugin (compile vers wasm32-wasi)

// Fonctions importées depuis le host
extern "C" {
    fn mc_log(level: i32, msg_ptr: *const u8, msg_len: u32);
    fn mc_get_player_name(player_id: u64, out_ptr: *mut u8, out_len: *mut u32);
    fn mc_send_message(player_id: u64, msg_ptr: *const u8, msg_len: u32);
    fn mc_broadcast(msg_ptr: *const u8, msg_len: u32);
}

// Fonctions exportées vers le host
#[no_mangle]
pub extern "C" fn on_enable() {
    log_info("MonPlugin activé !");
}

#[no_mangle]
pub extern "C" fn on_player_join(player_id: u64) {
    let name = get_player_name(player_id);
    broadcast(&format!("Bienvenue {} !", name));
}

#[no_mangle]
pub extern "C" fn on_player_chat(player_id: u64, msg_ptr: *const u8, msg_len: u32) -> i32 {
    let msg = unsafe { std::str::from_utf8_unchecked(std::slice::from_raw_parts(msg_ptr, msg_len as usize)) };

    if msg.contains("spam") {
        return 1; // cancelled
    }
    0 // not cancelled
}
```

## Couche 2 : Scripts Lua (mlua)

### Pourquoi Lua ?

- **Familier** aux moddeurs de jeux (Garry's Mod, Roblox, WoW)
- **Hot-reload** instantané (éditer → recharger sans restart)
- **LuaJIT** offre des performances excellentes pour un langage scripté
- **Simple** à apprendre pour les non-programmeurs

### Configuration

```toml
[dependencies]
mlua = { version = "0.10", features = ["luajit", "async", "serialize"] }
```

### API Lua exposée

```lua
-- Exemple de script Lua pour MC-RS

-- Événements
mc.on("player_join", function(event)
    local player = event.player
    mc.broadcast("§a" .. player:name() .. " a rejoint le serveur !")
    player:send_message("§6Bienvenue sur le serveur !")

    -- Donner un kit de départ
    player:give_item("minecraft:diamond_sword", 1)
    player:give_item("minecraft:bread", 16)
end)

mc.on("player_chat", function(event)
    -- Filtrer les mots interdits
    if event.message:find("badword") then
        event:cancel()
        event.player:send_message("§cMessage interdit !")
    end
end)

mc.on("player_break_block", function(event)
    -- Doubler les drops de diamant
    if event.block.name == "minecraft:diamond_ore" then
        event.player:give_item("minecraft:diamond", 1) -- +1 bonus
    end
end)

-- Commandes
mc.register_command("spawn", {
    description = "Téléporte au spawn",
    permission = "mc-rs.command.spawn",
    execute = function(player, args)
        local spawn = mc.get_world("overworld"):get_spawn()
        player:teleport(spawn.x, spawn.y, spawn.z)
        player:send_message("§aTéléporté au spawn !")
    end
})

-- Tâches planifiées
mc.schedule_repeating(20 * 60, function()  -- Toutes les 60 secondes
    mc.broadcast("§eTip: Utilisez /spawn pour retourner au spawn !")
end)

-- Formulaires
mc.on("player_interact", function(event)
    if event.action == "right_click" and event.target_block == "minecraft:chest" then
        local form = mc.form.simple("Menu", "Choisissez :")
            :button("Option 1")
            :button("Option 2")
            :button("Option 3")

        form:send(event.player, function(response)
            if response.index == 0 then
                event.player:send_message("Vous avez choisi l'option 1")
            end
        end)
    end
end)
```

### Structure des scripts

```
plugins/
├── mon_plugin/
│   ├── plugin.toml          # Métadonnées
│   ├── main.lua             # Point d'entrée
│   ├── commands.lua         # Commandes
│   ├── events.lua           # Handlers d'événements
│   └── config.toml          # Configuration du plugin
└── autre_plugin/
    ├── plugin.toml
    └── main.lua
```

### plugin.toml (Lua)

```toml
[plugin]
name = "MonPlugin"
version = "1.0.0"
author = "Dev"
description = "Un plugin Lua"
main = "main.lua"

[dependencies]
# Autres plugins requis
other_plugin = ">=1.0.0"
```

### Sandbox Lua

```rust
// Côté serveur : configuration du sandbox Lua
impl LuaPluginRuntime {
    fn create_sandbox(lua: &Lua) -> Result<()> {
        // Supprimer les fonctions dangereuses
        let globals = lua.globals();
        globals.set("os", mlua::Nil)?;           // Pas d'accès OS
        globals.set("io", mlua::Nil)?;           // Pas d'accès fichiers direct
        globals.set("loadfile", mlua::Nil)?;     // Pas de chargement de fichiers
        globals.set("dofile", mlua::Nil)?;        // Pas d'exécution de fichiers
        globals.set("debug", mlua::Nil)?;         // Pas de debug hooks

        // Limiter la mémoire
        lua.set_memory_limit(16 * 1024 * 1024)?;  // 16 MB max

        // Hook pour limiter le CPU (instruction count)
        lua.set_hook(
            HookTriggers::EVERY_NTH_INSTRUCTION { n: 10000 },
            |_lua, _debug| {
                // Vérifier si le plugin a dépassé son budget CPU
                Ok(())
            }
        )?;

        Ok(())
    }
}
```

## Couche 3 : Behavior Packs Bedrock

### Support natif des addons

MC-RS devrait supporter les behavior packs Bedrock natifs pour maximiser la compatibilité :

### Ce qu'il faut parser

```
behavior_pack/
├── manifest.json              # Métadonnées du pack
├── entities/
│   └── custom_mob.json        # Définitions d'entités custom
├── items/
│   └── custom_item.json       # Définitions d'items custom
├── blocks/
│   └── custom_block.json      # Définitions de blocs custom
├── recipes/
│   ├── shaped_recipe.json     # Recettes avec forme
│   └── shapeless_recipe.json  # Recettes sans forme
├── loot_tables/
│   └── my_loot.json           # Tables de loot
├── trading/
│   └── villager.json          # Tables de commerce
├── spawn_rules/
│   └── custom_mob.json        # Règles de spawn
├── features/
│   └── custom_feature.json    # Génération de monde
├── feature_rules/
│   └── custom_rule.json       # Règles de features
├── scripts/
│   └── main.js               # Script API (@minecraft/server)
└── animations/
    └── controller.json        # Animation controllers
```

### manifest.json

```json
{
    "format_version": 2,
    "header": {
        "name": "Mon Behavior Pack",
        "description": "Description du pack",
        "uuid": "12345678-1234-1234-1234-123456789012",
        "version": [1, 0, 0],
        "min_engine_version": [1, 21, 0]
    },
    "modules": [
        {
            "type": "data",
            "uuid": "87654321-4321-4321-4321-210987654321",
            "version": [1, 0, 0]
        }
    ],
    "dependencies": []
}
```

### Entité custom (JSON)

```json
{
    "format_version": "1.20.0",
    "minecraft:entity": {
        "description": {
            "identifier": "custom:guard",
            "is_spawnable": true,
            "is_summonable": true,
            "is_experimental": false
        },
        "component_groups": {
            "hostile": {
                "minecraft:behavior.melee_attack": {
                    "priority": 3,
                    "speed_multiplier": 1.2,
                    "track_target": true
                }
            }
        },
        "components": {
            "minecraft:health": { "value": 30, "max": 30 },
            "minecraft:movement": { "value": 0.3 },
            "minecraft:collision_box": { "width": 0.6, "height": 1.9 },
            "minecraft:physics": {},
            "minecraft:navigation.walk": {
                "can_path_over_water": true,
                "avoid_water": true
            },
            "minecraft:behavior.random_stroll": { "priority": 6, "speed_multiplier": 0.8 },
            "minecraft:behavior.look_at_player": { "priority": 7, "look_distance": 8 },
            "minecraft:behavior.hurt_by_target": { "priority": 1 }
        },
        "events": {
            "minecraft:entity_spawned": {
                "add": { "component_groups": ["hostile"] }
            }
        }
    }
}
```

### Molang (langage d'expressions)

Le serveur doit pouvoir parser et évaluer les expressions Molang :

```
// Exemples Molang
query.is_sneaking ? 1.0 : 0.0
math.sin(query.life_time * 360) * 0.5 + 0.5
variable.attack_time = query.is_angry ? 1.0 : 0.0
query.health / query.max_health
math.random(0, 10)
query.distance_from_camera < 16 ? 1.0 : 0.0
```

**Fonctions Molang essentielles :**

| Catégorie | Fonctions |
|-----------|-----------|
| Math | `math.sin`, `math.cos`, `math.abs`, `math.min`, `math.max`, `math.floor`, `math.ceil`, `math.round`, `math.sqrt`, `math.random`, `math.clamp`, `math.lerp` |
| Query | `query.health`, `query.max_health`, `query.is_sneaking`, `query.is_sprinting`, `query.life_time`, `query.distance_from_camera`, `query.yaw_speed`, `query.ground_speed` |
| Variables | `variable.*` — stockage persistant par entité |

## Cycle de vie des plugins

```
Démarrage serveur :
  1. Scanner le dossier plugins/
  2. Lire les manifests (plugin.toml)
  3. Résoudre les dépendances (ordre topologique)
  4. Charger dans l'ordre :
     a. Plugins Rust (compilés dans le serveur)
     b. Plugins WASM (fichiers .wasm)
     c. Scripts Lua (fichiers .lua)
     d. Behavior Packs (dossiers JSON)
  5. Appeler on_enable() pour chaque plugin

Rechargement (hot-reload) :
  1. /reload <plugin_name>
  2. Appeler on_disable() sur le plugin
  3. Désenregistrer tous les handlers/commandes
  4. Recharger le module (WASM/Lua)
  5. Appeler on_enable() sur la nouvelle version

Arrêt serveur :
  1. Appeler on_disable() pour chaque plugin (ordre inverse)
  2. Décharger les modules
  3. Sauvegarder les données
```

## Gestion des conflits

### Priorité des handlers

```
Si deux plugins écoutent le même événement :
  1. Trier par priorité (Lowest → Monitor)
  2. À priorité égale : ordre de chargement

Si un handler annule un événement :
  - Les handlers de priorité inférieure ne sont PAS exécutés
  - Les handlers Monitor sont TOUJOURS exécutés (lecture seule)
```

### Conventions de nommage

```
Commandes : /pluginname:commande (si conflit)
Permissions : pluginname.category.action
Events : pluginname:event_name (custom events)
```
