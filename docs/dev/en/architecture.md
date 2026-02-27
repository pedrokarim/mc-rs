---
layout: default
title: Architecture
nav_order: 3
---

# Plugin System Architecture

This page explains how the MC-RS plugin system works internally.

## Overview

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
   (per-callback)             (per-callback)
    │         │                │         │
    ▼         ▼                ▼         ▼
 Snapshot   Actions         Snapshot   Actions
  (read)    (deferred)       (read)    (deferred)
```

## The Snapshot + Deferred Action Pattern

The plugin system uses a **snapshot-based architecture** to avoid borrowing conflicts in async Rust:

1. **Before a callback**: The server creates a `ServerSnapshot` — a read-only copy of the current game state
2. **During a callback**: The plugin reads from the snapshot and queues write operations as `PendingAction`s
3. **After a callback**: The server applies all pending actions asynchronously

This pattern means:
- **Reads are instant** (from the snapshot): `online_players()`, `get_time()`, `get_tick()`, etc.
- **Writes are deferred**: `send_message()`, `teleport_player()`, `set_time()`, etc. are applied after the callback returns

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

## The Plugin Trait

Every plugin (Lua or WASM) implements this trait:

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

| Method | When Called | Purpose |
|--------|-----------|---------|
| `info()` | On load | Returns plugin name, version, description, author |
| `on_enable()` | Server startup | Initialize, register commands, schedule tasks |
| `on_disable()` | Server shutdown | Cleanup |
| `on_event()` | Every game event | React to events, optionally cancel them |
| `on_task()` | Scheduled task fires | Handle timed tasks |
| `on_command()` | Plugin command executed | Handle custom commands |
| `default_config()` | On first load | Provide default config.json |
| `load_config()` | On load if config exists | Receive saved configuration |

## Event Dispatch Flow

```
Game Event Occurs
       │
       ▼
build_snapshot()  ──→  ServerSnapshot
       │
       ▼
PluginManager.dispatch(event, snapshot)
       │
       ├──→ Plugin 1: on_event() → EventResult::Continue
       ├──→ Plugin 2: on_event() → EventResult::Continue
       └──→ Plugin 3: on_event() → EventResult::Cancelled  ← stops here
       │
       ▼
Collect PendingActions from all plugins
       │
       ▼
apply_plugin_actions()  ──→  Execute writes asynchronously
```

For **cancellable events**, if any plugin returns `EventResult::Cancelled`:
- Remaining plugins are **not** notified
- The default server action is **prevented** (e.g., chat message not sent, block not broken)

For **non-cancellable events** (like `ServerStarted`, `PlayerJoin`), the return value is ignored and all plugins are always notified.

## Task Scheduler

Plugins can schedule delayed or repeating tasks:

```
schedule_delayed(delay_ticks, task_id)
schedule_repeating(delay_ticks, interval_ticks, task_id)
cancel_task(task_id)
```

The `PluginManager` maintains a list of `ScheduledTask`s:

```rust
pub struct ScheduledTask {
    pub plugin_name: String,
    pub task_id: u32,
    pub remaining_ticks: u64,
    pub interval: Option<u64>,  // None = one-shot
}
```

Every server tick (50ms, 20 TPS), the scheduler:
1. Decrements `remaining_ticks` for all tasks
2. Fires `on_task(task_id)` for tasks that reach 0
3. Resets repeating tasks; removes one-shot tasks

## Plugin Command Routing

When a player executes a command:

1. Check if it's a **server command** (`/help`, `/gamemode`, etc.) → handle directly
2. Check if it's a **plugin command** (registered via `register_command()`) → route to the owning plugin's `on_command()`
3. Check the **CommandRegistry** (built-in command framework) → handle via registry
4. Not found → "Unknown command" error

Plugin commands are registered during `on_enable()` and stored in `PluginManager.plugin_commands` (a `HashMap<String, String>` mapping command name to plugin name).
