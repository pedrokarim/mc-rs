window.SEARCH_DATA = [
  {
    "title": "Home",
    "url": "./",
    "section": "Getting Started",
    "content": "MC-RS is a Minecraft Bedrock Edition server written in Rust, based on PocketMine-MP. Clean architecture, functional networking, and real protocol compliance. 6 crates, 16 commands, protocol 924, Bedrock 1.26.2. RakNet UDP transport with ECDH P-384 key exchange and AES-256-CTR encryption. Xbox Live JWT authentication and offline mode. Flat and Perlin noise terrain generation. Multi-player support with player sync and broadcasts. NBT codec with little-endian and network ZigZag variants."
  },
  {
    "title": "Overview",
    "url": "pages/overview.html",
    "section": "Getting Started",
    "content": "MC-RS is a Minecraft Bedrock Edition server written in Rust. Based on PocketMine-MP reference implementation. Protocol version 924 for Bedrock 1.26.2. Currently handles login, encryption, chunk streaming, player movement, chat, commands, and multi-player. Why Rust: performance, memory safety, fearless concurrency, strong type system. Comparison with PocketMine-MP PHP and Nukkit Java. Foundation and player basics phases completed. World interaction, entities, and gameplay phases planned."
  },
  {
    "title": "Architecture",
    "url": "pages/architecture.html",
    "section": "Getting Started",
    "content": "6-crate Rust workspace with clean separation of concerns. mc-rs-server entry point, tokio async runtime, connection state machine, game tick loop, player broadcasts. mc-rs-proto packet encoding decoding, VarInt VarUInt codec, zlib compression, batch framing. mc-rs-raknet RakNet UDP transport, reliability, fragmentation, ACK NACK, MOTD. mc-rs-crypto ECDH P-384, AES-256-CTR, JWT parsing. mc-rs-nbt NBT little-endian and network variant, all tag types. mc-rs-command 16 commands with CommandAction system. Data flow: Client UDP RakNet Decrypt Decompress Batch Split Decode Connection Handler. Connection states: SessionStart Login Encryption ResourcePacks PreSpawn InGame. Single-task async model with tokio."
  },
  {
    "title": "Protocol",
    "url": "pages/protocol.html",
    "section": "Core Systems",
    "content": "Minecraft Bedrock Edition protocol version 924. Login flow: RequestNetworkSettings, NetworkSettings compression, Login JWT chain, ServerToClientHandshake ECDH, ClientToServerHandshake, PlayStatus LoginSuccess, resource packs, StartGame, chunk loading, PlayStatus PlayerSpawn. Packet structure: batch framing, compression header, zlib, sub-packets with VarUInt32 length and packet ID. Key packets: Login, PlayStatus, StartGame, AddPlayer, MovePlayer, LevelChunk, SetTime, PlayerAuthInput, Text, UpdateAttributes. Block network IDs are hashes: sequential indices from canonical_block_states.nbt. Data types: VarInt zigzag, VarLong, VarUInt32, VarUInt64, Vec3, Vec2, BlockPos, Uuid, String."
  },
  {
    "title": "Networking",
    "url": "pages/networking.html",
    "section": "Core Systems",
    "content": "RakNet UDP transport on port 19132 with encryption, fragmentation, reliability. Connection handshake, five reliability types: unreliable, unreliable sequenced, reliable, reliable ordered, reliable sequenced. Sequence numbers, ACK NACK, congestion control. Fragmentation with compound_id compound_size compound_index reassembly. Encryption: ECDH P-384 key exchange, AES-256-CTR stream cipher, SHA-256 key derivation. Compression: zlib deflate snappy, configurable threshold. Connection lifecycle: unconnected ping pong, MTU negotiation, game packets, disconnect."
  },
  {
    "title": "World",
    "url": "pages/world.html",
    "section": "Core Systems",
    "content": "Chunk-based world with flat and Perlin noise terrain generation. Chunks 16x16 columns, sub-chunks 16x16x16, Y range -64 to 319, 24 sub-chunks, paletted storage. Flat generator: bedrock dirt grass layers. Terrain generator: multi-octave Perlin noise, dynamic heights, stone dirt grass sand water gravel snow oak leaves. Sub-chunk encoding with dynamic bits per block. Biome sections 4x4x4 resolution. Block network IDs from canonical_block_states.nbt. Day night cycle 24000 ticks. Weather clear rain thunder. Chunk streaming around player position. Dynamic spawn position based on terrain height."
  },
  {
    "title": "Commands",
    "url": "pages/commands.html",
    "section": "Features",
    "content": "16 built-in server commands. /help show commands, /list online players, /tp teleport with relative coordinates, /gamemode survival creative adventure spectator, /say broadcast, /time set day noon sunset night, /kill, /stop server, /ping latency, /seed world seed, /pos coordinates, /difficulty peaceful easy normal hard, /weather clear rain thunder, /spawn teleport to spawn, /up teleport up, /clear inventory. CommandAction system: Teleport SetGamemode Broadcast Stop SetTime SetWeather Kill None."
  },
  {
    "title": "Configuration",
    "url": "pages/configuration.html",
    "section": "Features",
    "content": "Server configuration via server.toml with sensible defaults. [server] motd, sub_motd, port 19132, max_players 20, online_mode, view_distance 8, tick_rate 10ms. [world] name, generator flat or terrain, seed. [gameplay] gamemode creative, difficulty normal, pvp, do_daylight_cycle, do_weather_cycle, spawn_protection 16. Auto-created on first run. Player data stored as JSON in players directory."
  },
  {
    "title": "Roadmap",
    "url": "pages/roadmap.html",
    "section": "Roadmap",
    "content": "Development phases for MC-RS. Phase 1 Foundation completed: RakNet protocol login encryption flat world movement chat. Phase 2 Player Basics completed: multi-player commands terrain generation time weather player persistence. Phase 3 World Interaction next: block breaking placing items inventory chunk persistence LevelDB. Phase 4 Entities Combat planned: mob spawning AI combat hunger effects. Phase 5 Game Systems planned: crafting enchanting advanced world generation. Phase 6 Plugin System planned: event system Lua WASM plugins. Phase 7 Polish planned: performance anti-cheat RCON scoreboard."
  }
];
