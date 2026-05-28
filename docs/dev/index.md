---
layout: default
title: MC-RS Developer Docs
nav_exclude: true
---

# MC-RS Developer Documentation

MC-RS targets **protocol 975 (Bedrock 1.26.20)** and reimplements PocketMine-MP
in Rust across a 7-crate workspace (~90k lines).

## Implemented foundations

- RakNet transport (UDP, reliability, ordering, fragmentation, ACK/NACK)
- Bedrock protocol codec (VarInt, zlib/snappy batches, packet headers)
- Login & encryption (ECDH P-384, AES-256-CTR, Xbox Live JWT chain + offline)
- World generation (flat + Simplex 3D terrain, biomes, ores, trees, structures)
- Persistence (LevelDB chunk storage, player data, level.dat)
- Multi-player sync, day/night cycle, weather
- 65 built-in commands with tab-complete
- Web admin panel (`mc-rs-webui`), RCON, GameSpy4 Query

## Plugin system (Lua)

The plugin runtime is **implemented** using `mlua` (Lua 5.4). Plugins are
discovered from a `plugin.yml` manifest (PMMP-style) and can:

- **Register commands** declared in the manifest, with a Lua handler
- **Subscribe to events** (e.g. `PlayerJoin`) via `RegisterEvent`
- **Schedule tasks** to fire after N server ticks
- **Log / broadcast** messages through the host API

> A native Rust plugin trait (`ServerCommandRuntime`) backs the command engine;
> a WASM runtime is a possible future addition but is not implemented today.

## Resource packs & custom UI

See [Resource Pack & Custom UI]({{ '/dev/resource-pack-ui/' | relative_url }})
for the server-driven UI system: the title-flag dispatcher, the 8 custom
form layouts, and the step-by-step guide for creating your own layout.

## Follow development

Check the [main documentation](../) for project status, or visit the
[GitHub repository](https://github.com/pedrokarim/mc-rs).
