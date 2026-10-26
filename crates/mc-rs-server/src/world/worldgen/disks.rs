//! Features « disque » vanilla (`minecraft:disk`) : plaques de sable, d'argile et de gravier au fond de l'eau
//! (`disk_sand`, `disk_clay`, `disk_gravel`, dans presque tous les biomes de l'Overworld), herbe sur la boue des
//! marais de palétuviers (`disk_grass`), glace compacte sur la neige (`ice_patch`).
//!
//! Lues dans les données vanilla (`data/worldgen/{biome,placed_feature,configured_feature}`) : nombre par chunk,
//! rayon, demi-hauteur, blocs cibles, fournisseur d'état (règles puis repli) et filtre de placement. Sémantique
//! de vanilla `DiskFeature` : pour chaque colonne du disque (distance² ≤ r²), de `y + demi-hauteur` à
//! `y - demi-hauteur` inclus, chaque bloc cible est remplacé. Comme pour le reste de la décoration de mc-rs,
//! l'aléa n'est pas celui de vanilla : mêmes règles et mêmes densités, pas les mêmes positions.

use std::collections::HashMap;
use std::sync::LazyLock;

use serde_json::Value;

use super::super::block_registry::BLOCKS;
use super::super::random::Random;
use super::data;
use super::noise_chunk::{grid_index, MAX_Y, MIN_Y};

/// Prédicat de bloc vanilla (le sous-ensemble utilisé par les disques).
enum Predicate {
    MatchingBlocks { blocks: Vec<u32>, offset: [i32; 3] },
    MatchingWater { offset: [i32; 3] },
    Solid { offset: [i32; 3] },
    Not(Box<Predicate>),
    AnyOf(Vec<Predicate>),
}

/// Hauteur de référence d'un placement (`heightmap`).
#[derive(Clone, Copy)]
enum Heightmap {
    /// `OCEAN_FLOOR_WG` : au-dessus du plus haut bloc qui n'est ni air ni liquide.
    OceanFloor,
    /// `MOTION_BLOCKING` : au-dessus du plus haut bloc qui n'est pas de l'air (liquides compris).
    MotionBlocking,
}

struct Disk {
    count: i32,
    heightmap: Heightmap,
    y_offset: i32,
    filter: Predicate,
    radius: (i32, i32),
    half_height: i32,
    targets: Vec<u32>,
    rules: Vec<(Predicate, u32)>,
    fallback: u32,
}

struct Registry {
    disks: HashMap<String, Disk>,
    /// Disques de chaque biome (`minecraft:<biome>`), dans l'ordre de ses features.
    biome_disks: HashMap<String, Vec<String>>,
}

static REG: LazyLock<Registry> = LazyLock::new(build);

fn strip(s: &str) -> &str {
    s.strip_prefix("minecraft:").unwrap_or(s)
}

fn offset(v: &Value) -> [i32; 3] {
    let a = v.get("offset").and_then(Value::as_array);
    let at = |i: usize| {
        a.and_then(|a| a.get(i))
            .and_then(Value::as_i64)
            .unwrap_or(0) as i32
    };
    [at(0), at(1), at(2)]
}

fn block_list(v: &Value) -> Vec<u32> {
    match v {
        Value::String(s) => vec![super::surface::resolve_block(s)],
        Value::Array(a) => a
            .iter()
            .filter_map(Value::as_str)
            .map(super::surface::resolve_block)
            .collect(),
        _ => Vec::new(),
    }
}

fn predicate(v: &Value) -> Option<Predicate> {
    Some(match strip(v.get("type")?.as_str()?) {
        "matching_blocks" => Predicate::MatchingBlocks {
            blocks: block_list(v.get("blocks")?),
            offset: offset(v),
        },
        "matching_fluids" => {
            let fluids = v.get("fluids")?;
            let water = match fluids {
                Value::String(s) => strip(s) == "water",
                Value::Array(a) => a.iter().any(|f| f.as_str().map(strip) == Some("water")),
                _ => false,
            };
            if !water {
                return None;
            }
            Predicate::MatchingWater { offset: offset(v) }
        }
        "solid" => Predicate::Solid { offset: offset(v) },
        "not" => Predicate::Not(Box::new(predicate(v.get("predicate")?)?)),
        "any_of" => Predicate::AnyOf(
            v.get("predicates")?
                .as_array()?
                .iter()
                .map(predicate)
                .collect::<Option<Vec<_>>>()?,
        ),
        _ => return None,
    })
}

fn state(v: &Value) -> Option<u32> {
    if strip(v.get("type")?.as_str()?) != "simple_state_provider" {
        return None;
    }
    Some(super::surface::resolve_block(
        v.get("state")?.get("Name")?.as_str()?,
    ))
}

