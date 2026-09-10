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

### AvailableCommandsPacket (0x4C) — Format binaire protocol 924

Le serveur envoie la liste complète des commandes au client pour l'autocomplétion.
Le paquet contient 8 sections sérialisées dans cet ordre :

```
1. enumValues: VarUInt count + String[]
   → Pool global de strings pour les hard enums (ex: "survival", "creative", "day", "rain"...)

2. chainedSubCommandValues: VarUInt count + String[]
   → Pool de noms de sous-commandes chaînées (rarement utilisé)

3. postfixes: VarUInt count + String[]
   → Pool de suffixes (ex: "L" pour xp levels)

4. enums (hard enums): VarUInt count + CommandEnumRawData[]
   → Enums fixes, indices dans enumValues[]
   → Format par enum :
     - String enumName
     - VarUInt valueCount
     - UInt[] valueIndices (indices dans enumValues[])

5. chainedSubCommandData: VarUInt count + ChainedSubCommandRawData[]

6. commandData: VarUInt count + CommandRawData[]
   → Les commandes elles-mêmes (voir ci-dessous)

7. softEnums: VarUInt count + CommandSoftEnum[]
   → Enums dynamiques (mises à jour en temps réel, ex: noms de joueurs)
   → Format par soft enum :
     - String name
     - VarUInt valueCount
     - String[] values (strings directes, pas d'indices)

8. enumConstraints: VarUInt count + CommandEnumConstraintRawData[]
   → Contraintes sur les valeurs d'enums hard
```

#### CommandRawData (par commande)

```
String name                          // nom en lowercase
String description                   // description affichée dans /help
UShort flags                         // généralement 0
String permission                    // "any", "gamedirectors", "admin", "host", "owner", "internal"
SignedInt aliasEnumIndex              // index dans enums[] ou -1 si pas d'alias
VarUInt chainedSubCommandIndexCount  // généralement 0
  UInt[] indices
VarUInt overloadCount                // nombre de syntaxes
  CommandOverloadRawData[] overloads
```

#### CommandOverloadRawData (par syntaxe)

```
Bool chaining                        // false en général
VarUInt parameterCount
  CommandParameterRawData[] parameters
```

#### CommandParameterRawData (par paramètre)

```
String paramName                     // ex: "player", "gamemode", "x"
UInt typeInfo                        // type + flags (voir encodage ci-dessous)
Bool isOptional                      // paramètre optionnel ?
Byte flags                           // 0x1=FORCE_COLLAPSE_ENUM, 0x2=HAS_ENUM_CONSTRAINT
```

#### Encodage du typeInfo

Le champ `typeInfo` encode le type du paramètre avec des flags :

```
Type basique :     ARG_FLAG_VALID (0x100000) | paramType
Hard enum :        ARG_FLAG_ENUM (0x200000) | ARG_FLAG_VALID (0x100000) | enumIndex
Soft enum :        ARG_FLAG_SOFT_ENUM (0x4000000) | ARG_FLAG_VALID (0x100000) | softEnumIndex
Postfix :          ARG_FLAG_POSTFIX (0x1000000) | postfixIndex
```

#### Types de paramètres (CommandParameterTypes)

```
INT = 1                    SELECTION = 8 (@a, @p, @s, @e)
VAL = 3 (float)            WILDCARDSELECTION = 10
RVAL = 4                   FULLINTEGERRANGE = 23
WILDCARDINT = 5            ID = 56 (string)
OPERATOR = 6               POSITION = 64 (x y z int)
COMPAREOPERATOR = 7        POSITION_FLOAT = 65 (x y z float)
                           MESSAGE_ROOT = 68 (message)
                           RAWTEXT = 70 (texte libre)
                           JSON_OBJECT = 74
                           BLOCK_STATE_ARRAY = 84
```

#### Hard Enums vs Soft Enums

**Hard Enums** — fixes, envoyées une seule fois au login :
- Gamemodes : `["survival", "creative", "adventure", "spectator", "s", "c", "a", "sp", "0", "1", "2", "3"]`
- Difficultés : `["peaceful", "easy", "normal", "hard", "p", "e", "n", "h", "0", "1", "2", "3"]`
- Weather : `["clear", "rain", "thunder"]`
- Time presets : `["day", "noon", "sunset", "night", "midnight", "sunrise"]`
- Boolean : `["true", "false"]`

**Soft Enums** — dynamiques, mises à jour via UpdateSoftEnumPacket :
- Noms de joueurs connectés (pour /tp, /tell, /kick, etc.)
- Noms de scoreboard objectives
- Tags d'entités

#### Aliases

PMMP crée une hard enum nommée `"{CommandName}Aliases"` contenant :
- Le nom de la commande lui-même (workaround bug client)
- Tous les alias (ex: pour `teleport` → `["teleport", "tp"]`)
L'index de cette enum est passé dans `aliasEnumIndex`.

#### Permission levels (CommandPermissions)

```
NORMAL = 0       → "any"           (tous les joueurs)
OPERATOR = 1     → "gamedirectors" (opérateurs)
AUTOMATION = 2   → "admin"         (command blocks)
HOST = 3         → "host"          (hôte LAN)
OWNER = 4        → "owner"         (console serveur)
INTERNAL = 5     → "internal"      (interne)
```

#### Comportement par défaut PMMP

Par défaut, PMMP envoie chaque commande avec un seul overload :
```
1 overload → 1 paramètre "args" de type RAWTEXT (70), optional=true
```
C'est ce que mc-rs fait actuellement. Le parsing réel se fait dans le handler, pas au niveau protocole.
Pour avoir de l'autocomplétion riche, il faut envoyer les vrais overloads avec les bons types et enums.

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
