# 12 - Command System

## PocketMine : Système de commandes

### Architecture

```
SimpleCommandMap
├── commands: HashMap<String, Command>  (nom → commande)
├── dispatch(sender, commandLine)       → parse et exécute
└── register(command)                   → enregistre une commande

Command (abstract)
├── name, aliases, description, usage
├── permission(s)
├── execute(sender, label, args[]) → bool
└── testPermission(sender) → bool

CommandSender (interface)
├── Player (joueur connecté)
├── ConsoleCommandSender (console)
└── [Custom senders]
```

### Dispatch d'une commande

```
1. Joueur tape "/gamemode creative"
2. CommandMap.dispatch(player, "gamemode creative")
3. Parse : command="gamemode", args=["creative"]
4. Lookup : commands["gamemode"] → GamemodeCommand
5. Permission check : player.hasPermission("minecraft.command.gamemode")
6. Execute : command.execute(player, "gamemode", ["creative"])
7. Résultat envoyé au joueur
```

### Commandes par défaut (~40)

**Administration :**
- `ban`, `banip`, `banlist`, `pardon`, `pardon-ip`
- `op`, `deop`
- `kick`, `transfer`
- `whitelist`
- `stop`

**Gameplay :**
- `gamemode` (survival, creative, adventure, spectator)
- `give` (donner des items)
- `effect` (effets de potion)
- `enchant`
- `kill`
- `teleport` / `tp`
- `xp`
- `clear` (vider inventaire)
- `spawnpoint`, `setworldspawn`

**Monde :**
- `time` (set, add, query)
- `difficulty`
- `defaultgamemode`
- `save-all`, `save-off`, `save-on`
- `seed`

**Information :**
- `help` / `?`
- `list` (joueurs en ligne)
- `version` / `ver`
- `plugins` / `pl`
- `status`

**Communication :**
- `say` (broadcast)
- `tell` / `msg` / `w` (message privé)
- `me` (action)
- `title`

**Debug :**
- `dumpmemory`
- `gc` (garbage collector)
- `timings`
- `particle`

### AvailableCommandsPacket

Le serveur envoie la liste des commandes au client pour l'autocompletion :

```
Pour chaque commande :
  - name
  - description
  - flags
  - permission_level (0=normal, 1=op, 2=hidden, 3=admin)
  - aliases[]
  - overloads[] (combinaisons de paramètres)
    - parameters[]
      - name
      - type (int, float, string, target, position, ...)
      - optional
      - options (enum values pour autocomplete)
```

### Fichiers PocketMine de référence

```
src/command/Command.php
src/command/SimpleCommandMap.php
src/command/CommandSender.php
src/command/PluginCommand.php
src/command/ClosureCommand.php
src/command/defaults/*.php        → ~40 commandes
```

---

## Équivalent Rust

### Crate : `mc-rs-command`

```rust
/// Expéditeur de commande
pub trait CommandSender: Send + Sync {
    fn name(&self) -> &str;
    fn send_message(&self, message: &str);
    fn has_permission(&self, permission: &str) -> bool;
    fn is_op(&self) -> bool;
}

/// Commande
pub trait CommandHandler: Send + Sync {
    fn execute(&self, sender: &dyn CommandSender, label: &str, args: &[&str]) -> Result<bool>;
}

/// Définition de commande
pub struct CommandDefinition {
    pub name: String,
    pub description: String,
    pub usage: String,
    pub aliases: Vec<String>,
    pub permissions: Vec<String>,
    pub overloads: Vec<CommandOverload>,
}

pub struct CommandOverload {
    pub parameters: Vec<CommandParameter>,
}

pub struct CommandParameter {
    pub name: String,
    pub param_type: CommandParamType,
    pub optional: bool,
    pub options: Vec<String>,  // enum values pour autocomplete
}

pub enum CommandParamType {
    Int,
    Float,
    String,
    Target,     // @a, @p, @r, @s, @e
    Position,   // x y z
    BlockPos,
    Message,    // Rest of the line
    Json,
    Command,
    Enum(String),
}

/// Registre de commandes
pub struct CommandMap {
    commands: HashMap<String, CommandEntry>,
}

struct CommandEntry {
    definition: CommandDefinition,
    handler: Box<dyn CommandHandler>,
    plugin_id: Option<PluginId>,
}

impl CommandMap {
    pub fn register(&mut self, def: CommandDefinition, handler: impl CommandHandler + 'static) {
        let name = def.name.clone();
        let aliases = def.aliases.clone();
        self.commands.insert(name.clone(), CommandEntry {
            definition: def,
            handler: Box::new(handler),
            plugin_id: None,
        });
        // Aussi enregistrer les aliases
        for alias in aliases {
            self.commands.insert(alias, /* reference to same entry */);
        }
    }

    pub fn dispatch(&self, sender: &dyn CommandSender, command_line: &str) -> Result<bool> {
        let parts: Vec<&str> = command_line.splitn(2, ' ').collect();
        let label = parts[0];
        let args: Vec<&str> = if parts.len() > 1 {
            parts[1].split_whitespace().collect()
        } else {
            vec![]
        };

        let entry = self.commands.get(label)
            .ok_or(CommandError::NotFound(label.to_string()))?;

        // Permission check
        for perm in &entry.definition.permissions {
            if !sender.has_permission(perm) {
                sender.send_message("You don't have permission to use this command.");
                return Ok(false);
            }
        }

        entry.handler.execute(sender, label, &args)
    }

    /// Générer le AvailableCommandsPacket pour un joueur
    pub fn build_available_commands(&self, sender: &dyn CommandSender) -> Vec<CommandDefinition> {
        self.commands.values()
            .filter(|e| e.definition.permissions.iter().all(|p| sender.has_permission(p)))
            .map(|e| e.definition.clone())
            .collect()
    }
}

/// Macro pour créer une commande simplement
macro_rules! command {
    ($name:expr, $desc:expr, |$sender:ident, $args:ident| $body:block) => {
        {
            struct Cmd;
            impl CommandHandler for Cmd {
                fn execute(&self, $sender: &dyn CommandSender, _label: &str, $args: &[&str]) -> Result<bool> {
                    $body
                }
            }
            (CommandDefinition {
                name: $name.into(),
                description: $desc.into(),
                usage: format!("/{}", $name),
                aliases: vec![],
                permissions: vec![],
                overloads: vec![],
            }, Cmd)
        }
    };
}
```
