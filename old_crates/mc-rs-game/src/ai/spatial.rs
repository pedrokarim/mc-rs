//! Spatial grid for efficient nearest-entity queries during AI tick.

use std::collections::HashMap;

/// Cell size in blocks (matches chunk size for simplicity).
const CELL_SIZE: f32 = 16.0;

/// Result of a nearest-entity spatial query: (entity_bits, runtime_id, distance, position).
type NearestResult = (u64, u64, f32, (f32, f32, f32));

/// An entity entry in the spatial grid.
#[derive(Clone)]
pub struct SpatialEntry {
    pub runtime_id: u64,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub mob_type: String,
    pub is_player: bool,
    pub held_item_name: String,
    pub in_love: bool,
    pub is_baby: bool,
    /// Bevy ECS Entity (stored as u64 bits for simplicity).
    pub entity_bits: u64,
}

/// A spatial hash grid for O(1) cell lookup of nearby entities.
#[derive(Default)]
pub struct SpatialGrid {
    cells: HashMap<(i32, i32), Vec<SpatialEntry>>,
}

impl SpatialGrid {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert an entry into the grid.
    pub fn insert(&mut self, entry: SpatialEntry) {
        let key = cell_key(entry.x, entry.z);
        self.cells.entry(key).or_default().push(entry);
    }

    /// Query the nearest player within `max_dist` of `(x, z)`.
    ///
    /// Returns `(entity_bits, runtime_id, distance, (px, py, pz))`.
    pub fn query_nearest_player(&self, x: f32, z: f32, max_dist: f32) -> Option<NearestResult> {
        let max_dist_sq = max_dist * max_dist;
        let radius = (max_dist / CELL_SIZE).ceil() as i32;
        let (cx, cz) = cell_key(x, z);

        let mut best: Option<NearestResult> = None;

        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if let Some(entries) = self.cells.get(&(cx + dx, cz + dz)) {
                    for e in entries {
                        if !e.is_player {
                            continue;
                        }
                        let dist_sq = dist_sq_xz(x, z, e.x, e.z);
                        if dist_sq > max_dist_sq {
                            continue;
                        }
                        let dist = dist_sq.sqrt();
                        if best.as_ref().map(|b| dist < b.2).unwrap_or(true) {
                            best = Some((e.entity_bits, e.runtime_id, dist, (e.x, e.y, e.z)));
                        }
                    }
                }
            }
        }

        best
    }

    /// Query the nearest player holding a tempt item, within `max_dist`.
    pub fn query_nearest_tempting_player(
        &self,
        x: f32,
        z: f32,
        max_dist: f32,
        mob_type: &str,
        is_tempt_item: &dyn Fn(&str, &str) -> bool,
    ) -> Option<NearestResult> {
        let max_dist_sq = max_dist * max_dist;
        let radius = (max_dist / CELL_SIZE).ceil() as i32;
        let (cx, cz) = cell_key(x, z);

        let mut best: Option<NearestResult> = None;

        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if let Some(entries) = self.cells.get(&(cx + dx, cz + dz)) {
                    for e in entries {
                        if !e.is_player {
                            continue;
                        }
                        if !is_tempt_item(mob_type, &e.held_item_name) {
                            continue;
                        }
                        let dist_sq = dist_sq_xz(x, z, e.x, e.z);
                        if dist_sq > max_dist_sq {
                            continue;
                        }
                        let dist = dist_sq.sqrt();
                        if best.as_ref().map(|b| dist < b.2).unwrap_or(true) {
                            best = Some((e.entity_bits, e.runtime_id, dist, (e.x, e.y, e.z)));
                        }
                    }
                }
            }
        }

        best
    }

    /// Query the nearest mob of a given type that is in love and not a baby,
    /// excluding a specific entity.
    pub fn query_nearest_breed_partner(
        &self,
        x: f32,
        z: f32,
        max_dist: f32,
        mob_type: &str,
        exclude_entity_bits: u64,
    ) -> Option<(u64, u64, f32, f32, f32)> {
        let max_dist_sq = max_dist * max_dist;
        let radius = (max_dist / CELL_SIZE).ceil() as i32;
        let (cx, cz) = cell_key(x, z);

        let mut best: Option<(u64, u64, f32, f32, f32, f32)> = None; // + dist

        for dx in -radius..=radius {
            for dz in -radius..=radius {
                if let Some(entries) = self.cells.get(&(cx + dx, cz + dz)) {
                    for e in entries {
                        if e.is_player
                            || e.entity_bits == exclude_entity_bits
                            || e.mob_type != mob_type
                            || !e.in_love
                            || e.is_baby
                        {
                            continue;
                        }
                        let dist_sq = dist_sq_xz(x, z, e.x, e.z);
                        if dist_sq > max_dist_sq {
                            continue;
                        }
                        let dist = dist_sq.sqrt();
                        if best.as_ref().map(|b| dist < b.5).unwrap_or(true) {
                            best = Some((e.entity_bits, e.runtime_id, e.x, e.y, e.z, dist));
                        }
                    }
                }
            }
        }

        best.map(|(eb, rid, ex, ey, ez, _)| (eb, rid, ex, ey, ez))
    }
}

