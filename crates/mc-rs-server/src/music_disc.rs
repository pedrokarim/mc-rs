//! Music disc list (13, cat, blocks, chirp, far, mall, mellohi, stal, strad, wait, ward, 5, Pigstep, Otherside, 13 duplicates).

pub fn all_music_discs() -> &'static [&'static str] {
    &[
        "minecraft:music_disc_13",
        "minecraft:music_disc_cat",
        "minecraft:music_disc_blocks",
        "minecraft:music_disc_chirp",
        "minecraft:music_disc_far",
        "minecraft:music_disc_mall",
        "minecraft:music_disc_mellohi",
        "minecraft:music_disc_stal",
        "minecraft:music_disc_strad",
        "minecraft:music_disc_ward",
        "minecraft:music_disc_11",
        "minecraft:music_disc_wait",
        "minecraft:music_disc_otherside",
        "minecraft:music_disc_5",
        "minecraft:music_disc_pigstep",
        "minecraft:music_disc_relic",
        "minecraft:music_disc_creator",
        "minecraft:music_disc_creator_music_box",
        "minecraft:music_disc_precipice",
        "minecraft:music_disc_tears",
        "minecraft:music_disc_lava_chicken",
    ]
}

/// Disc duration in seconds.
pub fn duration_seconds(disc: &str) -> u32 {
    match disc {
        "minecraft:music_disc_13" => 178,
        "minecraft:music_disc_cat" => 185,
        "minecraft:music_disc_blocks" => 345,
        "minecraft:music_disc_chirp" => 185,
        "minecraft:music_disc_far" => 174,
        "minecraft:music_disc_mall" => 197,
        "minecraft:music_disc_mellohi" => 96,
        "minecraft:music_disc_stal" => 150,
        "minecraft:music_disc_strad" => 188,
        "minecraft:music_disc_ward" => 260,
        "minecraft:music_disc_11" => 71,
        "minecraft:music_disc_wait" => 238,
        "minecraft:music_disc_otherside" => 195,
        "minecraft:music_disc_5" => 178,
        "minecraft:music_disc_pigstep" => 148,
        _ => 180,
    }
}

/// Drop sources in loot tables (skeleton + creeper kill).
pub fn drop_source() -> &'static str {
    "skeleton_kills_creeper"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discs_non_empty() {
        assert!(!all_music_discs().is_empty());
    }

    #[test]
    fn duration_positive() {
        assert!(duration_seconds("minecraft:music_disc_13") > 0);
    }
}
