---
layout: default
title: Event Reference
nav_order: 5
---

# Event Reference

MC-RS dispatches 17 events to plugins. **10 events are cancellable** — returning `Cancelled` from a cancellable event prevents the default server action and stops propagation to remaining plugins.

## Summary

| Event | Category | Cancellable | Fields |
|-------|----------|:-----------:|--------|
| `PlayerJoin` | Player | No | player |
| `PlayerQuit` | Player | No | player |
| `PlayerChat` | Player | **Yes** | player, message |
| `PlayerCommand` | Player | **Yes** | player, command, args |
| `PlayerMove` | Player | **Yes** | player, from, to |
| `PlayerDeath` | Player | No | player, message |
| `PlayerDamage` | Player | **Yes** | player, damage, cause |
| `PlayerRespawn` | Player | No | player |
| `BlockBreak` | Block | **Yes** | player, position, block_id |
| `BlockPlace` | Block | **Yes** | player, position, block_id |
| `MobSpawn` | Entity | **Yes** | mob_type, runtime_id, position |
| `MobDeath` | Entity | No | mob_type, runtime_id, killer_runtime_id |
| `EntityDamage` | Entity | **Yes** | runtime_id, damage, attacker_runtime_id |
| `WeatherChange` | World | **Yes** | raining, thundering |
| `TimeChange` | World | **Yes** | new_time |
| `ServerStarted` | Server | No | *(none)* |
| `ServerStopping` | Server | No | *(none)* |

---

## Player Events

### PlayerJoin

Fired when a player joins the server, after the login sequence completes.

| Field | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | The joining player |

**Lua event name:** `player_join`

```lua
mc.on("player_join", function(event)
    mc.broadcast(event.player.name .. " joined the server!")
end)
```

---

### PlayerQuit

Fired when a player disconnects.

| Field | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | The leaving player |

**Lua event name:** `player_quit`

```lua
mc.on("player_quit", function(event)
    mc.broadcast(event.player.name .. " left the server.")
end)
```

---

### PlayerChat (Cancellable)

Fired when a player sends a chat message. Cancel to prevent the message from being broadcast.

| Field | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | The sender |
| `message` | String | The chat message |

**Lua event name:** `player_chat`

```lua
mc.on("player_chat", function(event)
    if event.message:find("badword") then
        event.cancelled = true
        mc.send_message(event.player.name, "Watch your language!")
    end
end)
```

---

### PlayerCommand (Cancellable)

Fired when a player executes a command. Cancel to prevent execution.

| Field | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | The player |
| `command` | String | Command name (without `/`) |
| `args` | String[] | Command arguments |

**Lua event name:** `player_command`

```lua
mc.on("player_command", function(event)
    mc.log("Command: /" .. event.command .. " by " .. event.player.name)
end)
```

---

### PlayerMove (Cancellable)

Fired when a player moves. Cancel to teleport them back to the `from` position.

| Field | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | The player |
| `from` | (f32, f32, f32) | Previous position (x, y, z) |
| `to` | (f32, f32, f32) | New position (x, y, z) |

**Lua event name:** `player_move`

In Lua, `from` and `to` are tables with `x`, `y`, `z` fields:

```lua
mc.on("player_move", function(event)
    -- Prevent moving beyond X=100
    if event.to.x > 100 then
        event.cancelled = true
    end
end)
```

---

### PlayerDeath

Fired when a player dies.

| Field | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | The player who died |
| `message` | String | Death message |

**Lua event name:** `player_death`

```lua
mc.on("player_death", function(event)
    mc.broadcast(event.message)
end)
```

---

### PlayerDamage (Cancellable)

Fired when a player takes damage. Cancel to prevent the damage.

| Field | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | The damaged player |
| `damage` | f32 | Damage amount (in half-hearts) |
| `cause` | DamageCause | What caused the damage |

**Lua event name:** `player_damage`

DamageCause values in Lua: `"Attack"`, `"Fall"`, `"Drowning"`, `"Lava"`, `"Fire"`, `"Suffocation"`, `"Starvation"`, `"Void"`, `"Other"`

```lua
mc.on("player_damage", function(event)
    -- Disable fall damage
    if event.cause == "Fall" then
        event.cancelled = true
    end
end)
```

---

### PlayerRespawn

Fired when a player respawns after death.

| Field | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | The respawning player |

**Lua event name:** `player_respawn`

```lua
mc.on("player_respawn", function(event)
    mc.send_message(event.player.name, "Welcome back!")
end)
```

---

## Block Events

### BlockBreak (Cancellable)

Fired when a player breaks a block. Cancel to prevent the block from being broken.

