# 11 - Plugin System

## PocketMine : Système de plugins

### Architecture

```
PluginManager
├── loadPlugins(path)           → charge tous les plugins du dossier
├── enablePlugin(plugin)        → active un plugin
├── disablePlugin(plugin)       → désactive un plugin
└── registerEvents(listener)    → enregistre les listeners
```

### Cycle de vie

```
Discover → Load → Enable (STARTUP) → Enable (POSTWORLD) → Running → Disable
```

1. **Discover** : Scanner le dossier `plugins/` pour des `.phar` ou dossiers
2. **Load** : Lire `plugin.yml`, valider API version, résoudre dépendances
3. **Enable STARTUP** : Activer les plugins marqués `load: STARTUP`
4. **Enable POSTWORLD** : Activer les plugins marqués `load: POSTWORLD` (défaut)
5. **Running** : Le plugin est actif
6. **Disable** : À l'arrêt du serveur ou manuellement

### plugin.yml

```yaml
name: MyPlugin
version: 1.0.0
main: MyNamespace\MyPlugin
api: 5.0.0
mcpe-protocol: 924

# Optionnel
description: "Description du plugin"
author: "Auteur"
authors: ["Auteur1", "Auteur2"]
website: "https://example.com"
load: POSTWORLD              # STARTUP ou POSTWORLD
prefix: "MyPlugin"           # Préfixe logger

# Dépendances
depend: ["OtherPlugin"]      # Required
softdepend: ["OptionalDep"]  # Optional
loadbefore: ["LatePlugin"]   # Charger avant

# Extensions PHP requises
extensions:
  curl: []
  yaml: ">=2.0.0"

# Commandes
commands:
  mycommand:
    description: "Ma commande"
    usage: "/mycommand <args>"
    aliases: ["mc", "mycmd"]
    permission: "myplugin.command.mycommand"
    permission-message: "Pas la permission !"

# Permissions
permissions:
  myplugin.command.mycommand:
    description: "Utiliser /mycommand"
    default: op
  myplugin.admin:
    description: "Admin du plugin"
    default: op
    children:
      myplugin.command.mycommand: true
```

### PluginBase (classe abstraite)

```php
abstract class PluginBase implements Plugin, CommandExecutor {
    // Lifecycle hooks
    onLoad() → void       // Appelé au chargement (avant enable)
    onEnable() → void     // Appelé à l'activation
    onDisable() → void    // Appelé à la désactivation

    // API
    getServer() → Server
    getName() → string
    getDataFolder() → string        // Dossier de données du plugin
    getConfig() → Config            // config.yml du plugin
    saveConfig() → void
    saveDefaultConfig() → void
    getResource(filename) → stream  // Fichier dans le .phar
    saveResource(filename) → void   // Extraire fichier du .phar
    getLogger() → Logger
    getScheduler() → TaskScheduler  // Scheduler propre au plugin

    // Commands
    onCommand(sender, command, label, args[]) → bool
}
```

### Chargement de plugins

**PharPluginLoader :**
- Charge depuis des fichiers `.phar` (archives PHP)
- Extrait `plugin.yml` du phar
- Crée un class loader pour les classes du phar

**ScriptPluginLoader :**
- Charge depuis un dossier avec `plugin.yml` + `src/`
- Dossier classique pour le développement

### Résolution de dépendances

L'ordre de chargement respecte :
1. `depend` : chargé après ces plugins (erreur si manquant)
2. `softdepend` : chargé après si présent (ignoré si absent)
3. `loadbefore` : ces plugins sont chargés après nous

### Fichiers PocketMine de référence

```
src/plugin/Plugin.php
src/plugin/PluginBase.php
src/plugin/PluginManager.php
src/plugin/PluginLoader.php
src/plugin/PharPluginLoader.php
src/plugin/ScriptPluginLoader.php
src/plugin/PluginDescription.php
src/plugin/PluginLogger.php
```

---

## Équivalent Rust

### Approche : Plugins Lua + WASM

En Rust, on ne peut pas charger du code natif dynamiquement de manière sûre.
Deux approches pour les plugins :

