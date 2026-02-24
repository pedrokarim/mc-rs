# 10 - Performance et optimisation

## Objectifs de performance

| Métrique | Cible | Justification |
|----------|-------|---------------|
| **TPS** | 20.0 stable | Tick rate Minecraft standard |
| **Joueurs** | 500+ simultanés | 5-10× les serveurs PHP/Java |
| **Latence tick** | < 50ms (p99) | Ne jamais dépasser un tick |
| **Mémoire** | < 4 GB pour 500 joueurs | Raisonnable pour un VPS |
| **Chunk gen** | < 100ms par chunk | Pas de lag au déplacement |
| **Temps de démarrage** | < 5 secondes | Redémarrage rapide |

## Optimisations par couche

### Réseau

#### Compression intelligente

```rust
// Choisir la compression selon la taille du paquet
fn compress_packet(data: &[u8], algorithm: CompressionAlgorithm, threshold: usize) -> Vec<u8> {
    if data.len() < threshold {
        // Pas de compression pour les petits paquets
        return data.to_vec();
    }

    match algorithm {
        CompressionAlgorithm::Zlib => {
            // Niveau 6 = bon compromis vitesse/ratio
            zlib_compress(data, 6)
        }
        CompressionAlgorithm::Snappy => {
            // Plus rapide, moins compressif
            snappy_compress(data)
        }
    }
}
```

#### Batching des paquets

```rust
// Regrouper les paquets sortants par joueur
pub struct PacketBatcher {
    pending: HashMap<PlayerId, Vec<GamePacket>>,
    max_batch_size: usize,
}

impl PacketBatcher {
    pub fn queue(&mut self, player: PlayerId, packet: GamePacket) {
        self.pending.entry(player).or_default().push(packet);
    }

    pub fn flush(&mut self) -> Vec<(PlayerId, Vec<u8>)> {
        let mut batches = Vec::new();
        for (player, packets) in self.pending.drain() {
            // Sérialiser tous les paquets en un batch
            let batch = encode_batch(&packets);
            batches.push((player, batch));
        }
        batches
    }
}
```

#### Chunk cache avec blob protocol

```rust
// Cache de blobs de chunks pour le protocol client cache
pub struct BlobCache {
    blobs: HashMap<u64, Arc<Vec<u8>>>,  // hash -> data
    cache_size: usize,
    max_cache_size: usize,
}

impl BlobCache {
    pub fn get_or_insert(&mut self, data: &[u8]) -> u64 {
        let hash = xxhash64(data);
        if !self.blobs.contains_key(&hash) {
            self.blobs.insert(hash, Arc::new(data.to_vec()));
            self.cache_size += data.len();
            self.evict_if_needed();
        }
        hash
    }
}
```

### Chunks

#### Génération parallèle

```rust
use rayon::prelude::*;

pub struct ChunkGenerator {
    thread_pool: rayon::ThreadPool,
    pending: VecDeque<ChunkGenRequest>,
    results: mpsc::Receiver<ChunkGenResult>,
}

impl ChunkGenerator {
    pub fn new(threads: usize) -> Self {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|i| format!("chunk-gen-{}", i))
            .build()
            .unwrap();
        // ...
    }

    pub fn generate_batch(&self, positions: &[ChunkPos], seed: i64) {
        // Générer plusieurs chunks en parallèle
        positions.par_iter().for_each(|pos| {
            let chunk = generate_chunk(*pos, seed);
            self.results_tx.send(ChunkGenResult { pos: *pos, chunk }).ok();
        });
    }
}
```

#### Sérialisation chunk parallèle

```rust
// Sérialiser les chunks pour l'envoi réseau en parallèle
pub fn serialize_chunks_for_network(
    chunks: &[&ChunkColumn],
    runtime_palette: &BlockPalette,
) -> Vec<Vec<u8>> {
    chunks.par_iter().map(|chunk| {
        let mut buffer = Vec::with_capacity(16384);
        chunk.serialize_network(&mut buffer, runtime_palette);
        buffer
    }).collect()
}
```

#### Chunk prioritization

```rust
/// Trier les chunks à envoyer par distance au joueur (plus proches d'abord)
pub fn prioritize_chunks(
    player_pos: ChunkPos,
    pending: &mut VecDeque<ChunkPos>,
) {
    let mut chunks: Vec<ChunkPos> = pending.drain(..).collect();
    chunks.sort_by_key(|c| {
        let dx = (c.x - player_pos.x) as i64;
        let dz = (c.z - player_pos.z) as i64;
        dx * dx + dz * dz  // Distance au carré (pas besoin de sqrt)
    });
    pending.extend(chunks);
}
```

#### Envoi progressif des chunks

```rust
// Ne pas envoyer tous les chunks d'un coup (flood réseau)
const MAX_CHUNKS_PER_TICK: usize = 4;  // Par joueur

pub fn send_pending_chunks(player: &mut Player) {
    let mut sent = 0;
    while sent < MAX_CHUNKS_PER_TICK {
        if let Some(pos) = player.chunk_loader.pending_chunks.pop_front() {
            if let Some(chunk) = world.get_chunk(pos) {
                send_level_chunk_packet(player, chunk);
                player.chunk_loader.loaded_chunks.insert(pos);
                sent += 1;
            }
        } else {
            break;
        }
    }
}
```