| Field | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | The player |
| `position` | PluginBlockPos | Block coordinates (x, y, z) |
| `block_id` | u32 | Block runtime ID (FNV-1a hash) |

**Lua event name:** `block_break`

```lua
mc.on("block_break", function(event)
    local pos = event.position
    mc.log("Block broken at " .. pos.x .. ", " .. pos.y .. ", " .. pos.z)
end)
```

---

### BlockPlace (Cancellable)

Fired when a player places a block. Cancel to prevent placement.

| Field | Type | Description |
|-------|------|-------------|
| `player` | PluginPlayer | The player |
| `position` | PluginBlockPos | Block coordinates (x, y, z) |
| `block_id` | u32 | Block runtime ID (FNV-1a hash) |

**Lua event name:** `block_place`

```lua
mc.on("block_place", function(event)
    -- Prevent placing blocks above Y=200
    if event.position.y > 200 then
        event.cancelled = true
        mc.send_message(event.player.name, "Build limit reached!")
    end
end)
```

---

## Entity Events

### MobSpawn (Cancellable)

Fired when a mob spawns (naturally or via command). Cancel to prevent the spawn.

| Field | Type | Description |
|-------|------|-------------|
| `mob_type` | String | Mob identifier (e.g., `"zombie"`) |
| `runtime_id` | u64 | Entity runtime ID |
| `position` | (f32, f32, f32) | Spawn coordinates (x, y, z) |

**Lua event name:** `mob_spawn`

```lua
mc.on("mob_spawn", function(event)
    -- No zombies allowed!
    if event.mob_type == "zombie" then
        event.cancelled = true
    end
end)
```

---

### MobDeath

Fired when a mob dies.

| Field | Type | Description |
|-------|------|-------------|
| `mob_type` | String | Mob identifier |
| `runtime_id` | u64 | Entity runtime ID |
| `killer_runtime_id` | Option\<u64\> | Runtime ID of the killer (if any) |

**Lua event name:** `mob_death`

In Lua, `killer_runtime_id` is `nil` if there was no killer:

```lua
mc.on("mob_death", function(event)
    if event.killer_runtime_id then
        mc.log(event.mob_type .. " killed by entity " .. event.killer_runtime_id)
    end
end)
```

---

### EntityDamage (Cancellable)

Fired when any entity takes damage. Cancel to prevent the damage.

| Field | Type | Description |
|-------|------|-------------|
| `runtime_id` | u64 | Damaged entity's runtime ID |
| `damage` | f32 | Damage amount |
| `attacker_runtime_id` | Option\<u64\> | Attacker's runtime ID (if any) |

**Lua event name:** `entity_damage`

```lua
mc.on("entity_damage", function(event)
    mc.log("Entity " .. event.runtime_id .. " took " .. event.damage .. " damage")
end)
```

---

## World Events

### WeatherChange (Cancellable)

Fired when the weather changes. Cancel to keep the current weather.

| Field | Type | Description |
|-------|------|-------------|
| `raining` | bool | Whether it will rain |
| `thundering` | bool | Whether there will be thunder |

**Lua event name:** `weather_change`

```lua
mc.on("weather_change", function(event)
    -- Keep it sunny!
    if event.raining then
        event.cancelled = true
    end
end)
```

---

### TimeChange (Cancellable)

Fired when the world time changes (via command or natural cycle). Cancel to prevent the change.

| Field | Type | Description |
|-------|------|-------------|
| `new_time` | i64 | The new world time |

**Lua event name:** `time_change`

```lua
mc.on("time_change", function(event)
    mc.log("Time changing to " .. event.new_time)
end)
```

---

## Server Events

### ServerStarted

Fired once on the first game tick, after all plugins are loaded.

**Lua event name:** `server_started`

```lua
mc.on("server_started", function(event)
    mc.log("Server is ready!")
end)
```

---

### ServerStopping

Fired during server shutdown, before plugins are disabled. Actions queued during this event are **not applied** (the server is shutting down).

**Lua event name:** `server_stopping`

```lua
mc.on("server_stopping", function(event)
    mc.log("Server is shutting down...")
end)
```

---

## WASM Event Format

For WASM plugins, events are serialized as JSON and passed to `__on_event(ptr, len)`. The JSON uses serde's default tagged enum format:

```json
{
  "PlayerChat": {
    "player": {
      "name": "Steve",
      "uuid": "...",
      "runtime_id": 1,
      "position": [0.5, 65.62, 0.5],
      "gamemode": 0,
      "health": 20.0
    },
    "message": "Hello world"
  }
}
```

Return `1` from `__on_event` to cancel, `0` to continue.