1. **Lua** (via `mlua`) : Simple, sûr, performant pour la logique de jeu
2. **WASM** (via `wasmtime`) : Plus performant, sandboxé, multi-langage

### Crate : `mc-rs-plugin-api`

```rust
/// Identifiant de plugin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PluginId(pub u32);

/// Métadonnées du plugin
#[derive(Debug, Clone)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub authors: Vec<String>,
    pub main: String,
    pub load_order: LoadOrder,
    pub dependencies: Vec<Dependency>,
    pub commands: Vec<CommandDefinition>,
    pub permissions: Vec<PermissionDefinition>,
}

#[derive(Debug, Clone, Copy)]
pub enum LoadOrder {
    Startup,
    PostWorld,
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: String,
    pub required: bool,  // true=depend, false=softdepend
}

/// Trait pour les plugins natifs (tests, core)
pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &PluginManifest;
    fn on_load(&mut self, api: &PluginApi) {}
    fn on_enable(&mut self, api: &PluginApi) {}
    fn on_disable(&mut self, api: &PluginApi) {}
}

/// API exposée aux plugins
pub struct PluginApi {
    pub events: Arc<RwLock<EventManager>>,
    pub commands: Arc<RwLock<CommandMap>>,
    pub server: Arc<Server>,
}

impl PluginApi {
    pub fn register_listener<E: Event>(&self, handler: impl Fn(&mut E) + Send + Sync + 'static);
    pub fn register_command(&self, name: &str, handler: impl CommandHandler);
    pub fn schedule_task(&self, delay: u64, task: impl FnOnce() + Send + 'static);
    pub fn schedule_repeating(&self, period: u64, task: impl Fn() + Send + 'static) -> TaskHandle;
    pub fn get_data_folder(&self) -> PathBuf;
    pub fn get_config(&self) -> Config;
}
```

### Crate : `mc-rs-plugin-lua`

```rust
/// Plugin Lua
pub struct LuaPluginLoader {
    lua: Lua,
    plugins: HashMap<PluginId, LuaPlugin>,
}

pub struct LuaPlugin {
    id: PluginId,
    manifest: PluginManifest,
    // Lua state pour ce plugin
}

impl LuaPluginLoader {
    pub fn load_plugin(&mut self, path: &Path) -> Result<PluginId> {
        // Lire plugin.yml
        // Charger le script main.lua
        // Enregistrer les fonctions API dans Lua
        todo!()
    }
}
```

**Exemple plugin Lua :**
```lua
-- plugin.yml dans le même dossier

function on_enable()
    log("MyPlugin enabled!")
    register_command("hello", on_hello_command)
    register_event("PlayerJoinEvent", on_player_join)
end

function on_disable()
    log("MyPlugin disabled!")
end

function on_hello_command(sender, args)
    send_message(sender, "Hello from Lua!")
    return true
end

function on_player_join(event)
    local player = event.player_name
    broadcast(player .. " joined the server!")
end
```

### PluginManager

```rust
pub struct PluginManager {
    plugins: HashMap<PluginId, PluginEntry>,
    lua_loader: LuaPluginLoader,
    wasm_loader: Option<WasmPluginLoader>,
    load_order: Vec<PluginId>,
    next_id: u32,
}

struct PluginEntry {
    id: PluginId,
    manifest: PluginManifest,
    enabled: bool,
    loader: PluginLoaderType,
}

impl PluginManager {
    pub fn load_plugins(&mut self, plugins_dir: &Path) -> Result<()> {
        // Scanner le dossier
        // Charger chaque plugin.yml
        // Résoudre les dépendances (tri topologique)
        // Charger dans l'ordre
        todo!()
    }

    pub fn enable_all(&mut self, order: LoadOrder) {
        for id in &self.load_order {
            if self.plugins[id].manifest.load_order == order {
                self.enable_plugin(*id);
            }
        }
    }

    pub fn disable_all(&mut self) {
        for id in self.load_order.iter().rev() {
            self.disable_plugin(*id);
        }
    }
}
```
