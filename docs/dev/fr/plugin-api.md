---
layout: default
title: Référence ServerApi
nav_order: 4
---

# Référence ServerApi

Le trait `ServerApi` fournit toutes les fonctions disponibles pour les plugins pendant les callbacks. Les opérations de lecture retournent des données instantanément depuis un snapshot ; les opérations d'écriture sont différées et appliquées après le retour du callback.

## Opérations joueur

### Lecture

| Fonction | Signature | Description |
|----------|-----------|-------------|
| `online_players` | `() -> Vec<PluginPlayer>` | Retourne tous les joueurs connectés |
| `get_player` | `(name: &str) -> Option<PluginPlayer>` | Trouve un joueur par son nom |

### Écriture (différée)

| Fonction | Signature | Description |
|----------|-----------|-------------|
| `send_message` | `(player_name: &str, message: &str)` | Envoie un message à un joueur |
| `broadcast_message` | `(message: &str)` | Envoie un message à tous les joueurs |
| `kick_player` | `(player_name: &str, reason: &str)` | Déconnecte un joueur avec une raison |
| `set_player_health` | `(player_name: &str, health: f32)` | Définit la santé (0.0 – 20.0) |
| `set_player_food` | `(player_name: &str, food: i32)` | Définit le niveau de faim (0 – 20) |
| `teleport_player` | `(player_name: &str, x: f32, y: f32, z: f32)` | Téléporte un joueur |

## Opérations monde

| Fonction | Signature | Description |
|----------|-----------|-------------|
| `get_time` | `() -> i64` | Récupère l'heure du monde (cycle jour 0–24000) |
| `set_time` | `(time: i64)` | Définit l'heure du monde |
| `is_raining` | `() -> bool` | Vérifie l'état de la météo |

## Opérations entités

| Fonction | Signature | Description |
|----------|-----------|-------------|
| `spawn_mob` | `(mob_type: &str, x: f32, y: f32, z: f32)` | Fait apparaître un mob aux coordonnées |
| `remove_mob` | `(runtime_id: u64)` | Supprime un mob par son runtime ID |

Types de mobs disponibles : `zombie`, `skeleton`, `cow`, `pig`, `chicken`

## Opérations serveur

| Fonction | Signature | Description |
|----------|-----------|-------------|
| `get_tick` | `() -> u64` | Récupère le tick serveur actuel (20 ticks/seconde) |
| `log` | `(level: LogLevel, message: &str)` | Écrit un message de log au niveau spécifié |

## Planificateur

| Fonction | Signature | Description |
|----------|-----------|-------------|
| `schedule_delayed` | `(plugin_name: &str, delay_ticks: u64, task_id: u32)` | Planifie une tâche one-shot |
| `schedule_repeating` | `(plugin_name: &str, delay_ticks: u64, interval_ticks: u64, task_id: u32)` | Planifie une tâche répétitive |
| `cancel_task` | `(plugin_name: &str, task_id: u32)` | Annule une tâche planifiée |

**Note :** 20 ticks = 1 seconde. Un `delay_ticks` de 100 correspond à 5 secondes.

## Commandes

| Fonction | Signature | Description |
|----------|-----------|-------------|
| `register_command` | `(name: &str, description: &str, plugin_name: &str)` | Enregistre une commande personnalisée |

Les commandes enregistrées apparaissent dans la liste des commandes du serveur et sont routées vers le callback `on_command()` du plugin.

---

## Types

### PluginPlayer

```rust
pub struct PluginPlayer {
    pub name: String,          // Nom d'affichage du joueur
    pub uuid: String,          // UUID du joueur
    pub runtime_id: u64,       // ID runtime de l'entité
    pub position: (f32, f32, f32), // Coordonnées (x, y, z)
    pub gamemode: i32,         // 0=survie, 1=créatif, 2=aventure, 3=spectateur
    pub health: f32,           // 0.0 à 20.0
}
```

### PluginBlockPos

```rust
pub struct PluginBlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}
```

### DamageCause

```rust
pub enum DamageCause {
    Attack,       // Attaque au corps à corps ou par un mob
    Fall,         // Dégâts de chute
    Drowning,     // Noyade
    Lava,         // Contact avec la lave
    Fire,         // Feu/brûlure
    Suffocation,  // Coincé dans un bloc
    Starvation,   // Plus de nourriture
    Void,         // Sous le monde
    Other,        // Autre cause
}
```

### LogLevel

```rust
pub enum LogLevel {
    Info,
    Warn,
    Error,
    Debug,
}
```

### EventResult

```rust
pub enum EventResult {
    Continue,   // Laisser l'événement se poursuivre
    Cancelled,  // Annuler l'événement (uniquement pour les événements annulables)
}
```

### PluginInfo

```rust
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
}
```