/// Disque lu dans les données ; `None` si une partie n'est pas reconnue (rien n'est alors posé).
fn parse(placed: &Value, configured: &Value) -> Option<Disk> {
    if strip(configured.get("type")?.as_str()?) != "disk" {
        return None;
    }
    let mut count = 1;
    let mut heightmap = None;
    let mut y_offset = 0;
    let mut filter = None;
    for m in placed.get("placement")?.as_array()? {
        match strip(m.get("type")?.as_str()?) {
            "count" => count = m.get("count")?.as_i64()? as i32,
            "in_square" | "biome" => {}
            "heightmap" => {
                heightmap = Some(match m.get("heightmap")?.as_str()? {
                    "OCEAN_FLOOR_WG" | "OCEAN_FLOOR" => Heightmap::OceanFloor,
                    "MOTION_BLOCKING" => Heightmap::MotionBlocking,
                    _ => return None,
                })
            }
            "random_offset" => {
                if m.get("xz_spread")?.as_i64()? != 0 {
                    return None;
                }
                y_offset = m.get("y_spread")?.as_i64()? as i32;
            }
            "block_predicate_filter" => filter = Some(predicate(m.get("predicate")?)?),
            _ => return None,
        }
    }
    let config = configured.get("config")?;
    let radius = config.get("radius")?;
    let radius = match radius {
        Value::Number(n) => {
            let r = n.as_i64()? as i32;
            (r, r)
        }
        _ if strip(radius.get("type")?.as_str()?) == "uniform" => (
            radius.get("min_inclusive")?.as_i64()? as i32,
            radius.get("max_inclusive")?.as_i64()? as i32,
        ),
        _ => return None,
    };
    let provider = config.get("state_provider")?;
    let mut rules = Vec::new();
    for rule in provider.get("rules")?.as_array()? {
        rules.push((predicate(rule.get("if_true")?)?, state(rule.get("then")?)?));
    }
    let target = config.get("target")?;
    if strip(target.get("type")?.as_str()?) != "matching_blocks" {
        return None;
    }
    Some(Disk {
        count,
        heightmap: heightmap?,
        y_offset,
        filter: filter?,
        radius,
        half_height: config.get("half_height")?.as_i64()? as i32,
        targets: block_list(target.get("blocks")?),
        rules,
        fallback: state(provider.get("fallback")?)?,
    })
}

fn build() -> Registry {
    let mut disks = HashMap::new();
    let mut biome_disks = HashMap::new();
    for biome in data::list_names("biome") {
        let Some(bio) = data::json_value("biome", &biome) else {
            continue;
        };
        let mut names = Vec::new();
        for id in bio
            .get("features")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_array)
            .flatten()
            .filter_map(Value::as_str)
        {
            let name = strip(id).to_string();
            if !disks.contains_key(&name) {
                let Some(placed) = data::json_value("placed_feature", &name) else {
                    continue;
                };
                let Some(feature) = placed.get("feature").and_then(Value::as_str) else {
                    continue;
                };
                let Some(configured) = data::json_value("configured_feature", strip(feature))
                else {
                    continue;
                };
                let Some(disk) = parse(&placed, &configured) else {
                    continue;
                };
                disks.insert(name.clone(), disk);
            }
            names.push(name);
        }
        biome_disks.insert(format!("minecraft:{biome}"), names);
    }
    Registry { disks, biome_disks }
}

fn at(grid: &[u32], lx: i32, y: i32, lz: i32) -> Option<u32> {
    if !(0..16).contains(&lx) || !(0..16).contains(&lz) || !(MIN_Y..MAX_Y).contains(&y) {
        return None;
    }
    Some(grid[grid_index(lx as usize, y, lz as usize)])
}

fn test(p: &Predicate, grid: &[u32], lx: i32, y: i32, lz: i32) -> bool {
    let shifted = |o: &[i32; 3]| at(grid, lx + o[0], y + o[1], lz + o[2]);
    match p {
        Predicate::MatchingBlocks { blocks, offset } => {
            shifted(offset).is_some_and(|b| blocks.contains(&b))
        }
        Predicate::MatchingWater { offset } => shifted(offset).is_some_and(is_water),
        // Approximation de `BlockState.isSolid` : ni air, ni liquide.
        Predicate::Solid { offset } => {
            shifted(offset).is_some_and(|b| b != BLOCKS.air && !is_water(b) && !is_lava(b))
        }
        Predicate::Not(inner) => !test(inner, grid, lx, y, lz),
        Predicate::AnyOf(all) => all.iter().any(|q| test(q, grid, lx, y, lz)),
    }
}

fn is_water(b: u32) -> bool {
    b == BLOCKS.water || BLOCKS.name_for(b) == Some("minecraft:flowing_water")
}

fn is_lava(b: u32) -> bool {
    matches!(
        BLOCKS.name_for(b),
        Some("minecraft:lava" | "minecraft:flowing_lava")
    )
}

/// Hauteur de placement d'une colonne (bloc juste au-dessus du sol selon `heightmap`).
fn height(grid: &[u32], lx: i32, lz: i32, heightmap: Heightmap) -> i32 {
    for y in (MIN_Y..MAX_Y).rev() {
        let b = grid[grid_index(lx as usize, y, lz as usize)];
        let counts = match heightmap {
            Heightmap::OceanFloor => b != BLOCKS.air && !is_water(b) && !is_lava(b),
            Heightmap::MotionBlocking => b != BLOCKS.air,
        };
        if counts {
            return y + 1;
        }
    }
    MIN_Y
}

