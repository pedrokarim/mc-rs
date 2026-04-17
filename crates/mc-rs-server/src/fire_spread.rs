//! Fire spread — block flammability, burn rate.

#[derive(Debug, Clone, Copy)]
pub struct Flammability {
    pub burn_chance: u32,   // 0-60 (higher = burns faster)
    pub spread_chance: u32, // 0-100 (higher = more likely to spread)
}

/// Flammability per block (PMMP).
pub fn flammability_for(block_id: u16) -> Flammability {
    match block_id {
        17 | 162 => Flammability {
            burn_chance: 5,
            spread_chance: 5,
        }, // wood logs
        5 | 125 | 126 => Flammability {
            burn_chance: 5,
            spread_chance: 20,
        }, // planks
        35 => Flammability {
            burn_chance: 30,
            spread_chance: 60,
        }, // wool
        18 | 161 => Flammability {
            burn_chance: 30,
            spread_chance: 60,
        }, // leaves
        37 | 38 | 39 | 40 => Flammability {
            burn_chance: 60,
            spread_chance: 100,
        }, // flowers/mushrooms
        31 | 175 => Flammability {
            burn_chance: 60,
            spread_chance: 100,
        }, // grass/tall grass
        85 | 107 => Flammability {
            burn_chance: 5,
            spread_chance: 20,
        }, // fences
        321 | 58 => Flammability {
            burn_chance: 5,
            spread_chance: 20,
        }, // painting / crafting table
        170 => Flammability {
            burn_chance: 60,
            spread_chance: 20,
        }, // hay bale
        _ => Flammability {
            burn_chance: 0,
            spread_chance: 0,
        },
    }
}

pub fn is_flammable(block_id: u16) -> bool {
    flammability_for(block_id).burn_chance > 0
}

/// Infinite burn — netherrack, magma.
pub fn is_infinite_burn(block_id: u16) -> bool {
    matches!(block_id, 87 | 213) // netherrack, magma block
}

/// Fire immune — many nether blocks.
pub fn is_fire_immune(block_id: u16) -> bool {
    matches!(
        block_id,
        49  // obsidian
        | 7 // bedrock
        | 145 // anvil
        | 87 // netherrack (burns forever)
        | 153 // quartz ore
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wood_is_flammable() {
        assert!(is_flammable(17));
    }

    #[test]
    fn stone_not_flammable() {
        assert!(!is_flammable(1));
    }

    #[test]
    fn netherrack_burns_forever() {
        assert!(is_infinite_burn(87));
    }
}
