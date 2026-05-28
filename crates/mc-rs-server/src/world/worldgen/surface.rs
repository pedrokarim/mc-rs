//! Phase C-full — interpréteur de surface rules vanilla.
//!
//! Porte le système de surface vanilla (`noise_settings/overworld.json` →
//! `surface_rule`) : moteur d'exécution par colonne (haut→bas, suivi de
//! `stoneDepth`, `waterHeight`) + arbre de règles/conditions évalué verbatim.
//! Réf. d'exécution : `SurfaceSystem.ts` (deepslate), fidèle au vanilla.
//!
//! Conditions implémentées : `above_preliminary_surface`, `biome`, `not`,
//! `stone_depth` (avec `surface_secondary`), `vertical_gradient`, `water`,
//! `y_above`, `noise_threshold`, `hole`. Règles : `sequence`, `condition`,
//! `block`.
//!
//! Simplifications assumées (TODO, documentées) :
//! - `vertical_gradient` : cutoff déterministe au `true_at_and_below` (sans la
//!   bande probabiliste vanilla, qui demanderait un RNG positionnel `at`). Donne
//!   bedrock à y=-64 et deepslate à y≤0, comme la version générique précédente.
//! - terme aléatoire ±0.25 de `surfaceDepth` omis (même raison).
//! - `min_surface_level` approximé par le bloc solide le plus haut de la colonne
//!   (proxy de `preliminary_surface_level`) + `surfaceDepth − 8`.
//! - `temperature` (neige), `steep` (pentes), `bandlands` (terracotta badlands)
//!   stubbés → ces touches biome-spécifiques arriveront ensuite.

use std::sync::{LazyLock, Mutex};

use serde_json::Value;

use super::super::block_registry::BLOCKS;
use super::data;
use super::noise_chunk::{grid_index, MAX_Y, MIN_Y};
use super::perlin::NormalNoise;
use super::rng::XoroshiroRandom;

/// Plus haut Y constructible (exclusif − 1).
const TOP_Y: i32 = MAX_Y - 1;

/// Ancre verticale vanilla, résolue en Y absolu.
fn anchor_y(v: &Value) -> i32 {
    if let Some(n) = v.get("absolute").and_then(Value::as_i64) {
        n as i32
    } else if let Some(n) = v.get("above_bottom").and_then(Value::as_i64) {
        MIN_Y + n as i32
    } else if let Some(n) = v.get("below_top").and_then(Value::as_i64) {
        TOP_Y - n as i32
    } else {
        panic!("ancre verticale inconnue: {v}");
    }
}

#[inline]
fn map_range(v: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
    c + (v - a) * (d - c) / (b - a)
}

/// Résout un nom de bloc Java (des surface rules) en ID runtime Bedrock, en
/// gérant les quelques noms qui diffèrent entre Java et Bedrock.
fn resolve_block(java_name: &str) -> u32 {
    let bedrock = match java_name {
        "minecraft:terracotta" => "minecraft:hardened_clay",
        "minecraft:snow_block" => "minecraft:snow",
        other => other,
    };
    BLOCKS.get(bedrock)
}

enum Cond {
    AbovePreliminarySurface,
    Biome(Vec<String>),
    Not(Box<Cond>),
    StoneDepth {
        offset: i32,
        add_surface_depth: bool,
        secondary_range: i32,
        ceiling: bool,
    },
    /// Cutoff déterministe : vrai si `y <= true_below`.
    VerticalGradient {
        true_below: i32,
    },
    Water {
        offset: i32,
        mult: i32,
        add_stone_depth: bool,
    },
    YAbove {
        anchor: i32,
        mult: i32,
        add_stone_depth: bool,
    },
    NoiseThreshold {
        noise: NormalNoise,
        min: f64,
        max: f64,
    },
    Hole,
    /// `coldEnoughToSnow` vanilla. Approximé par l'appartenance du biome à
    /// l'ensemble des biomes froids (base temp < 0.15) ; l'ajustement de
    /// température par l'altitude (terme de bruit) est négligé.
    Temperature,
    /// Pente raide : différence de hauteur de surface ≥ 4 avec un voisin.
    Steep,
    /// Type de condition non géré.
    AlwaysFalse,
}

/// Biomes froids (base temperature < 0.15) → `coldEnoughToSnow` vrai.
const COLD_BIOMES: &[&str] = &[
    "minecraft:snowy_plains",
    "minecraft:ice_spikes",
    "minecraft:snowy_taiga",
    "minecraft:snowy_beach",
    "minecraft:grove",
    "minecraft:snowy_slopes",
    "minecraft:frozen_peaks",
    "minecraft:jagged_peaks",
    "minecraft:frozen_ocean",
    "minecraft:deep_frozen_ocean",
    "minecraft:frozen_river",
];

