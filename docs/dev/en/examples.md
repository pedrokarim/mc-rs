---
layout: default
title: Examples
nav_order: 8
---

# Plugin Examples

Complete, ready-to-use plugin examples for MC-RS.

---

## Example 1: Welcome Plugin (Lua)

A plugin that greets new players, registers a command, and sends periodic announcements.

### `plugins/welcome/plugin.toml`

```toml
[plugin]
name = "Welcome"
version = "1.0.0"
author = "Server Admin"
description = "Welcomes players and sends announcements"
main = "main.lua"
```

### `plugins/welcome/main.lua`

```lua
-- Greet players on join
mc.on("player_join", function(event)
    mc.send_message(event.player.name, "Welcome to the server, " .. event.player.name .. "!")

    -- Tell everyone else
    local players = mc.online_players()
    for _, p in ipairs(players) do
        if p.name ~= event.player.name then
            mc.send_message(p.name, event.player.name .. " has joined!")
        end
    end
end)

-- Say goodbye
mc.on("player_quit", function(event)
    mc.broadcast(event.player.name .. " has left the server.")
end)

-- Custom /welcome command
mc.register_command("welcome", "Send a welcome message", function(sender, args)
    local target = args[1] or sender
    local player = mc.get_player(target)
    if player then
        mc.send_message(target, "Welcome to the server! Enjoy your stay!")
        return "Welcomed " .. target
    else
        return "Player not found: " .. target
    end
end)

-- Periodic announcement every 5 minutes (6000 ticks)
mc.schedule_repeating(6000, 6000, function()
    local count = #mc.online_players()
    if count > 0 then
        mc.broadcast("There are " .. count .. " players online!")
    end
end)

mc.log("Welcome plugin loaded!")
```

---

## Example 2: Chat Filter (Lua)

A basic chat filter that blocks messages containing forbidden words.

### `plugins/chat-filter/plugin.toml`

```toml
[plugin]
name = "ChatFilter"
version = "1.0.0"
description = "Filters bad words from chat"
main = "main.lua"
```

### `plugins/chat-filter/main.lua`

```lua
-- Forbidden words (lowercase)
local forbidden = { "badword1", "badword2", "spam" }

mc.on("player_chat", function(event)
    local msg = event.message:lower()
    for _, word in ipairs(forbidden) do
        if msg:find(word, 1, true) then
            event.cancelled = true
            mc.send_message(event.player.name, "Your message was blocked (forbidden word).")
            mc.log_warn(event.player.name .. " tried to say: " .. event.message)
            return
        end
    end
end)

mc.log("ChatFilter loaded with " .. #forbidden .. " forbidden words.")
```

---

## Example 3: Spawn Protection (Lua)

Prevents block breaking and placing near the spawn point.

### `plugins/spawn-protect/plugin.toml`

```toml
[plugin]
name = "SpawnProtection"
version = "1.0.0"
description = "Protects blocks near spawn"
main = "main.lua"
```

### `plugins/spawn-protect/main.lua`

```lua
local SPAWN_X = 0
local SPAWN_Z = 0
local RADIUS = 50

local function is_protected(pos)
    local dx = pos.x - SPAWN_X
    local dz = pos.z - SPAWN_Z
    return (dx * dx + dz * dz) < (RADIUS * RADIUS)
end

mc.on("block_break", function(event)
    if is_protected(event.position) then
        event.cancelled = true
        mc.send_message(event.player.name, "You cannot break blocks near spawn!")
    end
end)

mc.on("block_place", function(event)
    if is_protected(event.position) then
        event.cancelled = true
        mc.send_message(event.player.name, "You cannot place blocks near spawn!")
    end
end)

mc.log("Spawn protection active (radius: " .. RADIUS .. " blocks)")
```

---

## Example 4: Mob Arena (Lua)

A command-driven mob arena that spawns waves of enemies.

### `plugins/mob-arena/plugin.toml`

```toml
[plugin]
name = "MobArena"
version = "1.0.0"
description = "Spawn mob waves with /arena"
main = "main.lua"
```

### `plugins/mob-arena/main.lua`

