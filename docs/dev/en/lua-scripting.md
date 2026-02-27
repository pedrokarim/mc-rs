---
layout: default
title: Lua Scripting
nav_order: 6
---

# Lua Scripting Guide

MC-RS embeds **Lua 5.4** for plugin scripting. Lua plugins are simple, lightweight, and require no compilation.

## Manifest (`plugin.toml`)

```toml
[plugin]
name = "MyPlugin"
version = "1.0.0"
author = "YourName"
description = "A short description"
main = "main.lua"          # Entry point script (default: main.lua)

[limits]
memory_mb = 16             # Max memory in MB (default: 16)
instruction_limit = 1000000 # Max instructions per callback (default: 1,000,000)
```

## The `mc` API

All server interactions go through the global `mc` table. Functions are grouped by category.

### Event Handling

#### `mc.on(event_name, handler)`

Register an event handler. The handler receives an event table.

```lua
mc.on("player_join", function(event)
    mc.broadcast("Welcome " .. event.player.name .. "!")
end)
```

To **cancel** a cancellable event, set `event.cancelled = true`:

```lua
mc.on("player_chat", function(event)
    if event.message:find("spam") then
        event.cancelled = true
    end
end)
```

See the [Event Reference](events) for all 17 events and their fields.

---

### Player Functions

#### `mc.send_message(player_name, message)`

Send a chat message to a specific player.

```lua
mc.send_message("Steve", "Hello Steve!")
```

#### `mc.broadcast(message)`

Send a message to all online players.

```lua
mc.broadcast("Server announcement!")
```

#### `mc.kick(player_name, reason)`

Disconnect a player.

```lua
mc.kick("Steve", "You have been kicked")
```

#### `mc.set_health(player_name, health)`

Set a player's health (0.0 to 20.0).

```lua
mc.set_health("Steve", 20.0)  -- Full health
```

#### `mc.set_food(player_name, food)`

Set a player's hunger level (0 to 20).

```lua
mc.set_food("Steve", 20)  -- Full hunger
```

#### `mc.teleport(player_name, x, y, z)`

Teleport a player to coordinates.

```lua
mc.teleport("Steve", 0.5, 65.0, 0.5)
```

---

### Player Queries

#### `mc.online_players()`

Returns a table (array) of all online players. Each player is a table with fields:

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Player name |
| `uuid` | string | Player UUID |
| `runtime_id` | number | Entity runtime ID |
| `x` | number | X coordinate |
| `y` | number | Y coordinate |
| `z` | number | Z coordinate |
| `gamemode` | number | 0=survival, 1=creative, 2=adventure, 3=spectator |
| `health` | number | Current health (0.0–20.0) |

```lua
local players = mc.online_players()
for _, p in ipairs(players) do
    mc.log(p.name .. " at " .. p.x .. ", " .. p.y .. ", " .. p.z)
end
```

#### `mc.get_player(name)`

Get a specific player by name. Returns a player table or `nil` if not found.

```lua
local player = mc.get_player("Steve")
if player then
    mc.log("Steve's health: " .. player.health)
end
```

---

### World Functions

#### `mc.get_time()`

Get the current world time (0–24000).

```lua
local time = mc.get_time()
mc.log("Current time: " .. time)
```

#### `mc.set_time(time)`

Set the world time.

```lua
mc.set_time(6000)  -- Noon
```

#### `mc.is_raining()`

Check if it's raining.

```lua
if mc.is_raining() then
    mc.broadcast("It's raining!")
end
```

---

### Entity Functions

#### `mc.spawn_mob(mob_type, x, y, z)`

Spawn a mob at coordinates. Available types: `zombie`, `skeleton`, `cow`, `pig`, `chicken`.

```lua
mc.spawn_mob("zombie", 10.5, 65.0, 10.5)
```

#### `mc.remove_mob(runtime_id)`

Remove a mob by its runtime ID.

```lua
mc.remove_mob(42)
```

---

### Server Functions

#### `mc.get_tick()`

Get the current server tick (20 ticks = 1 second).

```lua
mc.log("Server tick: " .. mc.get_tick())
```

---

### Logging

#### `mc.log(message)`

Log at INFO level.

#### `mc.log_warn(message)`

Log at WARN level.

#### `mc.log_error(message)`

Log at ERROR level.

#### `mc.log_debug(message)`

Log at DEBUG level.

All log messages are prefixed with the plugin name:
```
[MyPlugin] Your message here
```

---

### Commands

#### `mc.register_command(name, description, handler)`

Register a custom command. The handler receives the sender name and an arguments table. Return a string to send a response, or `nil` for no response.

```lua
mc.register_command("spawn", "Teleport to spawn", function(sender, args)
    mc.teleport(sender, 0.5, 65.0, 0.5)
    return "Teleported to spawn!"
end)

mc.register_command("heal", "Heal a player", function(sender, args)
    local target = args[1] or sender
    mc.set_health(target, 20.0)
    mc.set_food(target, 20)
    return "Healed " .. target
end)
```

---

### Task Scheduler

#### `mc.schedule(delay_ticks, callback)`

Schedule a one-shot task. Returns a task ID. (20 ticks = 1 second)

```lua
-- Run after 5 seconds
local task_id = mc.schedule(100, function()
    mc.broadcast("5 seconds have passed!")
end)
```

#### `mc.schedule_repeating(delay_ticks, interval_ticks, callback)`

Schedule a repeating task. Returns a task ID.

```lua
-- Broadcast every 60 seconds, starting after 60 seconds
local task_id = mc.schedule_repeating(1200, 1200, function()
    mc.broadcast("Periodic announcement!")
end)
```

#### `mc.cancel_task(task_id)`

Cancel a scheduled task.

```lua
local task_id = mc.schedule_repeating(200, 200, function()
    mc.broadcast("Repeating...")
end)

-- Cancel it later
mc.cancel_task(task_id)
```

---

## Sandbox Restrictions

For security, the following Lua globals are **removed** and unavailable:

| Removed | Reason |
|---------|--------|
| `os` | Filesystem and process access |
| `io` | File I/O operations |
| `debug` | Debug hooks and internals |
| `loadfile` | Dynamic file loading |
| `dofile` | Dynamic file execution |

**Available** standard libraries: `table`, `string`, `math`, `ipairs`, `pairs`, `type`, `tostring`, `tonumber`, `select`, `unpack`, `pcall`, `xpcall`, `error`, `assert`, `rawget`, `rawset`, `setmetatable`, `getmetatable`.

## Resource Limits

Each Lua plugin runs in its own isolated Lua VM with configurable limits:

- **Memory**: Controlled by `memory_mb` in `plugin.toml` (default: 16 MB)
- **Instructions**: Controlled by `instruction_limit` per callback (default: 1,000,000)

If a plugin exceeds its limits, the callback is interrupted and an error is logged.