### Entités

#### Spatial indexing

```rust
use std::collections::HashMap;

/// Index spatial pour les requêtes de proximité
pub struct SpatialIndex {
    // Grille de cellules, chaque cellule = 16×16 blocs (taille d'un chunk)
    cells: HashMap<(i32, i32), Vec<Entity>>,
    cell_size: f64,
}

impl SpatialIndex {
    pub fn new(cell_size: f64) -> Self {
        Self { cells: HashMap::new(), cell_size }
    }

    pub fn insert(&mut self, entity: Entity, pos: &Position) {
        let cell = self.pos_to_cell(pos);
        self.cells.entry(cell).or_default().push(entity);
    }

    pub fn query_radius(&self, center: &Position, radius: f64) -> Vec<Entity> {
        let mut results = Vec::new();
        let cell_radius = (radius / self.cell_size).ceil() as i32;
        let center_cell = self.pos_to_cell(center);

        for dx in -cell_radius..=cell_radius {
            for dz in -cell_radius..=cell_radius {
                let cell = (center_cell.0 + dx, center_cell.1 + dz);
                if let Some(entities) = self.cells.get(&cell) {
                    for &entity in entities {
                        // Vérifier la distance exacte
                        results.push(entity);
                    }
                }
            }
        }
        results
    }

    fn pos_to_cell(&self, pos: &Position) -> (i32, i32) {
        (
            (pos.x / self.cell_size).floor() as i32,
            (pos.z / self.cell_size).floor() as i32,
        )
    }
}
```

#### View distance culling

```rust
/// N'envoyer les mises à jour d'entité qu'aux joueurs dans le range
pub fn broadcast_entity_update(
    entity_pos: &Position,
    packet: &impl GamePacket,
    players: &Query<(&NetworkSession, &Position, &ChunkLoader), With<Player>>,
) {
    let entity_chunk = ChunkPos::from_block(entity_pos);

    for (session, player_pos, loader) in players.iter() {
        // Vérifier si le chunk de l'entité est chargé par ce joueur
        if loader.loaded_chunks.contains(&entity_chunk) {
            session.send(packet);
        }
    }
}
```

#### Entity ticking optimization

```rust
// Ne pas ticker toutes les entités à chaque tick
pub fn should_tick_entity(entity_pos: &Position, players: &[Position]) -> bool {
    // Simulation distance check
    let nearest = players.iter()
        .map(|p| p.distance_squared(entity_pos))
        .min_by(|a, b| a.partial_cmp(b).unwrap());

    match nearest {
        Some(dist_sq) if dist_sq <= (SIMULATION_DISTANCE as f64 * 16.0).powi(2) => true,
        _ => false,
    }
}
```

### Mémoire

#### Chunk unloading

```rust
pub struct ChunkManager {
    loaded_chunks: HashMap<ChunkPos, ChunkColumn>,
    unload_timer: HashMap<ChunkPos, u64>,  // Tick depuis dernier accès
}

impl ChunkManager {
    const UNLOAD_DELAY_TICKS: u64 = 600; // 30 secondes

    pub fn tick(&mut self, current_tick: u64, player_positions: &[ChunkPos]) {
        // Marquer les chunks hors de portée de tout joueur
        let active_chunks: HashSet<ChunkPos> = player_positions.iter()
            .flat_map(|p| p.chunks_in_radius(VIEW_DISTANCE))
            .collect();

        // Décharger les chunks inactifs
        let to_unload: Vec<ChunkPos> = self.loaded_chunks.keys()
            .filter(|pos| !active_chunks.contains(pos))
            .filter(|pos| {
                let timer = self.unload_timer.entry(**pos).or_insert(current_tick);
                current_tick - *timer > Self::UNLOAD_DELAY_TICKS
            })
            .copied()
            .collect();

        for pos in to_unload {
            if let Some(chunk) = self.loaded_chunks.remove(&pos) {
                if chunk.dirty {
                    self.save_chunk_async(&pos, chunk);
                }
            }
            self.unload_timer.remove(&pos);
        }
    }
}
```

#### Palette compression

```rust
// Compacter la palette des sub-chunks pour utiliser moins de bits
impl SubChunk {
    pub fn compact_palette(&mut self) {
        for layer in &mut self.layers {
            // Trouver les entrées de palette réellement utilisées
            let used: HashSet<u16> = layer.blocks.iter_indices().collect();

            if used.len() < layer.palette.len() {
                // Recréer la palette avec uniquement les entrées utilisées
                let mut new_palette = Vec::new();
                let mut remap = HashMap::new();

                for (old_idx, state) in layer.palette.iter().enumerate() {
                    if used.contains(&(old_idx as u16)) {
                        remap.insert(old_idx as u16, new_palette.len() as u16);
                        new_palette.push(state.clone());
                    }
                }

                // Recalculer bits_per_block
                let new_bits = bits_needed(new_palette.len());
                layer.reindex(&remap, new_bits);
                layer.palette = new_palette;
            }
        }
    }
}
```

