window.SEARCH_DATA = [
  {
    "title": "Home",
    "url": "./",
    "section": "Getting Started",
    "content": "MC-RS is a Minecraft Bedrock Edition server written in Rust, based on PocketMine-MP. 7 crates, ~90K lines, 65 commands, protocol 975, Bedrock 1.26.20. RakNet UDP transport with ECDH P-384 and AES-256-CTR encryption. Xbox Live JWT authentication and offline mode. Simplex 3D terrain with biomes, ores, trees, structures and LevelDB persistence. Multi-player sync. Inventory, crafting, combat, survival, entities and mobs. Lua plugins, web admin panel, RCON, GameSpy4 Query, resource pack and custom UI."
  },
  {
    "title": "Overview",
    "url": "pages/overview.html",
    "section": "Getting Started",
    "content": "MC-RS is a Minecraft Bedrock Edition server written in Rust, based on the PocketMine-MP reference implementation. Protocol 975 for Bedrock 1.26.20. 7-crate workspace, ~90K lines. Handles login, encryption, resource packs, chunk streaming, movement, chat, commands and multi-player, then gameplay: world generation, persistence, inventories, crafting, combat, survival, entities. Lua plugin runtime, web admin panel, RCON and Query. Why Rust: performance, memory safety, fearless concurrency, strong type system. Comparison with BDS, PocketMine-MP and Nukkit."
  },
  {
    "title": "Architecture",
    "url": "pages/architecture.html",
    "section": "Getting Started",
    "content": "7-crate Rust workspace. mc-rs-server main binary: tokio runtime, connection state machine, game loop, world generation and persistence, entities, gameplay, commands, plugins, RCON Query. mc-rs-proto packet encoding decoding, VarInt VarUInt codec, zlib snappy compression. mc-rs-raknet RakNet UDP transport, reliability, fragmentation, ordering, ACK NACK. mc-rs-crypto ECDH P-384, AES-256-CTR, JWT. mc-rs-nbt little-endian and network NBT variants. mc-rs-command generic command engine, parser, permissions, selectors. mc-rs-webui Axum web admin panel, JWT auth, SQLite, WebSocket metrics. 100 server TPS, 20 game TPS. Connection states: SessionStart Login Handshake ResourcePacks PreSpawn SpawnResponse InGame."
  },
  {
    "title": "Protocol",
    "url": "pages/protocol.html",
    "section": "Core Systems",
    "content": "Minecraft Bedrock Edition protocol version 975. Login flow: RequestNetworkSettings, NetworkSettings compression, Login JWT chain, ServerToClientHandshake ECDH, ClientToServerHandshake, PlayStatus LoginSuccess, resource packs, StartGame, chunk loading, PlayStatus PlayerSpawn. Packet structure: batch framing, compression header, zlib, sub-packets with VarUInt32 length and packet ID. Key packets: Login, PlayStatus, StartGame, AddPlayer, MovePlayer, LevelChunk, SetTime, PlayerAuthInput, Text, UpdateAttributes. Block network ids are hashes false: sequential indices from canonical_block_states.nbt. Data types: VarInt zigzag, VarLong, VarUInt32, VarUInt64, Vec3, Vec2, BlockPos, Uuid, String."
  },
  {
    "title": "Networking",
    "url": "pages/networking.html",
    "section": "Core Systems",
    "content": "RakNet UDP transport on port 19132 with encryption, fragmentation, reliability. Connection handshake, reliability types: unreliable, unreliable sequenced, reliable, reliable ordered, reliable sequenced. Sequence numbers, ACK NACK, congestion control. Fragmentation with compound_id compound_size compound_index reassembly. Encryption: ECDH P-384 key exchange, AES-256-CTR stream cipher, SHA-256 key derivation. Compression: zlib deflate snappy, configurable threshold. Connection lifecycle: unconnected ping pong, MTU negotiation, game packets, disconnect."
  },
  {
    "title": "World",
    "url": "pages/world.html",
    "section": "Core Systems",
    "content": "Chunk-based world with flat and Simplex 3D terrain generation, biomes and LevelDB persistence. Chunks 16x16 columns, sub-chunks 16x16x16, paletted storage. Normal generator: Simplex 3D noise, 11 biomes from temperature rainfall, ores, trees, water, structures villages temples mineshafts strongholds monuments. Nether and End dimensions. Flat generator: bedrock dirt grass. Block network IDs sequential indices from canonical_block_states.nbt. Day night cycle 24000 ticks. Weather clear rain thunder. Chunk streaming around player. LevelDB chunk persistence under worlds db. Server-authoritative block break place. Random ticks."
  },
  {
    "title": "Commands",
    "url": "pages/commands.html",
    "section": "Features",
    "content": "65 built-in commands with tab-complete, entity selectors and relative coordinates. World blocks: setblock fill clone locate setworldspawn spawnpoint spreadplayers seed. Player: tp gamemode defaultgamemode give clear replaceitem effect enchant xp damage kill ability menu. Entities: summon loot tag testfor testforblock. World rules: time weather difficulty gamerule. Chat: say me tell tellraw title list. Audio: playsound stopsound music particle. Scoreboard boss. Moderation: op deop ban pardon ban-ip pardon-ip banlist kick whitelist transferserver. Admin: stop save save-on save-off reload status timings version plugins gc dumpmemory event help. Generic command engine in mc-rs-command, handlers in mc-rs-server, ServerCommandRuntime trait."
  },
  {
    "title": "Configuration",
    "url": "pages/configuration.html",
    "section": "Features",
    "content": "Server configuration via server.toml. [server] motd, sub_motd, port 19132, max_players 20, online_mode, view_distance 16, tick_rate 10ms. [world] name, generator normal or flat, seed. [gameplay] gamemode survival, difficulty normal, pvp, do_daylight_cycle, do_weather_cycle, spawn_protection 16. [logging] directory rotation max_files level stdout file ansi. [webui] enabled bind database_url sqlite session_duration_hours tls. [resource_pack] must_accept. File locations: worlds LevelDB, players JSON, resource_packs, logs, webui.db. Auto-created on first run."
  },
  {
    "title": "Roadmap",
    "url": "pages/roadmap.html",
    "section": "Roadmap",
    "content": "Development phases for MC-RS. Foundation done: RakNet protocol login encryption resource packs chunk streaming. Player basics done: multi-player 65 commands Simplex terrain time weather persistence. World interaction done: block break place inventory crafting. Entities combat done: mobs AI combat hunger survival effects. Game systems done: crafting enchanting world generation redstone containers. Plugin system done: Lua mlua manifests events scheduler commands forms. Server tooling done: anti-cheat RCON Query web panel scoreboard permissions advanced commands resource packs. Phase 8 in progress: vanilla parity and polish, Postgres MongoDB backends."
  },
  {
    "title": "Plugins",
    "url": "pages/plugins.html",
    "section": "Extensions",
    "content": "Lua 5.4 plugins via mlua. plugin.yml manifest PocketMine style: name version main api load commands permissions. Capabilities: register commands, subscribe to events like PlayerJoin, schedule callbacks after N ticks, log and broadcast. Example main.lua with mcrs.log, mcrs.command, mcrs.on, mcrs.after. /plugins lists loaded plugins, /reload reloads them. No WASM runtime. Native command engine via ServerCommandRuntime trait."
  },
  {
    "title": "Web Admin Panel",
    "url": "pages/web-panel.html",
    "section": "Extensions",
    "content": "Built-in web dashboard, the mc-rs-webui crate, Axum HTTP server default 127.0.0.1:8080. Live metrics TPS players memory uptime over WebSocket. Console run commands stream logs. Players users management roles admin moderator. World view. Audit log. JWT auth with Argon2 passwords, rate limiting, session_duration_hours. SQLite default, Postgres MongoDB planned. TLS optional. Configuration [webui] enabled bind database_url session_duration_hours, [webui.tls]."
  },
  {
    "title": "Resource Packs & UI",
    "url": "pages/resource-packs.html",
    "section": "Extensions",
    "content": "Server-driven resource pack delivery and 8 custom form layouts via a title-flag UI dispatcher. Pipeline: zip in memory, SHA-256, ResourcePacksInfo must_accept, chunked download 1MB, verify, ResourcePackStack. Title-flag dispatcher injects an invisible flag into the form title, pack server_form.json renders the matching layout. Layouts: grid left_button bottom_button image_grid square_image motd store wrapped. Flags m a b c d e f 0 1. Try in game: /menu, /menu showcase, /menu motd. Full guide and tutorial in developer docs."
  }
];
