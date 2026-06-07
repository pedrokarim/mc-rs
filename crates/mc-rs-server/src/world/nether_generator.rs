//! Générateur Nether — port fidèle de PMMP `src/world/generator/hell/Nether.php`.
//!
//! Algorithme (identique à PMMP) :
//! - Bruit `Simplex(seed, 4 octaves, persistence 1/4, expansion 1/64)`, échantillonné
//!   en 3D via `get_fast_noise_3d(16, 128, 16, 4, 8, 4, ...)` → `noise[x][z][y]`.
//! - Dimension haute de 128 (Y 0..127), sol+toit bedrock (Y 0 et 127).
//! - `noiseValue = (|emptyHeight - y| / emptyHeight) * emptyAmplitude - noise[x][z][y] - (1-density)`
//!   avec emptyHeight=64, emptyAmplitude=1, density=0.5.
//!   - `> 0` → netherrack ; sinon si `y <= 32` → lave ; sinon air.
//! - Biome HELL partout (id 8).
//! - Ore : nether quartz (`minecraft:quartz_ore`) en veines dans la netherrack, Y 10..117.
//!
//! Format de payload identique à `terrain_generator` / `flat_generator`
//! (sub-chunks paletted + sections de biome + border byte).

use super::block_registry::BLOCKS;
use super::chunk_serializer;
use super::noise::Simplex;
use super::random::Random;

const NETHER_SUB_CHUNKS: usize = 8; // hauteur 128 / 16
const NETHER_HEIGHT: i32 = 128;
const WATER_HEIGHT: i32 = 32; // niveau de lave
const EMPTY_HEIGHT: f64 = 64.0;
const EMPTY_AMPLITUDE: f64 = 1.0;
const DENSITY: f64 = 0.5;
const HELL_BIOME: u32 = 8; // BiomeIds::HELL

/// Génère un chunk Nether : renvoie `(sub_chunk_count, payload)`.
pub fn generate_nether_chunk(chunk_x: i32, chunk_z: i32, seed: u64) -> (u32, Vec<u8>) {
    let air = BLOCKS.air;
    let bedrock = BLOCKS.bedrock;
    let lava = BLOCKS.lava;
    let netherrack = BLOCKS.get("minecraft:netherrack");
    let quartz_ore = BLOCKS.get("minecraft:quartz_ore");

    // Bruit de base — construit depuis la seed du monde (déterministe, comme le
    // `noiseBase` unique de PMMP). `Random::new(seed)` alimente la table de permutation.
    let mut world_random = Random::new(seed as i64);
    let noise_base = Simplex::new(&mut world_random, 4, 0.25, 1.0 / 64.0);

    let base_x = chunk_x * 16;
    let base_z = chunk_z * 16;
    let noise =
        noise_base.get_fast_noise_3d(16, NETHER_HEIGHT as usize, 16, 4, 8, 4, base_x, 0, base_z);

    // RNG par chunk (même graine que PMMP : 0xdeadbeef ^ (x<<8) ^ z ^ seed) pour les ores.
    let mut chunk_random =
        Random::new(0xdead_beef_i64 ^ ((chunk_x as i64) << 8) ^ chunk_z as i64 ^ seed as i64);
    let ore_positions = nether_quartz_positions(&mut chunk_random);

    let mut payload = Vec::with_capacity(8192);

    for sub_idx in 0..NETHER_SUB_CHUNKS {
        let sub_y_start = sub_idx as i32 * 16; // Nether min_y = 0
        let mut blocks = [0u32; 4096];
        let mut palette: Vec<u32> = vec![air];
        let mut has_blocks = false;

        #[allow(clippy::needless_range_loop)]
        for local_x in 0..16usize {
            for local_z in 0..16usize {
                for local_y in 0..16usize {
                    let y = sub_y_start + local_y as i32;

                    let mut block = if y == 0 || y == NETHER_HEIGHT - 1 {
                        bedrock
                    } else {
                        let noise_value = ((EMPTY_HEIGHT - y as f64).abs() / EMPTY_HEIGHT)
                            * EMPTY_AMPLITUDE
                            - noise[local_x][local_z][y as usize]
                            - (1.0 - DENSITY);
                        if noise_value > 0.0 {
                            netherrack
                        } else if y <= WATER_HEIGHT {
                            lava
                        } else {
                            air
                        }
                    };

                    // Ore : la netherrack devient quartz aux positions tirées.
                    if block == netherrack
                        && ore_positions.contains(&(local_x as u8, y, local_z as u8))
                    {
                        block = quartz_ore;
                    }

                    if block != air {
                        let idx = (local_x << 8) | (local_z << 4) | local_y;
                        let pidx = palette.iter().position(|&b| b == block).unwrap_or_else(|| {
                            palette.push(block);
                            palette.len() - 1
                        });
                        blocks[idx] = pidx as u32;
                        has_blocks = true;
                    }
                }
            }
        }

        if has_blocks {
            payload.extend_from_slice(&chunk_serializer::serialize_sub_chunk(&blocks, &palette));
        } else {
            payload.push(8); // version
            payload.push(0); // 0 storage layers
        }
    }

    // Sections de biome (HELL partout) — 8 sections pour le Nether.
    let biome_ids = [[HELL_BIOME; 16]; 16];
    payload.extend_from_slice(&chunk_serializer::serialize_biome_sections_from_columns(
        &biome_ids,
        NETHER_SUB_CHUNKS,
    ));

    payload.push(0); // border blocks count

    (NETHER_SUB_CHUNKS as u32, payload)
}

/// Positions (local_x, world_y, local_z) où de la netherrack devient quartz.
/// Approximation des veines de l'Ore populator PMMP : quelques petits clusters,
/// Y 10..117. Le remplacement n'a lieu que si le bloc cible est de la netherrack.
fn nether_quartz_positions(random: &mut Random) -> std::collections::HashSet<(u8, i32, u8)> {
    let mut set = std::collections::HashSet::new();
    // ~10 clusters par chunk (cf. OreType count=16/clusterSize=14 — version simplifiée).
    for _ in 0..10 {
        let cx = random.next_bounded_int(16) as u8;
        let cz = random.next_bounded_int(16) as u8;
        let cy = random.next_range(10, 117);
        // Petit cluster 3x3x3 autour du centre.
        let size = 3 + random.next_bounded_int(4); // 3..6 blocs
        for _ in 0..size {
            let dx = random.next_bounded_int(3) - 1;
            let dy = random.next_bounded_int(3) - 1;
            let dz = random.next_bounded_int(3) - 1;
            let x = cx as i32 + dx;
            let y = cy + dy;
            let z = cz as i32 + dz;
            if (0..16).contains(&x) && (10..=117).contains(&y) && (0..16).contains(&z) {
                set.insert((x as u8, y, z as u8));
            }
        }
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nether_chunk_has_8_sub_chunks() {
        let (count, payload) = generate_nether_chunk(0, 0, 12345);
        assert_eq!(count, NETHER_SUB_CHUNKS as u32);
        assert!(!payload.is_empty());
    }

    #[test]
    fn nether_generation_is_deterministic() {
        let a = generate_nether_chunk(3, -2, 999);
        let b = generate_nether_chunk(3, -2, 999);
        assert_eq!(a.0, b.0);
        assert_eq!(a.1, b.1, "same chunk+seed must produce identical payload");
    }

    #[test]
    fn different_chunks_differ() {
        let a = generate_nether_chunk(0, 0, 7);
        let b = generate_nether_chunk(10, 10, 7);
        // Très improbable que deux chunks éloignés aient le même payload.
        assert_ne!(a.1, b.1);
    }
}
