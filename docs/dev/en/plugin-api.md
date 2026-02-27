---
layout: default
title: ServerApi Reference
nav_order: 4
---

# ServerApi Reference

The `ServerApi` trait provides all functions available to plugins during callbacks. Read operations return data immediately from a snapshot; write operations are deferred and applied after the callback returns.

## Player Operations

### Read

| Function | Signature | Description |
|----------|-----------|-------------|
| `online_players` | `() -> Vec<PluginPlayer>` | Returns all connected players |
| `get_player` | `(name: &str) -> Option<PluginPlayer>` | Find a player by name |

### Write (Deferred)

| Function | Signature | Description |
|----------|-----------|-------------|
| `send_message` | `(player_name: &str, message: &str)` | Send a chat message to a specific player |
| `broadcast_message` | `(message: &str)` | Send a message to all online players |
| `kick_player` | `(player_name: &str, reason: &str)` | Disconnect a player with a reason |
| `set_player_health` | `(player_name: &str, health: f32)` | Set health (0.0 – 20.0) |
| `set_player_food` | `(player_name: &str, food: i32)` | Set hunger level (0 – 20) |
| `teleport_player` | `(player_name: &str, x: f32, y: f32, z: f32)` | Teleport a player to coordinates |

## World Operations

| Function | Signature | Description |
|----------|-----------|-------------|
| `get_time` | `() -> i64` | Get world time (0–24000 day cycle) |
| `set_time` | `(time: i64)` | Set world time |
| `is_raining` | `() -> bool` | Check current weather state |

## Entity Operations

| Function | Signature | Description |
|----------|-----------|-------------|
| `spawn_mob` | `(mob_type: &str, x: f32, y: f32, z: f32)` | Spawn a mob at coordinates |
| `remove_mob` | `(runtime_id: u64)` | Remove a mob by its runtime ID |

Available mob types: `zombie`, `skeleton`, `cow`, `pig`, `chicken`

## Server Operations

| Function | Signature | Description |
|----------|-----------|-------------|
| `get_tick` | `() -> u64` | Get the current server tick (20 ticks/second) |
| `log` | `(level: LogLevel, message: &str)` | Log a message at the specified level |

## Scheduler

| Function | Signature | Description |
|----------|-----------|-------------|
| `schedule_delayed` | `(plugin_name: &str, delay_ticks: u64, task_id: u32)` | Schedule a one-shot task |
| `schedule_repeating` | `(plugin_name: &str, delay_ticks: u64, interval_ticks: u64, task_id: u32)` | Schedule a repeating task |
| `cancel_task` | `(plugin_name: &str, task_id: u32)` | Cancel a scheduled task |

**Note:** 20 ticks = 1 second. A `delay_ticks` of 100 means 5 seconds.

## Commands

| Function | Signature | Description |
|----------|-----------|-------------|
| `register_command` | `(name: &str, description: &str, plugin_name: &str)` | Register a custom command |

Registered commands appear in the server's command list and are routed to the plugin's `on_command()` callback.

---

## Types

### PluginPlayer

```rust
pub struct PluginPlayer {
    pub name: String,          // Player display name
    pub uuid: String,          // Player UUID
    pub runtime_id: u64,       // Entity runtime ID
    pub position: (f32, f32, f32), // (x, y, z) coordinates
    pub gamemode: i32,         // 0=survival, 1=creative, 2=adventure, 3=spectator
    pub health: f32,           // 0.0 to 20.0
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
    Attack,       // Melee or mob attack
    Fall,         // Fall damage
    Drowning,     // Underwater too long
    Lava,         // Lava contact
    Fire,         // Fire/burning
    Suffocation,  // Inside a block
    Starvation,   // No food
    Void,         // Below the world
    Other,        // Anything else
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
    Continue,   // Allow the event to proceed
    Cancelled,  // Cancel the event (only for cancellable events)
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
