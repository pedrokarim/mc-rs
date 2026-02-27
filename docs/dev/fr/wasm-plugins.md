---
layout: default
title: Plugins WASM
nav_order: 7
---

# Développement de plugins WASM

MC-RS supporte les plugins WebAssembly via [wasmtime](https://wasmtime.dev/). Les plugins WASM peuvent être écrits dans n'importe quel langage compilant vers `wasm32-unknown-unknown` (Rust, C, Zig, etc.).

## Manifeste (`plugin.toml`)

```toml
[plugin]
name = "MonPluginWasm"
version = "1.0.0"
author = "VotreNom"
description = "Un plugin WASM"
wasm_file = "plugin.wasm"       # Chemin vers le fichier .wasm

[limits]
fuel_per_event = 1000000        # Budget fuel par callback d'événement (défaut : 1 000 000)
fuel_per_command = 1000000      # Budget fuel par callback de commande (défaut : 1 000 000)
fuel_per_task = 500000          # Budget fuel par callback de tâche (défaut : 500 000)
fuel_on_enable = 5000000        # Budget fuel pour on_enable (défaut : 5 000 000)
max_memory_pages = 256          # Pages mémoire max, 64Ko chacune (défaut : 256 = 16Mo)
```

La présence de `wasm_file` est ce qui distingue les plugins WASM des plugins Lua.

## Exports requis

Votre module WASM **doit** exporter les éléments suivants :

| Export | Signature | Description |
|--------|-----------|-------------|
| `memory` | Memory | Mémoire linéaire exportée |
| `__malloc` | `(size: i32) -> i32` | Allouer de la mémoire guest, retourner un pointeur |
| `__free` | `(ptr: i32, size: i32)` | Libérer de la mémoire guest |
| `__plugin_info` | `() -> i32` | Retourner les infos du plugin (JSON préfixé par la longueur) |
| `__on_enable` | `()` | Appelé au chargement du plugin |
| `__on_disable` | `()` | Appelé au déchargement du plugin |
| `__on_event` | `(ptr: i32, len: i32) -> i32` | Gérer un événement (JSON), retourner 0=continuer, 1=annuler |
| `__on_task` | `(task_id: i32)` | Gérer une tâche planifiée |
| `__on_command` | `(ptr: i32, len: i32) -> i32` | Gérer une commande (JSON), retourner la réponse préfixée ou 0 |
| `__default_config` | `() -> i32` | Retourner la config par défaut (JSON préfixé ou 0) |
| `__load_config` | `(ptr: i32, len: i32)` | Recevoir la config (JSON) |

## Fonctions hôte (module `mcrs`)

Importez ces fonctions depuis le module `"mcrs"` pour interagir avec le serveur :

### API joueur

| Fonction | Signature | Description |
|----------|-----------|-------------|
| `send_message` | `(name_ptr: i32, name_len: i32, msg_ptr: i32, msg_len: i32)` | Envoyer un message à un joueur |
| `broadcast_message` | `(ptr: i32, len: i32)` | Diffuser à tous les joueurs |
| `kick_player` | `(name_ptr: i32, name_len: i32, reason_ptr: i32, reason_len: i32)` | Expulser un joueur |
| `set_player_health` | `(name_ptr: i32, name_len: i32, health: f32)` | Définir la santé |
| `set_player_food` | `(name_ptr: i32, name_len: i32, food: i32)` | Définir la faim |
| `teleport_player` | `(name_ptr: i32, name_len: i32, x: f32, y: f32, z: f32)` | Téléporter un joueur |
| `online_players` | `() -> i32` | Lister les joueurs (retourne JSON préfixé par la longueur) |
| `get_player` | `(name_ptr: i32, name_len: i32) -> i32` | Obtenir un joueur (retourne JSON préfixé ou 0) |

### API monde

| Fonction | Signature | Description |
|----------|-----------|-------------|
| `get_time` | `() -> i64` | Obtenir l'heure du monde |
| `set_time` | `(time: i64)` | Définir l'heure du monde |
| `is_raining` | `() -> i32` | Vérifier la météo (1=pluie, 0=clair) |

### API entité

| Fonction | Signature | Description |
|----------|-----------|-------------|
| `spawn_mob` | `(type_ptr: i32, type_len: i32, x: f32, y: f32, z: f32)` | Faire apparaître un mob |
| `remove_mob` | `(runtime_id: i64)` | Supprimer un mob |

### API serveur

| Fonction | Signature | Description |
|----------|-----------|-------------|
| `get_tick` | `() -> i64` | Obtenir le tick serveur actuel |
| `log` | `(level: i32, ptr: i32, len: i32)` | Logger un message (0=info, 1=warn, 2=error, 3=debug) |

### Planificateur

| Fonction | Signature | Description |
|----------|-----------|-------------|
| `schedule_delayed` | `(delay_ticks: i64, task_id: i32)` | Planifier une tâche one-shot |
| `schedule_repeating` | `(delay_ticks: i64, interval_ticks: i64, task_id: i32)` | Planifier une tâche répétitive |
| `cancel_task` | `(task_id: i32)` | Annuler une tâche |

### Commandes

| Fonction | Signature | Description |
|----------|-----------|-------------|
| `register_command` | `(name_ptr: i32, name_len: i32, desc_ptr: i32, desc_len: i32)` | Enregistrer une commande |

## Protocole mémoire

Toutes les chaînes sont passées entre l'hôte et le guest comme des octets UTF-8 bruts via des paires `(pointeur, longueur)`.

### Lecture de chaînes (hôte → guest)

Quand l'hôte appelle votre fonction exportée avec `(ptr, len)` :
1. Lisez `len` octets à partir de `ptr` dans votre mémoire linéaire
2. Décodez en UTF-8

### Écriture de chaînes (guest → hôte)

Pour appeler les fonctions hôte, passez un pointeur vers vos données de chaîne et leur longueur. La chaîne doit être dans votre `memory` exportée.

### Retours préfixés par la longueur

Les fonctions comme `online_players()` et `get_player()` retournent un pointeur vers une **chaîne préfixée par la longueur** :

```
Offset 0 : u32 (little-endian) — longueur en octets des données UTF-8
Offset 4 : octets UTF-8
```

Si la fonction retourne `0`, cela signifie "non trouvé" ou "vide".

## Format JSON des événements

Les événements sont passés à `__on_event` en JSON utilisant le format enum tagué de serde :

```json
{
  "PlayerJoin": {
    "player": {
      "name": "Steve",
      "uuid": "00000000-0000-0000-0000-000000000001",
      "runtime_id": 1,
      "position": [0.5, 65.62, 0.5],
      "gamemode": 0,
      "health": 20.0
    }
  }
}
```

```json
{
  "BlockBreak": {
    "player": { ... },
    "position": { "x": 10, "y": 64, "z": -5 },
    "block_id": 3456789
  }
}
```

```json
"ServerStarted"
```

## Format JSON des commandes

Les commandes sont passées à `__on_command` comme :

```json
{
  "command": "hello",
  "args": ["arg1", "arg2"],
  "sender": "Steve"
}
```

Retournez une chaîne préfixée par la longueur avec le message de réponse, ou `0` pour aucune réponse.

## Fuel metering

Chaque callback a un **budget fuel** (configuré dans `plugin.toml`). Chaque instruction WASM consomme du fuel. Si un callback épuise son fuel, l'exécution est interrompue et une erreur est loguée.

Budgets par défaut :
- `on_enable` : 5 000 000
- Événements : 1 000 000
- Commandes : 1 000 000
- Tâches : 500 000

## Exemple minimal (WAT)

Voici le plugin WASM minimum absolu (en format texte WAT) :

```wat
(module
    (memory (export "memory") 1)
    (global $heap_ptr (mut i32) (i32.const 1024))

    ;; Allocateur bump simple
    (func (export "__malloc") (param $size i32) (result i32)
        (local $ptr i32)
        (local.set $ptr (global.get $heap_ptr))
        (global.set $heap_ptr (i32.add (global.get $heap_ptr) (local.get $size)))
        (local.get $ptr)
    )
    (func (export "__free") (param $ptr i32) (param $size i32))
    (func (export "__plugin_info") (result i32) (i32.const 0))
    (func (export "__on_enable"))
    (func (export "__on_disable"))
    (func (export "__on_event") (param $ptr i32) (param $len i32) (result i32)
        (i32.const 0)  ;; Toujours continuer
    )
    (func (export "__on_task") (param $task_id i32))
    (func (export "__on_command") (param $ptr i32) (param $len i32) (result i32)
        (i32.const 0)  ;; Pas de réponse
    )
    (func (export "__default_config") (result i32) (i32.const 0))
    (func (export "__load_config") (param $ptr i32) (param $len i32))
)
```

## Développer en Rust

Pour les plugins WASM en Rust, ciblez `wasm32-unknown-unknown` :

### `Cargo.toml`

```toml
[package]
name = "mon-plugin-mcrs"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]
```

### `src/lib.rs`

```rust
// Import des fonctions hôte depuis le module "mcrs"
extern "C" {
    #[link_name = "broadcast_message"]
    fn host_broadcast(ptr: *const u8, len: i32);
    #[link_name = "log"]
    fn host_log(level: i32, ptr: *const u8, len: i32);
}

// Allocateur bump simple
static mut HEAP: [u8; 65536] = [0; 65536];
static mut HEAP_POS: usize = 0;

#[no_mangle]
pub extern "C" fn __malloc(size: i32) -> i32 {
    unsafe {
        let ptr = HEAP.as_ptr().add(HEAP_POS) as i32;
        HEAP_POS += size as usize;
        ptr
    }
}

#[no_mangle]
pub extern "C" fn __free(_ptr: i32, _size: i32) {}

#[no_mangle]
pub extern "C" fn __on_enable() {
    let msg = b"Plugin WASM actif !";
    unsafe { host_log(0, msg.as_ptr() as i32, msg.len() as i32); }
}

#[no_mangle]
pub extern "C" fn __on_disable() {}

#[no_mangle]
pub extern "C" fn __on_event(_ptr: i32, _len: i32) -> i32 { 0 }

#[no_mangle]
pub extern "C" fn __on_task(_task_id: i32) {}

#[no_mangle]
pub extern "C" fn __on_command(_ptr: i32, _len: i32) -> i32 { 0 }

#[no_mangle]
pub extern "C" fn __plugin_info() -> i32 { 0 }

#[no_mangle]
pub extern "C" fn __default_config() -> i32 { 0 }

#[no_mangle]
pub extern "C" fn __load_config(_ptr: i32, _len: i32) {}
```

### Compilation et déploiement

```bash
cd mon-plugin-mcrs
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/mon_plugin_mcrs.wasm ../plugins/mon-plugin/plugin.wasm
```