```lua
local arena_task = nil
local wave = 0
local ARENA_X, ARENA_Y, ARENA_Z = 50.5, 65.0, 50.5

local mob_types = { "zombie", "skeleton", "zombie", "skeleton", "zombie" }

local function spawn_wave()
    wave = wave + 1
    mc.broadcast("Wave " .. wave .. " incoming!")

    local count = math.min(wave + 2, 10)  -- More mobs each wave
    for i = 1, count do
        local mob = mob_types[(i % #mob_types) + 1]
        local offset_x = (i - count/2) * 2
        mc.spawn_mob(mob, ARENA_X + offset_x, ARENA_Y, ARENA_Z)
    end
end

mc.register_command("arena", "Start or stop the mob arena", function(sender, args)
    local action = args[1] or "start"

    if action == "start" then
        if arena_task then
            return "Arena is already running!"
        end
        wave = 0
        mc.broadcast("Mob Arena started by " .. sender .. "!")
        spawn_wave()
        -- New wave every 30 seconds
        arena_task = mc.schedule_repeating(600, 600, function()
            spawn_wave()
        end)
        return "Arena started!"

    elseif action == "stop" then
        if arena_task then
            mc.cancel_task(arena_task)
            arena_task = nil
            wave = 0
            mc.broadcast("Mob Arena stopped!")
            return "Arena stopped."
        else
            return "Arena is not running."
        end
    else
        return "Usage: /arena <start|stop>"
    end
end)

mc.log("MobArena plugin loaded!")
```

---

## Example 5: Minimal WASM Plugin (Rust)

A complete WASM plugin written in Rust that logs a message on enable and broadcasts on player join.

### Directory structure

```
plugins/hello-wasm/
├── plugin.toml
└── plugin.wasm        (compiled from the Rust project below)

hello-wasm-src/
├── Cargo.toml
└── src/
    └── lib.rs
```

### `plugin.toml`

```toml
[plugin]
name = "HelloWasm"
version = "1.0.0"
author = "Dev"
description = "Minimal WASM plugin"
wasm_file = "plugin.wasm"

[limits]
fuel_per_event = 1000000
max_memory_pages = 16
```

### `Cargo.toml`

```toml
[package]
name = "hello-wasm"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = "s"
lto = true
```

### `src/lib.rs`

```rust
// ─── Host function imports (from "mcrs" module) ───

#[link(wasm_import_module = "mcrs")]
extern "C" {
    fn broadcast_message(ptr: i32, len: i32);
    fn log(level: i32, ptr: i32, len: i32);
}

// ─── Memory management ───

static mut HEAP: [u8; 32768] = [0; 32768];
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

// ─── Plugin lifecycle ───

#[no_mangle]
pub extern "C" fn __plugin_info() -> i32 { 0 }

#[no_mangle]
pub extern "C" fn __on_enable() {
    let msg = b"HelloWasm plugin enabled!";
    unsafe { log(0, msg.as_ptr() as i32, msg.len() as i32); }
}

#[no_mangle]
pub extern "C" fn __on_disable() {}

#[no_mangle]
pub extern "C" fn __on_event(ptr: i32, len: i32) -> i32 {
    // Read the event JSON from memory
    let json = unsafe {
        let slice = core::slice::from_raw_parts(ptr as *const u8, len as usize);
        core::str::from_utf8_unchecked(slice)
    };

    // Simple check: if the event contains "PlayerJoin", broadcast a welcome
    if json.contains("PlayerJoin") {
        let msg = b"A new player has joined!";
        unsafe { broadcast_message(msg.as_ptr() as i32, msg.len() as i32); }
    }

    0 // Continue (don't cancel)
}

#[no_mangle]
pub extern "C" fn __on_task(_task_id: i32) {}

#[no_mangle]
pub extern "C" fn __on_command(_ptr: i32, _len: i32) -> i32 { 0 }

#[no_mangle]
pub extern "C" fn __default_config() -> i32 { 0 }

#[no_mangle]
pub extern "C" fn __load_config(_ptr: i32, _len: i32) {}
```

### Build and deploy

```bash
cd hello-wasm-src
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/hello_wasm.wasm ../plugins/hello-wasm/plugin.wasm
```
