---
layout: default
title: Getting Started
nav_order: 2
---

# Getting Started

Create your first MC-RS plugin in under 5 minutes.

## Prerequisites

- A compiled `mc-rs-server` binary (or `cargo run` from the workspace)
- For Lua plugins: nothing else needed (Lua 5.4 is embedded)
- For WASM plugins: a Rust toolchain with `wasm32-unknown-unknown` target

## Plugin Directory Structure

Plugins live in the `plugins/` directory at the server root. Each plugin is a subdirectory with a `plugin.toml` manifest:

```
server/
├── mc-rs-server(.exe)
├── server.toml
└── plugins/
    ├── my-lua-plugin/
    │   ├── plugin.toml
    │   └── main.lua
    └── my-wasm-plugin/
        ├── plugin.toml
        └── plugin.wasm
```

The server auto-detects plugin type: if `plugin.toml` contains a `wasm_file` field, it's loaded as WASM. Otherwise, it's loaded as Lua.

## Your First Lua Plugin

### 1. Create the directory

```bash
mkdir -p plugins/hello-world
```

### 2. Create `plugin.toml`

```toml
[plugin]
name = "HelloWorld"
version = "1.0.0"
author = "YourName"
description = "Greets players on join"
main = "main.lua"
```

### 3. Create `main.lua`

```lua
-- Greet players when they join
mc.on("player_join", function(event)
    mc.broadcast("Welcome " .. event.player.name .. " to the server!")
end)

-- Register a custom command
mc.register_command("hello", "Say hello to everyone", function(sender, args)
    mc.broadcast(sender .. " says hello!")
    return "Hello sent!"
end)

mc.log("HelloWorld plugin loaded!")
```

### 4. Start the server

```bash
cargo run
# or
./mc-rs-server
```

You should see in the logs:
```
Loaded Lua plugin: HelloWorld v1.0.0
[HelloWorld] HelloWorld plugin loaded!
```

## How Plugin Loading Works

On startup, the server:

1. Creates the `plugins/` directory if it doesn't exist
2. Scans each subdirectory for a `plugin.toml` file
3. Loads **WASM plugins** first (directories with `wasm_file` in manifest)
4. Loads **Lua plugins** second (directories without `wasm_file`)
5. Calls `on_enable` on each plugin in load order
6. Dispatches `ServerStarted` event on the first game tick

Plugins are unloaded in reverse order on server shutdown, receiving a `ServerStopping` event followed by `on_disable`.

## Plugin Configuration

Plugins can have a `config.json` file in their directory. The server loads it automatically and passes it to the plugin via `load_config()`.

For Lua plugins, you can provide a default config by returning a table from `default_config()` in the Plugin trait (this is handled automatically by the Lua runtime if you set defaults in your script).

## Next Steps

- Read the [Lua Scripting Guide](lua-scripting) for the complete `mc.*` API reference
- Check out the [Event Reference](events) to see all 17 events you can listen to
- See [Examples](examples) for more complete plugin examples
