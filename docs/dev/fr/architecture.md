---
layout: default
title: Architecture
nav_order: 3
---

# Architecture du système de plugins

Cette page explique le fonctionnement interne du système de plugins MC-RS.

## Vue d'ensemble

```
┌─────────────────────────────────────────────────┐
│                  PluginManager                   │
│                                                  │
│  plugins: Vec<Box<dyn Plugin>>                   │
│  tasks: Vec<ScheduledTask>                       │
│  plugin_commands: HashMap<String, String>         │
│                                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐       │
│  │ LuaPlugin│  │WasmPlugin│  │ LuaPlugin│  ...   │
│  └──────────┘  └──────────┘  └──────────┘       │
└─────────────────────────────────────────────────┘
         │                          │
         ▼                          ▼
   ServerApiImpl              ServerApiImpl
   (par callback)             (par callback)
    │         │                │         │
    ▼         ▼                ▼         ▼
 Snapshot   Actions         Snapshot   Actions
 (lecture)  (différées)     (lecture)  (différées)
```

## Le pattern Snapshot + Actions différées

Le système de plugins utilise une **architecture basée sur des snapshots** pour éviter les conflits d'emprunt en Rust asynchrone :

1. **Avant un callback** : le serveur crée un `ServerSnapshot` — une copie en lecture seule de l'état du jeu
2. **Pendant un callback** : le plugin lit depuis le snapshot et met en file d'attente les opérations d'écriture comme des `PendingAction`
3. **Après un callback** : le serveur applique toutes les actions en attente de manière asynchrone

Cela signifie :
- **Les lectures sont instantanées** (depuis le snapshot) : `online_players()`, `get_time()`, `get_tick()`, etc.
- **Les écritures sont différées** : `send_message()`, `teleport_player()`, `set_time()`, etc. sont appliquées après le retour du callback

### ServerSnapshot

```rust
pub struct ServerSnapshot {
    pub players: Vec<PluginPlayer>,
    pub world_time: i64,
    pub current_tick: u64,
    pub is_raining: bool,
}
```

### PendingAction

```rust
pub enum PendingAction {
    SendMessage { player_name, message },
    BroadcastMessage { message },
    KickPlayer { player_name, reason },
    SetPlayerHealth { player_name, health },
    SetPlayerFood { player_name, food },
    TeleportPlayer { player_name, x, y, z },
    SetTime { time },
    SpawnMob { mob_type, x, y, z },
    RemoveMob { runtime_id },
    RegisterCommand { name, description, plugin_name },
    ScheduleTask { task },
    CancelTask { plugin_name, task_id },
    Log { level, message },
}
```

## Le trait Plugin

Chaque plugin (Lua ou WASM) implémente ce trait :

```rust
pub trait Plugin: Send {
    fn info(&self) -> PluginInfo;
    fn on_enable(&mut self, api: &mut dyn ServerApi);
    fn on_disable(&mut self) {}
    fn on_event(&mut self, event: &PluginEvent, api: &mut dyn ServerApi) -> EventResult;
    fn on_task(&mut self, task_id: u32, api: &mut dyn ServerApi);
    fn on_command(&mut self, command: &str, args: &[String], sender: &str, api: &mut dyn ServerApi) -> Option<String>;
    fn default_config(&self) -> Option<serde_json::Value>;
    fn load_config(&mut self, config: serde_json::Value);
}
```

| Méthode | Quand appelée | Objectif |
|---------|--------------|----------|
| `info()` | Au chargement | Retourne le nom, version, description, auteur |
| `on_enable()` | Démarrage du serveur | Initialisation, enregistrement de commandes |
| `on_disable()` | Arrêt du serveur | Nettoyage |
| `on_event()` | Chaque événement | Réagir aux événements, les annuler optionnellement |
| `on_task()` | Tâche planifiée déclenchée | Gérer les tâches temporisées |
| `on_command()` | Commande plugin exécutée | Gérer les commandes personnalisées |
| `default_config()` | Premier chargement | Fournir un config.json par défaut |
| `load_config()` | Au chargement si config existe | Recevoir la configuration sauvegardée |

## Flux de dispatch des événements

```
Un événement de jeu survient
       │
       ▼
build_snapshot()  ──→  ServerSnapshot
       │
       ▼
PluginManager.dispatch(event, snapshot)
       │
       ├──→ Plugin 1: on_event() → EventResult::Continue
       ├──→ Plugin 2: on_event() → EventResult::Continue
       └──→ Plugin 3: on_event() → EventResult::Cancelled  ← arrêt ici
       │
       ▼
Collecter les PendingActions de tous les plugins
       │
       ▼
apply_plugin_actions()  ──→  Exécuter les écritures de manière asynchrone
```

Pour les **événements annulables**, si un plugin retourne `EventResult::Cancelled` :
- Les plugins restants ne sont **pas** notifiés
- L'action par défaut du serveur est **empêchée** (ex : message non envoyé, bloc non cassé)

Pour les **événements non annulables** (`ServerStarted`, `PlayerJoin`, etc.), la valeur de retour est ignorée et tous les plugins sont toujours notifiés.

## Planificateur de tâches

Les plugins peuvent planifier des tâches différées ou répétitives :

```
schedule_delayed(delay_ticks, task_id)
schedule_repeating(delay_ticks, interval_ticks, task_id)
cancel_task(task_id)
```

Le `PluginManager` maintient une liste de `ScheduledTask` :

```rust
pub struct ScheduledTask {
    pub plugin_name: String,
    pub task_id: u32,
    pub remaining_ticks: u64,
    pub interval: Option<u64>,  // None = one-shot
}
```

À chaque tick serveur (50ms, 20 TPS), le planificateur :
1. Décrémente `remaining_ticks` pour toutes les tâches
2. Déclenche `on_task(task_id)` pour les tâches arrivées à 0
3. Réinitialise les tâches répétitives ; supprime les tâches one-shot

## Routage des commandes plugin

Quand un joueur exécute une commande :

1. Vérifier si c'est une **commande serveur** (`/help`, `/gamemode`, etc.) → traitement direct
2. Vérifier si c'est une **commande plugin** (enregistrée via `register_command()`) → routage vers le `on_command()` du plugin propriétaire
3. Vérifier le **CommandRegistry** (framework de commandes intégré) → traitement via le registre
4. Non trouvée → erreur "Commande inconnue"

Les commandes plugin sont enregistrées pendant `on_enable()` et stockées dans `PluginManager.plugin_commands` (un `HashMap<String, String>` associant le nom de commande au nom du plugin).
