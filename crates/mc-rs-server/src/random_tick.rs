//! Random block tick dispatcher.

/// Dispatches random ticks per block type.
pub fn random_tick_block(block_id: u16) {
    match block_id {
        2 => {
            // Grass — spread to adjacent dirt.
        }
        6 => {
            // Sapling — grow to tree.
        }
        31 => {
            // Tall grass — spread.
        }
        59 => {
            // Wheat crop — grow stage.
        }
        79 | 212 => {
            // Ice — check for melting.
        }
        81 => {
            // Cactus — grow.
        }
        83 => {
            // Sugar cane — grow.
        }
        110 => {
            // Mycelium — spread.
        }
        116 => {
            // Farmland — hydration check.
        }
        _ => {}
    }
}

/// Chance per random tick speed (gamerule).
pub fn base_chance(speed: u32) -> f32 {
    speed as f32 / 16.0 * 16.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doesnt_crash() {
        random_tick_block(2);
    }
}