/// Pose les disques des biomes présents dans le chunk (chaque disque `count` fois par chunk, gardé seulement si
/// le biome de la colonne tirée le contient, comme le filtre `biome` vanilla). Les disques ne débordent pas du
/// chunk.
pub(super) fn decorate(
    grid: &mut [u32],
    biome_idx: &[[u16; 16]; 16],
    biome_names: &[String],
    rng: &mut Random,
) {
    let reg = &*REG;
    let mut present: Vec<&String> = Vec::new();
    for row in biome_idx {
        for &b in row {
            for name in reg
                .biome_disks
                .get(&biome_names[b as usize])
                .into_iter()
                .flatten()
            {
                if !present.contains(&name) {
                    present.push(name);
                }
            }
        }
    }
    for name in present {
        let disk = &reg.disks[name];
        for _ in 0..disk.count {
            let lx = rng.next_bounded_int(16);
            let lz = rng.next_bounded_int(16);
            let radius = disk.radius.0 + rng.next_bounded_int(disk.radius.1 - disk.radius.0 + 1);
            let biome = &biome_names[biome_idx[lx as usize][lz as usize] as usize];
            if !reg
                .biome_disks
                .get(biome)
                .is_some_and(|names| names.contains(name))
            {
                continue;
            }
            let y = height(grid, lx, lz, disk.heightmap) + disk.y_offset;
            if !test(&disk.filter, grid, lx, y, lz) {
                continue;
            }
            place(grid, disk, lx, y, lz, radius);
        }
    }
}

fn place(grid: &mut [u32], disk: &Disk, ox: i32, oy: i32, oz: i32, radius: i32) {
    let top = oy + disk.half_height;
    let bottom = oy - disk.half_height - 1;
    for lx in (ox - radius).max(0)..=(ox + radius).min(15) {
        for lz in (oz - radius).max(0)..=(oz + radius).min(15) {
            let (dx, dz) = (lx - ox, lz - oz);
            if dx * dx + dz * dz > radius * radius {
                continue;
            }
            let mut y = top;
            while y > bottom {
                if at(grid, lx, y, lz).is_some_and(|b| disk.targets.contains(&b)) {
                    let block = disk
                        .rules
                        .iter()
                        .find(|(p, _)| test(p, grid, lx, y, lz))
                        .map_or(disk.fallback, |&(_, b)| b);
                    grid[grid_index(lx as usize, y, lz as usize)] = block;
                }
                y -= 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vanilla_disks_are_read() {
        let reg = &*REG;
        for name in [
            "disk_sand",
            "disk_clay",
            "disk_gravel",
            "disk_grass",
            "ice_patch",
        ] {
            assert!(reg.disks.contains_key(name), "{name} non lu");
        }
        let sand = &reg.disks["disk_sand"];
        assert_eq!((sand.count, sand.radius, sand.half_height), (3, (2, 6), 2));
        assert_eq!(sand.fallback, BLOCKS.get("minecraft:sand"));
        assert!(reg.biome_disks["minecraft:river"].contains(&"disk_sand".to_string()));
        // `BLOCKS.get` rend de l'air pour un nom inconnu : tous les blocs des disques doivent exister.
        for (name, disk) in &reg.disks {
            let blocks = disk.targets.iter().chain(disk.rules.iter().map(|(_, b)| b));
            for &b in blocks.chain(std::iter::once(&disk.fallback)) {
                assert_ne!(b, BLOCKS.air, "{name} : bloc inconnu de la palette");
            }
        }
    }

    #[test]
    fn sand_disk_replaces_dirt_under_water_only() {
        let reg = &*REG;
        let mut grid = vec![BLOCKS.air; super::super::noise_chunk::GRID_LEN];
        for lx in 0..16usize {
            for lz in 0..16usize {
                for y in MIN_Y..=58 {
                    grid[grid_index(lx, y, lz)] = BLOCKS.get("minecraft:dirt");
                }
                for y in 59..=62 {
                    grid[grid_index(lx, y, lz)] = BLOCKS.water;
                }
            }
        }
        place(&mut grid, &reg.disks["disk_sand"], 8, 59, 8, 3);
        let sand = BLOCKS.get("minecraft:sand");
        // Origine sur le premier bloc d'eau (y = 59), demi-hauteur 2 : y = 61 à 57, deux couches de sable.
        assert_eq!(grid[grid_index(8, 58, 8)], sand);
        assert_eq!(grid[grid_index(8, 57, 8)], sand);
        assert_eq!(grid[grid_index(8, 56, 8)], BLOCKS.get("minecraft:dirt"));
        assert_eq!(grid[grid_index(8, 59, 8)], BLOCKS.water);
        assert_eq!(grid[grid_index(12, 58, 8)], BLOCKS.get("minecraft:dirt"));
    }
}