### CPU

#### SIMD pour la génération de bruit

```rust
use simdnoise::NoiseBuilder;

// Générer le terrain d'un chunk entier avec SIMD
pub fn generate_terrain_simd(chunk_x: i32, chunk_z: i32, seed: i32) -> Vec<f32> {
    let (noise, _, _) = NoiseBuilder::fbm_3d_offset(
        chunk_x as f32 * 16.0,
        16, // width
        0.0,
        256, // height
        chunk_z as f32 * 16.0,
        16, // depth
    )
    .with_seed(seed)
    .with_octaves(6)
    .with_freq(0.01)
    .with_lacunarity(2.0)
    .with_gain(0.5)
    .generate();

    noise
}
```

#### Tick scheduling intelligent

```rust
// Répartir le travail sur plusieurs ticks pour lisser la charge
pub struct TickScheduler {
    pending: BinaryHeap<Reverse<(u64, ScheduledTask)>>,
}

impl TickScheduler {
    pub fn schedule(&mut self, target_tick: u64, task: ScheduledTask) {
        self.pending.push(Reverse((target_tick, task)));
    }

    pub fn process(&mut self, current_tick: u64, max_tasks: usize) -> usize {
        let mut processed = 0;
        while processed < max_tasks {
            if let Some(Reverse((tick, _))) = self.pending.peek() {
                if *tick > current_tick { break; }
            } else {
                break;
            }
            let Reverse((_, task)) = self.pending.pop().unwrap();
            task.execute();
            processed += 1;
        }
        processed
    }
}
```

## Métriques et monitoring

### Métriques à tracker

```rust
pub struct ServerMetrics {
    // Tick
    pub tick_times: RingBuffer<Duration, 100>,  // 100 derniers ticks
    pub current_tps: f64,
    pub tick_overruns: u64,

    // Réseau
    pub packets_in_per_second: u64,
    pub packets_out_per_second: u64,
    pub bytes_in_per_second: u64,
    pub bytes_out_per_second: u64,

    // Joueurs
    pub online_players: u32,
    pub max_players_seen: u32,

    // Chunks
    pub loaded_chunks: u32,
    pub chunks_generated: u64,
    pub chunk_gen_avg_ms: f64,

    // Entités
    pub total_entities: u32,
    pub entities_ticked: u32,

    // Mémoire
    pub heap_used: usize,
    pub chunk_cache_size: usize,
}
```

### Commandes de debug

```
/tps                    → TPS actuel, moyen (1m, 5m, 15m)
/timings                → Temps par système ECS
/gc                     → Forcer le nettoyage des chunks
/debug chunks           → Nombre de chunks chargés par monde
/debug entities         → Nombre d'entités par type
/debug network          → Stats réseau par joueur
/debug memory           → Utilisation mémoire détaillée
```

## Benchmarks de référence

### Cibles par opération

| Opération | Cible | Notes |
|-----------|-------|-------|
| Sérialiser un chunk | < 1ms | Palette + packed array |
| Décompresser un paquet (zlib) | < 0.1ms | Pour paquets typiques |
| Chiffrer/déchiffrer (AES) | < 0.01ms | AES-NI si disponible |
| Parse d'un paquet moyen | < 0.05ms | Désérialisation |
| Tick d'un mob (AI + physics) | < 0.1ms | Par entité |
| Block tick (random) | < 0.01ms | Par bloc tické |
| Requête spatiale (50 entités) | < 0.1ms | Avec spatial index |
| Pathfinding A* (50 blocs) | < 1ms | Chemin court |
| Génération d'un chunk | < 100ms | Terrain + features |
| Sauvegarde d'un chunk (LevelDB) | < 5ms | Écriture batch |

## Configuration performance

```toml
# server.toml
[performance]
# Nombre de threads pour la génération de chunks
chunk_gen_threads = 4

# Nombre max de chunks à envoyer par joueur par tick
max_chunks_per_tick = 4

# Simulation distance (chunks)
simulation_distance = 4

# View distance max autorisée
max_view_distance = 16

# Intervalle de sauvegarde auto (ticks)
autosave_interval = 6000  # 5 minutes

# Chunks max en mémoire
max_loaded_chunks = 10000

# Compression
compression_algorithm = "zlib"  # "zlib", "snappy", "none"
compression_threshold = 256     # Taille min pour compresser (octets)
compression_level = 6           # 1-9 pour zlib

# Entités
entity_activation_range = 32    # Blocs
max_entities_per_chunk = 50

# Network
max_players = 500
max_packets_per_second = 200
network_compression_threshold = 1

[performance.tick]
# Budget max par tick (ms) avant warning
tick_warning_threshold = 40
# Si un tick dépasse ce budget, reporter du travail au prochain
tick_max_budget = 48
```
