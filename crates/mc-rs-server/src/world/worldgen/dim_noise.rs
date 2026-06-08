//! Générateur de chunk **paramétré par dimension** (Nether / End), réutilisant le
//! moteur de density functions vanilla.
//!
//! L'overworld a son propre générateur riche (`noise_chunk.rs` : biomes multi-noise,
//! surface rules, arbres…). Ici on factorise la forme commune — échantillonner
//! `final_density` aux coins des cellules, interpoler trilinéairement, placer
//! `default_block` (densité > 0) / fluide (sous le sea level) / air — pour les
//! dimensions dont seules les bornes Y, la taille de cellule et les blocs changent.
//! Le `final_density` provient du vrai `noise_settings/{nether,end}.json` vanilla,
//! donc la **forme du terrain est fidèle** (Nether en cloche + mer de lave ;
//! îles flottantes de l'End une fois `end_islands` implémenté).

use super::super::block_registry::BLOCKS;
use super::super::chunk_serializer;
use super::density::NoiseRouter;

/// Paramètres de génération d'une dimension via le moteur density.
pub struct DimConfig {
    pub min_y: i32,
    pub height: i32,
    pub sea_level: i32,
    /// Largeur d'une cellule horizontale = `size_horizontal * 4`.
    pub cell_w: i32,
    /// Hauteur d'une cellule = `size_vertical * 4`.
    pub cell_h: i32,
    pub default_block: u32,
    /// Fluide sous le sea level (`None` = pas de fluide, ex. End).
    pub default_fluid: Option<u32>,
    /// Biome Bedrock unique (v1 ; multi-biome = milestone suivant).
    pub biome_id: u32,
    /// Nombre de couches de bedrock au plancher et au plafond (0 = aucune).
    pub bedrock_floor: i32,
    pub bedrock_roof: i32,
}

