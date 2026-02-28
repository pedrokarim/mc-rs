<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/License-MIT-4ecca3?style=for-the-badge" alt="MIT License">
  <img src="https://img.shields.io/badge/Tests-924-blue?style=for-the-badge" alt="924 Tests">
  <img src="https://img.shields.io/badge/Protocol-766_(1.21.50)-orange?style=for-the-badge" alt="Protocol 766">
</p>

# MC-RS

A high-performance **Minecraft Bedrock Edition** server written entirely in Rust.

MC-RS targets **protocol version 766** (Minecraft Bedrock 1.21.50) with multi-version support down to 748 (1.21.40). It implements the full server-authoritative gameplay loop — movement, combat, inventory, world generation, crafting, and more — with a modular plugin system supporting both WASM and Lua scripting.

## Features

- **Full RakNet Implementation** — UDP transport with fragmentation, ordering, reliability, and encryption (ECDH P-384 + AES-256-CFB8)
- **World Generation** — Perlin noise terrain with 10 biomes, spaghetti caves, 8 ore types, 4 tree species, villages, dungeons, Nether, and End dimensions
- **ECS Entities** — bevy_ecs-powered entity system with 5 mob types, 9 AI behaviors, pathfinding, and natural spawn/despawn
- **Complete Survival** — Hunger, fall damage, drowning, lava, combat with armor/enchantments/criticals, 50+ crafting recipes, furnaces, enchanting tables, anvils
- **Plugin System** — Rust API with 15 event hooks, WASM runtime (wasmtime), Lua scripting (mlua), behavior pack support, forms UI
- **Persistent Worlds** — LevelDB chunk storage, player data, level.dat, auto-save, BDS world import/export
- **40+ Commands** — `/gamemode`, `/tp`, `/give`, `/fill`, `/execute`, `/scoreboard`, `/bossbar`, `/transfer`, and more
- **Anti-Cheat** — Speed, fly, noclip, reach validation, rate limiting, violation tracking with auto-kick
- **Server Admin** — RCON, Query protocol (GameSpy4), console REPL, permissions, whitelist, bans

## Workspace

| Crate | Description |
|-------|-------------|
| `mc-rs-server` | Server entry point, connection handling, orchestration |
| `mc-rs-proto` | Packet definitions, codec, serialization (50+ packet types) |
| `mc-rs-raknet` | RakNet transport (UDP, reliability, fragmentation, ordering) |
| `mc-rs-crypto` | ECDH P-384 key exchange, AES-256-CFB8 encryption, JWT |
| `mc-rs-nbt` | NBT little-endian + network variant parser/serializer |
| `mc-rs-world` | Chunks, block registry, world generation, LevelDB storage |
| `mc-rs-game` | Game logic (combat, food, recipes, enchantments, ECS) |
| `mc-rs-command` | Command framework, argument parsing, entity selectors |
| `mc-rs-plugin-api` | Plugin interfaces, event types, server API traits |
| `mc-rs-plugin-lua` | Lua scripting runtime (mlua) |
| `mc-rs-plugin-wasm` | WASM plugin runtime (wasmtime) |
| `mc-rs-behavior-pack` | Behavior pack loader (JSON entities, items, recipes, loot) |

## Quick Start

```bash
# Build
cargo build --release

# Run tests (924 tests)
cargo test

# Run the server
cargo run --release
```

The server reads configuration from `server.toml` (created on first run).

## Documentation

Full documentation is available at:

**[https://pedrokarim.github.io/mc-rs/](https://pedrokarim.github.io/mc-rs/)**

## License

This project is licensed under the [MIT License](LICENSE).