enum Rule {
    Block(u32),
    Sequence(Vec<Rule>),
    Condition(Cond, Box<Rule>),
    /// `bandlands` (terracotta) non porté, ou type inconnu.
    Noop,
}

/// Contexte d'évaluation pour une position donnée.
struct Ctx<'a> {
    block_x: i32,
    block_y: i32,
    block_z: i32,
    stone_depth_above: i32,
    stone_depth_below: i32,
    water_height: i32,
    surface_depth: f64,
    surface_secondary: f64,
    min_surface_level: i32,
    biome: &'a str,
    steep: bool,
}

impl Cond {
    fn test(&self, c: &Ctx) -> bool {
        match self {
            Cond::AbovePreliminarySurface => c.block_y >= c.min_surface_level,
            Cond::Biome(set) => set.iter().any(|b| b == c.biome),
            Cond::Not(inner) => !inner.test(c),
            Cond::StoneDepth {
                offset,
                add_surface_depth,
                secondary_range,
                ceiling,
            } => {
                let depth = if *ceiling {
                    c.stone_depth_below
                } else {
                    c.stone_depth_above
                };
                let sd = if *add_surface_depth {
                    c.surface_depth
                } else {
                    0.0
                };
                let secondary = if *secondary_range == 0 {
                    0.0
                } else {
                    map_range(c.surface_secondary, -1.0, 1.0, 0.0, *secondary_range as f64)
                };
                (depth as f64) <= 1.0 + *offset as f64 + sd + secondary
            }
            Cond::VerticalGradient { true_below } => c.block_y <= *true_below,
            Cond::Water {
                offset,
                mult,
                add_stone_depth,
            } => {
                if c.water_height == i32::MIN {
                    return true;
                }
                let sd_stone = if *add_stone_depth {
                    c.stone_depth_above
                } else {
                    0
                };
                (c.block_y + sd_stone) as f64
                    >= (c.water_height + offset) as f64 + c.surface_depth * *mult as f64
            }
            Cond::YAbove {
                anchor,
                mult,
                add_stone_depth,
            } => {
                let sd_stone = if *add_stone_depth {
                    c.stone_depth_above
                } else {
                    0
                };
                (c.block_y + sd_stone) as f64 >= *anchor as f64 + c.surface_depth * *mult as f64
            }
            Cond::NoiseThreshold { noise, min, max } => {
                let v = noise.get_value(c.block_x as f64, 0.0, c.block_z as f64);
                v >= *min && v <= *max
            }
            Cond::Hole => c.surface_depth <= 0.0,
            Cond::Temperature => COLD_BIOMES.contains(&c.biome),
            Cond::Steep => c.steep,
            Cond::AlwaysFalse => false,
        }
    }
}

fn run_rule(rule: &Rule, c: &Ctx) -> Option<u32> {
    match rule {
        Rule::Block(id) => Some(*id),
        Rule::Sequence(rules) => rules.iter().find_map(|r| run_rule(r, c)),
        Rule::Condition(cond, then) => {
            if cond.test(c) {
                run_rule(then, c)
            } else {
                None
            }
        }
        Rule::Noop => None,
    }
}

/// Système de surface compilé pour une seed (règles + bruits seedés).
pub struct SurfaceBuilder {
    rule: Rule,
    surface_noise: NormalNoise,
    surface_secondary_noise: NormalNoise,
    default_block: u32,
}

impl SurfaceBuilder {
    fn new(seed: u64) -> Self {
        let mut base = XoroshiroRandom::from_seed(seed);
        let deriver = base.fork_positional();
        let mk = |name: &str| {
            let params =
                data::noise_params(name).unwrap_or_else(|| panic!("bruit manquant: {name}"));
            let mut rng = deriver.from_hash_of(name);
            NormalNoise::create(&mut rng, &params)
        };
        let surface_noise = mk("minecraft:surface");
        let surface_secondary_noise = mk("minecraft:surface_secondary");

        let settings: Value =
            serde_json::from_str(data::noise_settings_json("minecraft:overworld").unwrap())
                .expect("settings JSON valide");
        let rule = parse_rule(&settings["surface_rule"], &deriver);

        SurfaceBuilder {
            rule,
            surface_noise,
            surface_secondary_noise,
            default_block: BLOCKS.stone,
        }
    }
}