/// Hash déterministe `(x,y,z,seed) -> [0,1)` pour le dégradé de bedrock (jagged).
fn hash01(x: i32, y: i32, z: i32, seed: u64) -> f64 {
    let mut h = seed
        ^ (x as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (y as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
        ^ (z as u64).wrapping_mul(0x1656_67B1_9E37_79F9);
    h ^= h >> 33;
    h = h.wrapping_mul(0xff51_afd7_ed55_8ccd);
    h ^= h >> 33;
    (h >> 11) as f64 / ((1u64 << 53) as f64)
}

#[inline]
fn lerp(t: f64, a: f64, b: f64) -> f64 {
    a + t * (b - a)
}

/// Génère un chunk de dimension : renvoie `(sub_chunk_count, payload réseau)`,
/// même contrat que les autres générateurs.
pub fn generate_dim_chunk(
    chunk_x: i32,
    chunk_z: i32,
    seed: u64,
    router: &NoiseRouter,
    cfg: &DimConfig,
) -> (u32, Vec<u8>) {
    let base_x = chunk_x * 16;
    let base_z = chunk_z * 16;
    let max_y = cfg.min_y + cfg.height;
    let sub_count = (cfg.height / 16) as usize;
    let cells_xz = (16 / cfg.cell_w) as usize;
    let cells_y = (cfg.height / cfg.cell_h) as usize;
    let nx = cells_xz + 1;
    let ny = cells_y + 1;

    // Coins de cellule : densité échantillonnée à `final_density`.
    let fd = &router.final_density;
    let mut corners = vec![0.0f64; nx * ny * nx];
    let cidx = |cx: usize, cy: usize, cz: usize| (cx * ny + cy) * nx + cz;
    for cx in 0..nx {
        let wx = base_x + cx as i32 * cfg.cell_w;
        for cy in 0..ny {
            let wy = cfg.min_y + cy as i32 * cfg.cell_h;
            for cz in 0..nx {
                let wz = base_z + cz as i32 * cfg.cell_w;
                corners[cidx(cx, cy, cz)] = fd.compute(wx, wy, wz);
            }
        }
    }

    // Densité interpolée trilinéairement en un bloc local.
    let density_at = |lx: usize, wy: i32, lz: usize| -> f64 {
        let cx = lx / cfg.cell_w as usize;
        let cz = lz / cfg.cell_w as usize;
        let cy = ((wy - cfg.min_y) / cfg.cell_h) as usize;
        let dx = (lx % cfg.cell_w as usize) as f64 / cfg.cell_w as f64;
        let dz = (lz % cfg.cell_w as usize) as f64 / cfg.cell_w as f64;
        let dy = ((wy - cfg.min_y) % cfg.cell_h) as f64 / cfg.cell_h as f64;
        let v000 = corners[cidx(cx, cy, cz)];
        let v100 = corners[cidx(cx + 1, cy, cz)];
        let v010 = corners[cidx(cx, cy + 1, cz)];
        let v110 = corners[cidx(cx + 1, cy + 1, cz)];
        let v001 = corners[cidx(cx, cy, cz + 1)];
        let v101 = corners[cidx(cx + 1, cy, cz + 1)];
        let v011 = corners[cidx(cx, cy + 1, cz + 1)];
        let v111 = corners[cidx(cx + 1, cy + 1, cz + 1)];
        let v00 = lerp(dx, v000, v100);
        let v10 = lerp(dx, v010, v110);
        let v01 = lerp(dx, v001, v101);
        let v11 = lerp(dx, v011, v111);
        lerp(dz, lerp(dy, v00, v10), lerp(dy, v01, v11))
    };

    let bedrock = BLOCKS.bedrock;
    // Bedrock jagged : couche du bord toujours pleine, puis probabilité dégressive.
    let is_bedrock = |wx: i32, wy: i32, wz: i32| -> bool {
        if cfg.bedrock_floor > 0 {
            let h = wy - cfg.min_y; // 0 = plancher
            if h < cfg.bedrock_floor
                && hash01(wx, wy, wz, seed ^ 0xb3d_0c4)
                    < (cfg.bedrock_floor - h) as f64 / cfg.bedrock_floor as f64
            {
                return true;
            }
        }
        if cfg.bedrock_roof > 0 {
            let h = (max_y - 1) - wy; // 0 = plafond
            if h < cfg.bedrock_roof
                && hash01(wx, wy, wz, seed ^ 0x12_face)
                    < (cfg.bedrock_roof - h) as f64 / cfg.bedrock_roof as f64
            {
                return true;
            }
        }
        false
    };

    let mut payload = Vec::with_capacity(8192);
    for sub_idx in 0..sub_count {
        let sub_y_start = cfg.min_y + sub_idx as i32 * 16;
        let mut blocks = [0u32; 4096];
        let mut palette: Vec<u32> = vec![BLOCKS.air];
        let mut has_blocks = false;

        for lx in 0..16usize {
            for lz in 0..16usize {
                let wx = base_x + lx as i32;
                let wz = base_z + lz as i32;
                for ly in 0..16usize {
                    let wy = sub_y_start + ly as i32;
                    let block = if is_bedrock(wx, wy, wz) {
                        bedrock
                    } else if density_at(lx, wy, lz) > 0.0 {
                        cfg.default_block
                    } else if wy <= cfg.sea_level {
                        match cfg.default_fluid {
                            Some(f) => f,
                            None => continue,
                        }
                    } else {
                        continue;
                    };
                    let pidx = palette.iter().position(|&b| b == block).unwrap_or_else(|| {
                        palette.push(block);
                        palette.len() - 1
                    });
                    blocks[(lx << 8) | (lz << 4) | ly] = pidx as u32;
                    has_blocks = true;
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

    // Biomes : un seul biome pour toute la dimension (v1), une section par sub-chunk.
    let cells = [cfg.biome_id; 64];
    for _ in 0..sub_count {
        payload.extend_from_slice(&chunk_serializer::serialize_biome_section(&cells));
    }
    payload.push(0); // border blocks count

    (sub_count as u32, payload)
}
