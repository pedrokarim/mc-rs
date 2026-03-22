---
layout: default
title: MC-RS Developer Docs
nav_exclude: true
---

# MC-RS Developer Documentation

The plugin system for MC-RS is currently **planned** and not yet implemented.

## Current Status

MC-RS is focused on building a solid foundation first:

- RakNet networking
- Bedrock protocol (v924, Bedrock 1.26.2)
- Login & encryption (ECDH P-384, AES-256-CTR)
- World generation (flat & Perlin terrain)
- Multi-player support
- 16 built-in commands
- Day/night & weather cycles

## Planned Plugin Features

Once the core gameplay systems (block interaction, inventory, entities) are implemented, the plugin system will be developed with:

- **Event system** — Subscribe to server events (player join, chat, block changes)
- **Lua scripting** — Lightweight scripts for simple plugins
- **WASM plugins** — Sandboxed WebAssembly for performance-critical plugins
- **Rust API** — Native plugin trait for maximum performance

## Follow Development

Check the [main documentation](../) for the current project status, or visit the [GitHub repository](https://github.com/pedrokarim/mc-rs) to follow progress.
