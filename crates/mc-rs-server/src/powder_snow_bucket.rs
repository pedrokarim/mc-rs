//! Powder snow bucket — empty = snow from block, filled = water/powder snow/lava bucket.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BucketContent {
    Empty,
    Water,
    Lava,
    Milk,
    PowderSnow,
    FishBucket(FishKind),
    AxolotlBucket(u8), // variant
    TadpoleBucket,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FishKind {
    Cod,
    Salmon,
    Pufferfish,
    TropicalFish,
}

/// Drink milk removes all effects.
pub fn milk_removes_effects() -> bool { true }

/// Filling a bucket from water source.
pub fn can_fill_from(block_id: u16) -> Option<BucketContent> {
    match block_id {
        8 | 9 => Some(BucketContent::Water),
        10 | 11 => Some(BucketContent::Lava),
        478 => Some(BucketContent::PowderSnow),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fills_water() {
        assert_eq!(can_fill_from(9), Some(BucketContent::Water));
    }
}
