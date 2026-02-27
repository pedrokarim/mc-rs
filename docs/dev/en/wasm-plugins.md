---
layout: default
title: WASM Plugins
nav_order: 7
---

# WASM Plugin Development

MC-RS supports WebAssembly plugins via [wasmtime](https://wasmtime.dev/). WASM plugins can be written in any language that compiles to `wasm32-unknown-unknown` (Rust, C, Zig, etc.).

## Manifest (`plugin.toml`)

```toml
[plugin]
name = "MyWasmPlugin"
version = "1.0.0"
author = "YourName"
description = "A WASM plugin"
wasm_file = "plugin.wasm"       # Path to the .wasm file

[limits]
fuel_per_event = 1000000        # Fuel budget per event callback (default: 1,000,000)
fuel_per_command = 1000000      # Fuel budget per command callback (default: 1,000,000)
fuel_per_task = 500000          # Fuel budget per task callback (default: 500,000)
fuel_on_enable = 5000000        # Fuel budget for on_enable (default: 5,000,000)
max_memory_pages = 256          # Max memory pages, each 64KB (default: 256 = 16MB)
```

The presence of `wasm_file` is what distinguishes WASM plugins from Lua plugins.

## Required Exports

Your WASM module **must** export the following:

| Export | Signature | Description |
|--------|-----------|-------------|
| `memory` | Memory | Exported linear memory |
| `__malloc` | `(size: i32) -> i32` | Allocate guest memory, return pointer |
| `__free` | `(ptr: i32, size: i32)` | Free guest memory |
| `__plugin_info` | `() -> i32` | Return plugin info (length-prefixed JSON) |
| `__on_enable` | `()` | Called when the plugin is loaded |
| `__on_disable` | `()` | Called when the plugin is unloaded |
| `__on_event` | `(ptr: i32, len: i32) -> i32` | Handle event (JSON input), return 0=continue, 1=cancel |
| `__on_task` | `(task_id: i32)` | Handle scheduled task |
| `__on_command` | `(ptr: i32, len: i32) -> i32` | Handle command (JSON input), return length-prefixed response or 0 |
| `__default_config` | `() -> i32` | Return default config (length-prefixed JSON or 0) |
| `__load_config` | `(ptr: i32, len: i32)` | Receive config (JSON input) |

## Host Functions (`mcrs` Module)

Import these functions from the `"mcrs"` module to interact with the server:

### Player API

| Function | Signature | Description |
|----------|-----------|-------------|
| `send_message` | `(name_ptr: i32, name_len: i32, msg_ptr: i32, msg_len: i32)` | Send message to player |
| `broadcast_message` | `(ptr: i32, len: i32)` | Broadcast to all players |
| `kick_player` | `(name_ptr: i32, name_len: i32, reason_ptr: i32, reason_len: i32)` | Kick a player |
| `set_player_health` | `(name_ptr: i32, name_len: i32, health: f32)` | Set player health |
| `set_player_food` | `(name_ptr: i32, name_len: i32, food: i32)` | Set player food |
| `teleport_player` | `(name_ptr: i32, name_len: i32, x: f32, y: f32, z: f32)` | Teleport player |
| `online_players` | `() -> i32` | Get all players (returns length-prefixed JSON) |
| `get_player` | `(name_ptr: i32, name_len: i32) -> i32` | Get player (returns length-prefixed JSON or 0) |

### World API

| Function | Signature | Description |
|----------|-----------|-------------|
| `get_time` | `() -> i64` | Get world time |
| `set_time` | `(time: i64)` | Set world time |
| `is_raining` | `() -> i32` | Check weather (1=raining, 0=clear) |

### Entity API

| Function | Signature | Description |
|----------|-----------|-------------|
| `spawn_mob` | `(type_ptr: i32, type_len: i32, x: f32, y: f32, z: f32)` | Spawn a mob |
| `remove_mob` | `(runtime_id: i64)` | Remove a mob |

### Server API

| Function | Signature | Description |
|----------|-----------|-------------|
| `get_tick` | `() -> i64` | Get current server tick |
| `log` | `(level: i32, ptr: i32, len: i32)` | Log message (0=info, 1=warn, 2=error, 3=debug) |

### Scheduler

| Function | Signature | Description |
|----------|-----------|-------------|
| `schedule_delayed` | `(delay_ticks: i64, task_id: i32)` | Schedule one-shot task |
| `schedule_repeating` | `(delay_ticks: i64, interval_ticks: i64, task_id: i32)` | Schedule repeating task |
| `cancel_task` | `(task_id: i32)` | Cancel a task |

### Commands

| Function | Signature | Description |
|----------|-----------|-------------|
| `register_command` | `(name_ptr: i32, name_len: i32, desc_ptr: i32, desc_len: i32)` | Register a command |

## Memory Protocol

All strings are passed between host and guest as raw UTF-8 bytes using `(pointer, length)` pairs.

### Reading strings (host -> guest)

When the host calls your exported function with `(ptr, len)`:
1. Read `len` bytes starting at `ptr` in your linear memory
2. Decode as UTF-8

### Writing strings (guest -> host)

When calling host functions, pass a pointer to your string data and its length. The string must be in your exported `memory`.

### Length-prefixed returns

Functions like `online_players()` and `get_player()` return a pointer to a **length-prefixed string**:

```
Offset 0: u32 (little-endian) — byte length of the UTF-8 data
Offset 4: UTF-8 bytes
```

If the function returns `0`, it means "not found" or "empty".

## Event JSON Format

Events are passed to `__on_event` as JSON using serde's externally-tagged enum format:

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

## Command JSON Format

Commands are passed to `__on_command` as:

```json
{
  "command": "hello",
  "args": ["arg1", "arg2"],
  "sender": "Steve"
}
```

Return a length-prefixed string with the response message, or `0` for no response.

## Fuel Metering

Each callback has a **fuel budget** (configured in `plugin.toml`). Every WASM instruction consumes fuel. If a callback exhausts its fuel, execution is interrupted and an error is logged.

Default budgets:
- `on_enable`: 5,000,000
- Events: 1,000,000
- Commands: 1,000,000
- Tasks: 500,000

## Minimal Example (WAT)

Here's the absolute minimum WASM plugin (in WAT text format):

```wat
(module
    (memory (export "memory") 1)
    (global $heap_ptr (mut i32) (i32.const 1024))

    ;; Simple bump allocator
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
        (i32.const 0)  ;; Always continue
    )
    (func (export "__on_task") (param $task_id i32))
    (func (export "__on_command") (param $ptr i32) (param $len i32) (result i32)
        (i32.const 0)  ;; No response
    )
    (func (export "__default_config") (result i32) (i32.const 0))
    (func (export "__load_config") (param $ptr i32) (param $len i32))
)
```

## Developing in Rust

For Rust WASM plugins, target `wasm32-unknown-unknown`:

### `Cargo.toml`

```toml
[package]
name = "my-mcrs-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]
```

### `src/lib.rs`

```rust
// Import host functions from the "mcrs" module
extern "C" {
    #[link_name = "broadcast_message"]
    fn host_broadcast(ptr: *const u8, len: i32);
    #[link_name = "log"]
    fn host_log(level: i32, ptr: *const u8, len: i32);
}

// Simple bump allocator
static mut HEAP: [u8; 65536] = [0; 65536];
static mut HEAP_POS: usize = 0;

#[no_mangle]
pub extern "C" fn __malloc(size: i32) -> i32 {
    unsafe {
        let ptr = HEAP_POS;
        HEAP_POS += size as usize;
        HEAP.as_ptr().add(ptr) as i32
    }
}

#[no_mangle]
pub extern "C" fn __free(_ptr: i32, _size: i32) {}

#[no_mangle]
pub extern "C" fn __on_enable() {
    let msg = "Hello from WASM!";
    unsafe { host_log(0, msg.as_ptr(), msg.len() as i32); }
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

### Build

```bash
cargo build --target wasm32-unknown-unknown --release
cp target/wasm32-unknown-unknown/release/my_mcrs_plugin.wasm plugins/my-plugin/plugin.wasm
```
