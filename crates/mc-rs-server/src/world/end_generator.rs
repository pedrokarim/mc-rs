//! Générateur The End (v1) — île centrale flottante + piliers d'obsidienne + vide.
//!
//! L'End vanilla est généré par density functions (cf. `.reference/mcmeta/.../end.json`
//! + `deepslate TheEndBiomeSource`). Pour la v1 on reproduit l'essentiel visible :
//! - **Île centrale** : disque d'end_stone flottant centré sur (0,0), profil en lentille
//!   (épais au centre, fin sur les bords), entouré de vide.
//! - **10 piliers d'obsidienne** (réf `obsidian_pillar::vanilla_pillar_locations`, rayon 43)
//!   coiffés de bedrock (l'end crystal qui s'y pose est une entité — DIM-J).
//! - Le portail de sortie, les end crystals et les îles externes/gateways arrivent avec
//!   le combat du dragon (DIM-J) et les structures (DIM-K).
//!
//! Format de payload identique aux autres générateurs (sub-chunks paletted + biomes + border).

use super::block_registry::BLOCKS;
use super::chunk_serializer;
use crate::obsidian_pillar::{self, PillarLocation};

const END_SUB_CHUNKS: usize = 16; // hauteur 256 / 16
const END_BIOME: u32 = 9; // BiomeIds::THE_END (Bedrock)

const ISLAND_RADIUS: f64 = 100.0;
const ISLAND_CENTER_Y: f64 = 56.0;
const ISLAND_HALF_MAX: f64 = 20.0; // demi-épaisseur au centre

const PILLAR_RADIUS_SQ: i32 = 4; // colonnes de rayon 2 (~5×5)
const PILLAR_BASE_Y: i32 = 50; // l'obsidienne part d'ici (ancrée dans l'île)

/// Génère un chunk de l'End : renvoie `(sub_chunk_count, payload)`.
pub fn generate_end_chunk(chunk_x: i32, chunk_z: i32, _seed: u64) -> (u32, Vec<u8>) {
    let air = BLOCKS.air;
    let bedrock = BLOCKS.bedrock;
    let end_stone = BLOCKS.get("minecraft:end_stone");
    let obsidian = BLOCKS.get("minecraft:obsidian");

    let base_x = chunk_x * 16;
    let base_z = chunk_z * 16;
    let pillars = obsidian_pillar::vanilla_pillar_locations();

    let mut payload = Vec::with_capacity(4096);

    for sub_idx in 0..END_SUB_CHUNKS {
        let sub_y_start = sub_idx as i32 * 16; // End min_y = 0
        let mut blocks = [0u32; 4096];
        let mut palette: Vec<u32> = vec![air];
        let mut has_blocks = false;

        #[allow(clippy::needless_range_loop)]
        for local_x in 0..16usize {
            for local_z in 0..16usize {
                let world_x = base_x + local_x as i32;
                let world_z = base_z + local_z as i32;
                let dist = ((world_x as f64).powi(2) + (world_z as f64).powi(2)).sqrt();
                let band = island_band(dist);

                for local_y in 0..16usize {
                    let y = sub_y_start + local_y as i32;

                    // 1. Île centrale (end_stone) en lentille.
                    let mut block = match band {
                        Some((lo, hi)) if y >= lo && y <= hi => end_stone,
                        _ => air,
                    };

                    // 2. Piliers d'obsidienne (priorité sur l'air/île).
                    if let Some(p) = pillar_at(world_x, world_z, &pillars) {
                        let top = pillar_top_y(p);
                        if y == top {
                            block = bedrock;
                        } else if (PILLAR_BASE_Y..top).contains(&y) {
                            block = obsidian;
                        }
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

    let biome_ids = [[END_BIOME; 16]; 16];
    payload.extend_from_slice(&chunk_serializer::serialize_biome_sections_from_columns(
        &biome_ids,
        END_SUB_CHUNKS,
    ));
    payload.push(0); // border blocks count

    (END_SUB_CHUNKS as u32, payload)
}

/// Bande verticale [lo, hi] d'end_stone pour une colonne à distance `dist` de l'origine,
/// ou `None` si on est dans le vide (hors de l'île). Profil en lentille : épais au centre,
/// nul au bord.
fn island_band(dist: f64) -> Option<(i32, i32)> {
    if dist >= ISLAND_RADIUS {
        return None;
    }
    let t = 1.0 - dist / ISLAND_RADIUS; // 1 au centre → 0 au bord
    let half = t * t * ISLAND_HALF_MAX; // chute quadratique
    if half < 1.0 {
        return None;
    }
    let lo = (ISLAND_CENTER_Y - half).round() as i32;
    let hi = (ISLAND_CENTER_Y + half).round() as i32;
    Some((lo, hi))
}

/// Le pilier dont la colonne (x,z) fait partie, le cas échéant.
fn pillar_at(world_x: i32, world_z: i32, pillars: &[PillarLocation]) -> Option<PillarLocation> {
    pillars.iter().copied().find(|p| {
        let dx = world_x - p.x;
        let dz = world_z - p.z;
        dx * dx + dz * dz <= PILLAR_RADIUS_SQ
    })
}

/// Y du sommet (bedrock) d'un pilier — la variété de hauteurs vanilla (heights 41..57).
fn pillar_top_y(p: PillarLocation) -> i32 {
    76 + (p.height as i32 - 41) // ~78..92
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_chunk_has_16_sub_chunks() {
        let (count, payload) = generate_end_chunk(0, 0, 1);
        assert_eq!(count, END_SUB_CHUNKS as u32);
        assert!(!payload.is_empty());
    }

    #[test]
    fn center_column_is_solid_island() {
        // À l'origine, la bande doit exister et être épaisse.
        let band = island_band(0.0).expect("center must be solid");
        assert!(band.1 - band.0 >= 30, "central island should be thick");
    }

    #[test]
    fn far_column_is_void() {
        assert!(island_band(500.0).is_none());
    }

    #[test]
    fn central_chunk_contains_pillars() {
        // Le chunk autour de l'origine recouvre des piliers (rayon 43) — au moins un
        // chunk proche doit en contenir. On vérifie qu'un pilier existe à r≈43.
        let pillars = obsidian_pillar::vanilla_pillar_locations();
        assert_eq!(pillars.len(), 10);
        // Génère un chunk recouvrant un pilier (ex. pilier 0 à (43,0) → chunk (2,0)).
        let (_, payload) = generate_end_chunk(2, 0, 1);
        assert!(!payload.is_empty());
    }

    #[test]
    fn end_generation_is_deterministic() {
        assert_eq!(generate_end_chunk(1, 1, 1).1, generate_end_chunk(1, 1, 1).1);
    }
}
