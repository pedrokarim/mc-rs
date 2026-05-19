<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/License-MIT-4ecca3?style=for-the-badge" alt="MIT License">
  <img src="https://img.shields.io/badge/Tests-924-blue?style=for-the-badge" alt="924 Tests">
  <img src="https://img.shields.io/badge/Protocol-924_(1.26.0)-orange?style=for-the-badge" alt="Protocol 924">
</p>

# MC-RS

A high-performance **Minecraft Bedrock Edition** server written entirely in Rust.

MC-RS targets **protocol version 924** (Minecraft Bedrock 1.26.0). It implements the full server-authoritative gameplay loop — movement, combat, inventory, world generation, crafting, and more — with a modular plugin system supporting both WASM and Lua scripting.

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

## Web Admin Panel

MC-RS ships with an integrated **web back-office** (`mc-rs-webui` crate) — no
external panel to set up, it boots with the server. Real-time monitoring,
remote control, player management and an audit trail, served straight from the
server binary.

<p align="center">
  <img src="docs/screenshots/02-dashboard.png" alt="Dashboard" width="48%">
  <img src="docs/screenshots/03-console.png" alt="Console web" width="48%">
</p>
<p align="center">
  <img src="docs/screenshots/04-world.png" alt="Contrôles monde" width="32%">
  <img src="docs/screenshots/05-audit.png" alt="Audit log" width="32%">
  <img src="docs/screenshots/01-login.png" alt="Login" width="32%">
</p>

### Enabling it

In `server.toml` (off by default for safety):

```toml
[webui]
enabled = true
bind = "127.0.0.1:8080"          # loopback only by default
database_url = "sqlite://webui.db"
session_duration_hours = 24
```

On first boot with an empty database, the panel redirects to `/setup` to
create the first administrator. After that, `/setup` is locked and login is
required.

### Pages

| Page | What it does |
|------|--------------|
| **Dashboard** | Live metrics — TPS, players, chunks, memory, CPU, entities, uptime — with 5-minute time-series charts (uPlot) and per-card sparklines |
| **Joueurs** | Connected players list with kick / op / deop / gamemode actions |
| **Console** | Live log stream + a command terminal that runs anything you'd type in the server console |
| **Monde** | One-click time / weather / difficulty controls |
| **Configuration** | View & hot-edit `server.toml` (validated + atomic write) |
| **Audit log** | Paginated, immutable history of every admin action (who, when, what) |
| **Utilisateurs** | Admin-only user CRUD (create / delete / password / role) |

### How it works

The panel is wired into the server through three lightweight channels — **no
duplication of game logic**:

1. **Commands** — every admin action (stop, kick, op, `/time`, `/weather`…)
   is pushed into the same `console_tx` channel that feeds the server's stdin
   console, then dispatched via the existing `dispatch_command_line`. The web
   layer never reimplements gameplay.
2. **State snapshot** — an `Arc<RwLock<ServerSnapshot>>` is refreshed by the
   main loop (~20 Hz: TPS, players, chunks, world; ~1 Hz: system metrics +
   5-minute history ring-buffers) and **pushed over WebSocket** to connected
   browsers. Zero polling.
3. **Events & logs** — server events (join/quit/chat/death/gamemode) and every
   `tracing` log line are fanned out over `broadcast` channels and streamed
   live to the dashboard & console.

Auth is multi-user with **Argon2id** password hashing and **JWT** sessions
(secret auto-generated and persisted on first boot). Login is rate-limited
(5 attempts / 5 min / IP). Storage is pluggable: **SQLite** by default,
**PostgreSQL** and **MongoDB** behind feature flags. Optional **TLS** via the
`tls` feature. UI is server-rendered (Askama + htmx + Alpine.js + Lucide
icons) — no JS build step.

> A full reference doc will come later — this is just a quick tour.

## Documentation

Full documentation is available at:

**[https://pedrokarim.github.io/mc-rs/](https://pedrokarim.github.io/mc-rs/)**

## License

This project is licensed under the [MIT License](LICENSE).
