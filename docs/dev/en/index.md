---
layout: default
title: Home
nav_order: 1
parent: null
---

# MC-RS Plugin Development

MC-RS is a Minecraft Bedrock Edition server written in Rust, targeting **protocol v766** (1.21.50). It features a modular plugin system supporting both **Lua scripts** and **WASM modules**.

## Architecture at a Glance

The server is organized as a Rust workspace with 11 crates:

| Crate | Role |
|-------|------|
| `mc-rs-server` | Entry point, orchestration, configuration |
| `mc-rs-proto` | Packet definitions, codec, serialization |
| `mc-rs-raknet` | RakNet UDP transport (reliability, fragmentation) |
| `mc-rs-crypto` | ECDH P-384, AES-256-CFB8, JWT |
| `mc-rs-nbt` | NBT little-endian + network variant |
| `mc-rs-world` | Chunks, block registry, world generation |
| `mc-rs-game` | Game logic (combat, food, crafting, XP) |
| `mc-rs-command` | Command framework |
| `mc-rs-plugin-api` | Plugin interfaces (types, traits, events) |
| `mc-rs-plugin-lua` | Lua 5.4 scripting runtime |
| `mc-rs-plugin-wasm` | WASM plugin runtime (wasmtime) |

## Plugin Runtimes

| Runtime | Language | Sandboxing | Best For |
|---------|----------|------------|----------|
| **Lua** | Lua 5.4 | Memory + instruction limits, `os`/`io`/`debug` removed | Quick scripts, simple plugins |
| **WASM** | Any → Wasm (Rust, C, ...) | Fuel metering + memory page limits | Performance-critical, complex plugins |

Both runtimes share the same [Plugin API](plugin-api) with 17 events, a task scheduler, and full server interaction.

## Documentation

- [Getting Started](getting-started) — Create your first plugin in 5 minutes
- [Architecture](architecture) — How the plugin system works internally
- [ServerApi Reference](plugin-api) — All available API functions and types
- [Event Reference](events) — Complete list of 17 events
- [Lua Scripting](lua-scripting) — Lua API guide and `mc.*` reference
- [WASM Plugins](wasm-plugins) — WASM development guide, exports and host functions
- [Examples](examples) — Complete working plugin examples

[Fran&ccedil;ais](../fr/){: .btn }
