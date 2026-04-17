//! Azalea + Flowering Azalea — grows into azalea tree with bone meal.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AzaleaKind {
    Azalea,
    Flowering,
}

/// Bone meal grows azalea tree (+oak variant).
pub const BONE_MEAL_GROWS_TREE: bool = true;

/// Azalea sapling requires grass block.
pub fn valid_ground() -> &'static [u16] {
    &[2, 3, 208, 110] // grass, dirt, podzol, mycelium
}

/// Drops when harvested.
pub fn shear_drops(kind: AzaleaKind) -> &'static str {
    match kind {
        AzaleaKind::Azalea => "minecraft:azalea_leaves",
        AzaleaKind::Flowering => "minecraft:flowering_azalea_leaves",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flowering_shears_to_flowering_leaves() {
        assert_eq!(
            shear_drops(AzaleaKind::Flowering),
            "minecraft:flowering_azalea_leaves"
        );
    }
}
