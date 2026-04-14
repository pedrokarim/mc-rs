//! Chunk serializer extensions — heightmap + light data + sky light.
//! Port conceptuel de PMMP `src/world/format/io/LevelProvider` extensions.

/// Heightmap d'un chunk : 16×16 hauteurs Y max.
#[derive(Debug, Clone)]
pub struct HeightMap {
    pub values: [[i16; 16]; 16],
}

impl HeightMap {
    pub fn new() -> Self {
        Self {
            values: [[0; 16]; 16],
        }
    }

    pub fn set(&mut self, x: usize, z: usize, height: i16) {
        if x < 16 && z < 16 {
            self.values[z][x] = height;
        }
    }

    pub fn get(&self, x: usize, z: usize) -> i16 {
        if x < 16 && z < 16 {
            self.values[z][x]
        } else {
            0
        }
    }
}

impl Default for HeightMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Sky light per-block. 0 (dark) à 15 (full sunlight). Stocké en 4-bit nibbles.
#[derive(Debug, Clone)]
pub struct SkyLightData {
    /// Un nibble (4 bits) par bloc = 16*16*16 / 2 = 2048 bytes par section 16³.
    pub nibbles: Vec<u8>,
}

impl SkyLightData {
    pub fn new_full_bright(section_count: usize) -> Self {
        Self {
            nibbles: vec![0xFF; 2048 * section_count],
        }
    }

    pub fn new_dark(section_count: usize) -> Self {
        Self {
            nibbles: vec![0x00; 2048 * section_count],
        }
    }
}

/// Block light (émis par torches, lave, etc.).
#[derive(Debug, Clone)]
pub struct BlockLightData {
    pub nibbles: Vec<u8>,
}

impl BlockLightData {
    pub fn new_dark(section_count: usize) -> Self {
        Self {
            nibbles: vec![0x00; 2048 * section_count],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heightmap_set_get() {
        let mut h = HeightMap::new();
        h.set(5, 10, 64);
        assert_eq!(h.get(5, 10), 64);
    }

    #[test]
    fn skylight_full_bright() {
        let s = SkyLightData::new_full_bright(24);
        assert_eq!(s.nibbles.len(), 2048 * 24);
        assert_eq!(s.nibbles[0], 0xFF);
    }
}
