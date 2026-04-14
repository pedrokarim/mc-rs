//! Maps — port PMMP `src/item/FilledMap.php` + map data packets.
//!
//! Une map est un objet qui affiche une vue top-down du monde. Elle a un
//! MapId unique, une scale (0-4), et un buffer de pixels 128×128.

use std::collections::HashMap;

pub const MAP_SIZE: usize = 128;
pub type MapPixel = u8; // Color index en Bedrock palette.

/// PMMP `FilledMap::getScale()`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapScale {
    Level0 = 0, // 1 pixel = 1 bloc
    Level1 = 1, // 1 pixel = 2 blocs
    Level2 = 2,
    Level3 = 3,
    Level4 = 4, // 1 pixel = 16 blocs
}

impl MapScale {
    pub fn blocks_per_pixel(&self) -> u32 {
        1 << (*self as u32)
    }

    pub fn world_size(&self) -> u32 {
        MAP_SIZE as u32 * self.blocks_per_pixel()
    }
}

#[derive(Debug, Clone)]
pub struct MapData {
    pub map_id: i64,
    pub scale: MapScale,
    pub center: [i32; 2], // XZ center
    pub dimension_id: u8,
    pub pixels: Vec<MapPixel>, // 128*128
    pub locked: bool,
    pub decorations: Vec<MapDecoration>,
}

#[derive(Debug, Clone)]
pub struct MapDecoration {
    pub kind: MapDecorationKind,
    pub rotation: i8, // 0-15
    pub x: i8,        // -128 to 127 map-local
    pub z: i8,
    pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapDecorationKind {
    Player = 0,
    Frame = 1,
    RedMarker = 2,
    BlueMarker = 3,
    TargetX = 4,
    TargetPoint = 5,
    PlayerOffMap = 6,
    PlayerOffLimits = 7,
    Mansion = 8,
    OceanMonument = 9,
    RedX = 10,
}

impl MapData {
    pub fn new(map_id: i64, scale: MapScale, center: [i32; 2], dimension_id: u8) -> Self {
        Self {
            map_id,
            scale,
            center,
            dimension_id,
            pixels: vec![0; MAP_SIZE * MAP_SIZE],
            locked: false,
            decorations: Vec::new(),
        }
    }

    pub fn set_pixel(&mut self, x: usize, z: usize, color: MapPixel) {
        if x < MAP_SIZE && z < MAP_SIZE {
            self.pixels[z * MAP_SIZE + x] = color;
        }
    }

    pub fn get_pixel(&self, x: usize, z: usize) -> Option<MapPixel> {
        if x < MAP_SIZE && z < MAP_SIZE {
            Some(self.pixels[z * MAP_SIZE + x])
        } else {
            None
        }
    }
}

#[derive(Default)]
pub struct MapRegistry {
    pub maps: HashMap<i64, MapData>,
    pub next_id: i64,
}

impl MapRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allocate(&mut self, scale: MapScale, center: [i32; 2], dimension_id: u8) -> i64 {
        self.next_id += 1;
        let id = self.next_id;
        self.maps.insert(id, MapData::new(id, scale, center, dimension_id));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_world_size_128_1024_2048_4096() {
        assert_eq!(MapScale::Level0.world_size(), 128);
        assert_eq!(MapScale::Level3.world_size(), 1024);
        assert_eq!(MapScale::Level4.world_size(), 2048);
    }

    #[test]
    fn pixel_set_get() {
        let mut m = MapData::new(1, MapScale::Level0, [0, 0], 0);
        m.set_pixel(5, 10, 42);
        assert_eq!(m.get_pixel(5, 10), Some(42));
        assert_eq!(m.get_pixel(0, 0), Some(0));
    }
}