/// Compute the cell key for a world position.
fn cell_key(x: f32, z: f32) -> (i32, i32) {
    (
        (x / CELL_SIZE).floor() as i32,
        (z / CELL_SIZE).floor() as i32,
    )
}

/// Squared distance in XZ plane.
fn dist_sq_xz(x1: f32, z1: f32, x2: f32, z2: f32) -> f32 {
    let dx = x2 - x1;
    let dz = z2 - z1;
    dx * dx + dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_player(rid: u64, x: f32, z: f32, held: &str) -> SpatialEntry {
        SpatialEntry {
            runtime_id: rid,
            x,
            y: 4.0,
            z,
            mob_type: String::new(),
            is_player: true,
            held_item_name: held.to_string(),
            in_love: false,
            is_baby: false,
            entity_bits: rid,
        }
    }

    fn make_mob(rid: u64, x: f32, z: f32, mob_type: &str, in_love: bool) -> SpatialEntry {
        SpatialEntry {
            runtime_id: rid,
            x,
            y: 4.0,
            z,
            mob_type: mob_type.to_string(),
            is_player: false,
            held_item_name: String::new(),
            in_love,
            is_baby: false,
            entity_bits: rid,
        }
    }

    #[test]
    fn nearest_player_basic() {
        let mut grid = SpatialGrid::new();
        grid.insert(make_player(1, 10.0, 10.0, ""));
        grid.insert(make_player(2, 50.0, 50.0, ""));

        let result = grid.query_nearest_player(12.0, 12.0, 128.0);
        assert!(result.is_some());
        let (_, rid, _, _) = result.unwrap();
        assert_eq!(rid, 1, "Player 1 is closer");
    }

    #[test]
    fn nearest_player_max_dist() {
        let mut grid = SpatialGrid::new();
        grid.insert(make_player(1, 100.0, 100.0, ""));

        let result = grid.query_nearest_player(0.0, 0.0, 10.0);
        assert!(result.is_none(), "Player is beyond max_dist");
    }

    #[test]
    fn nearest_breed_partner() {
        let mut grid = SpatialGrid::new();
        grid.insert(make_mob(10, 5.0, 5.0, "minecraft:cow", true));
        grid.insert(make_mob(11, 100.0, 100.0, "minecraft:cow", true));
        grid.insert(make_mob(12, 3.0, 3.0, "minecraft:pig", true)); // wrong type

        let result = grid.query_nearest_breed_partner(0.0, 0.0, 128.0, "minecraft:cow", 999);
        assert!(result.is_some());
        let (_, rid, _, _, _) = result.unwrap();
        assert_eq!(rid, 10, "Cow at (5,5) is closest");
    }

    #[test]
    fn nearest_breed_partner_excludes_self() {
        let mut grid = SpatialGrid::new();
        grid.insert(make_mob(10, 0.0, 0.0, "minecraft:cow", true));

        let result = grid.query_nearest_breed_partner(0.0, 0.0, 128.0, "minecraft:cow", 10);
        assert!(result.is_none(), "Should exclude self");
    }
}