fn parse_rule(v: &Value, deriver: &super::rng::PositionalRandomFactory) -> Rule {
    let t = v["type"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches("minecraft:");
    match t {
        "block" => Rule::Block(resolve_block(v["result_state"]["Name"].as_str().unwrap())),
        "sequence" => Rule::Sequence(
            v["sequence"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| parse_rule(e, deriver))
                .collect(),
        ),
        "condition" => Rule::Condition(
            parse_cond(&v["if_true"], deriver),
            Box::new(parse_rule(&v["then_run"], deriver)),
        ),
        // `bandlands` (terracotta des badlands) : approximation en terracotta
        // uniforme (les bandes colorées par Y restent à porter).
        "bandlands" => Rule::Block(resolve_block("minecraft:terracotta")),
        _ => Rule::Noop,
    }
}

fn parse_cond(v: &Value, deriver: &super::rng::PositionalRandomFactory) -> Cond {
    let t = v["type"]
        .as_str()
        .unwrap_or("")
        .trim_start_matches("minecraft:");
    match t {
        "above_preliminary_surface" => Cond::AbovePreliminarySurface,
        "biome" => Cond::Biome(
            v["biome_is"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| e.as_str().unwrap().to_string())
                .collect(),
        ),
        "not" => Cond::Not(Box::new(parse_cond(&v["invert"], deriver))),
        "stone_depth" => Cond::StoneDepth {
            offset: v["offset"].as_i64().unwrap_or(0) as i32,
            add_surface_depth: v["add_surface_depth"].as_bool().unwrap_or(false),
            secondary_range: v["secondary_depth_range"].as_i64().unwrap_or(0) as i32,
            ceiling: v["surface_type"].as_str() == Some("ceiling"),
        },
        "vertical_gradient" => Cond::VerticalGradient {
            true_below: anchor_y(&v["true_at_and_below"]),
        },
        "water" => Cond::Water {
            offset: v["offset"].as_i64().unwrap_or(0) as i32,
            mult: v["surface_depth_multiplier"].as_i64().unwrap_or(0) as i32,
            add_stone_depth: v["add_stone_depth"].as_bool().unwrap_or(false),
        },
        "y_above" => Cond::YAbove {
            anchor: anchor_y(&v["anchor"]),
            mult: v["surface_depth_multiplier"].as_i64().unwrap_or(0) as i32,
            add_stone_depth: v["add_stone_depth"].as_bool().unwrap_or(false),
        },
        "noise_threshold" => {
            let name = v["noise"].as_str().unwrap();
            let params =
                data::noise_params(name).unwrap_or_else(|| panic!("bruit manquant: {name}"));
            let mut rng = deriver.from_hash_of(name);
            Cond::NoiseThreshold {
                noise: NormalNoise::create(&mut rng, &params),
                min: v["min_threshold"].as_f64().unwrap(),
                max: v["max_threshold"].as_f64().unwrap(),
            }
        }
        "hole" => Cond::Hole,
        "temperature" => Cond::Temperature,
        "steep" => Cond::Steep,
        _ => Cond::AlwaysFalse,
    }
}

static BUILDER: LazyLock<Mutex<(u64, Option<SurfaceBuilder>)>> =
    LazyLock::new(|| Mutex::new((0, None)));

/// Applique les surface rules sur la grille (déjà remplie stone/water/air).
/// `biome_idx[lx][lz]` indexe `biome_names` (noms Java pour les conditions).
pub fn build(
    grid: &mut [u32],
    seed: u64,
    base_x: i32,
    base_z: i32,
    biome_idx: &[[u16; 16]; 16],
    biome_names: &[String],
    surfaces: &[[i32; 16]; 16],
) {
    let mut guard = BUILDER.lock().unwrap();
    if guard.1.is_none() || guard.0 != seed {
        *guard = (seed, Some(SurfaceBuilder::new(seed)));
    }
    let b = guard.1.as_ref().unwrap();

    for lx in 0..16usize {
        for lz in 0..16usize {
            let wx = base_x + lx as i32;
            let wz = base_z + lz as i32;

            // Pente raide (vanilla `steep`) : diff. de hauteur ≥ 4 avec un
            // voisin cardinal (hauteurs de colonne, bornées au chunk).
            let steep = {
                let zk = lz.saturating_sub(1);
                let zl = (lz + 1).min(15);
                if surfaces[lx][zl] >= surfaces[lx][zk] + 4 {
                    true
                } else {
                    let xo = lx.saturating_sub(1);
                    let xp = (lx + 1).min(15);
                    surfaces[xo][lz] >= surfaces[xp][lz] + 4
                }
            };
            let surface_depth = b.surface_noise.get_value(wx as f64, 0.0, wz as f64) * 2.75 + 3.0;
            let surface_secondary = b
                .surface_secondary_noise
                .get_value(wx as f64, 0.0, wz as f64);
            let biome = &biome_names[biome_idx[lx][lz] as usize];

            // Proxy de preliminary_surface_level : bloc solide le plus haut.
            let mut top = MIN_Y;
            for wy in (MIN_Y..MAX_Y).rev() {
                let blk = grid[grid_index(lx, wy, lz)];
                if blk != BLOCKS.air && blk != BLOCKS.water {
                    top = wy;
                    break;
                }
            }
            let min_surface_level = (top as f64 + surface_depth - 8.0).floor() as i32;

            // Scan haut→bas (port de SurfaceSystem.buildSurface).
            let mut stone_above = 0i32;
            let mut water_height = i32::MIN;
            let mut stone_offset = i32::MAX;

            for wy in (MIN_Y..MAX_Y).rev() {
                let i = grid_index(lx, wy, lz);
                let blk = grid[i];
                if blk == BLOCKS.air {
                    stone_above = 0;
                    water_height = i32::MIN;
                    continue;
                }
                if blk == BLOCKS.water {
                    if water_height == i32::MIN {
                        water_height = wy + 1;
                    }
                    continue;
                }
                if stone_offset >= wy {
                    stone_offset = i32::MIN;
                    for j in (MIN_Y..wy).rev() {
                        let s = grid[grid_index(lx, j, lz)];
                        if s == BLOCKS.air || s == BLOCKS.water {
                            stone_offset = j + 1;
                            break;
                        }
                    }
                }
                stone_above += 1;
                let stone_below = if stone_offset == i32::MIN {
                    wy - MIN_Y + 1
                } else {
                    wy - stone_offset + 1
                };

                if blk != b.default_block {
                    continue;
                }
                let ctx = Ctx {
                    block_x: wx,
                    block_y: wy,
                    block_z: wz,
                    stone_depth_above: stone_above,
                    stone_depth_below: stone_below,
                    water_height,
                    surface_depth,
                    surface_secondary,
                    min_surface_level,
                    biome,
                    steep,
                };
                if let Some(id) = run_rule(&b.rule, &ctx) {
                    grid[i] = id;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::noise_chunk::{GRID_LEN, SEA_LEVEL};
    use super::*;

    fn fill_column(grid: &mut [u32], lx: usize, lz: usize, top: i32, underwater: bool) {
        for wy in MIN_Y..=top {
            grid[grid_index(lx, wy, lz)] = BLOCKS.stone;
        }
        if underwater {
            for wy in (top + 1)..=SEA_LEVEL {
                grid[grid_index(lx, wy, lz)] = BLOCKS.water;
            }
        }
    }

    #[test]
    fn all_result_blocks_resolve() {
        // Tous les blocs de surface vanilla doivent exister dans le registre.
        for name in [
            "minecraft:grass_block",
            "minecraft:dirt",
            "minecraft:sand",
            "minecraft:sandstone",
            "minecraft:gravel",
            "minecraft:bedrock",
            "minecraft:deepslate",
            "minecraft:coarse_dirt",
            "minecraft:terracotta",
            "minecraft:snow_block",
            "minecraft:mud",
            "minecraft:podzol",
            "minecraft:terracotta",
            "minecraft:snow_block",
            "minecraft:calcite",
            "minecraft:powder_snow",
            "minecraft:packed_ice",
            "minecraft:ice",
            "minecraft:mycelium",
            "minecraft:red_sand",
            "minecraft:red_sandstone",
            "minecraft:orange_terracotta",
            "minecraft:white_terracotta",
        ] {
            assert_ne!(resolve_block(name), BLOCKS.air, "bloc introuvable: {name}");
        }
    }

    #[test]
    fn builder_parses() {
        // Ne doit pas paniquer : tout l'arbre surface_rule parse + bruits seedés.
        let _ = SurfaceBuilder::new(42);
    }

    #[test]
    fn plains_column_grass_dirt_bedrock_deepslate() {
        let mut grid = vec![BLOCKS.air; GRID_LEN].into_boxed_slice();
        let top = 80;
        fill_column(&mut grid, 0, 0, top, false);
        let idx = [[0u16; 16]; 16];
        let names = vec!["minecraft:plains".to_string()];

        let surfaces = [[top; 16]; 16];
        build(&mut grid, 42, 0, 0, &idx, &names, &surfaces);

        assert_eq!(
            grid[grid_index(0, top, 0)],
            BLOCKS.grass_block,
            "sommet plains = grass"
        );
        assert_eq!(
            grid[grid_index(0, MIN_Y, 0)],
            BLOCKS.bedrock,
            "plancher = bedrock"
        );
        assert_eq!(
            grid[grid_index(0, 0, 0)],
            BLOCKS.deepslate,
            "y=0 = deepslate"
        );
    }

    #[test]
    fn desert_column_gets_sand() {
        let mut grid = vec![BLOCKS.air; GRID_LEN].into_boxed_slice();
        let top = 80;
        fill_column(&mut grid, 0, 0, top, false);
        let idx = [[0u16; 16]; 16];
        let names = vec!["minecraft:desert".to_string()];

        let surfaces = [[top; 16]; 16];
        build(&mut grid, 42, 0, 0, &idx, &names, &surfaces);

        // Le désert pose du sable en surface (condition biome activée).
        assert_eq!(
            grid[grid_index(0, top, 0)],
            BLOCKS.sand,
            "sommet désert = sable"
        );
    }
}
