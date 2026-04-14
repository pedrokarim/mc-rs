//! Climbable blocks — ladder, vine, scaffolding, twisting_vines, weeping_vines.

/// Un bloc est grimpable ?
pub fn is_climbable(block_name: &str) -> bool {
    matches!(
        block_name,
        "minecraft:ladder"
            | "minecraft:vine"
            | "minecraft:scaffolding"
            | "minecraft:twisting_vines"
            | "minecraft:twisting_vines_plant"
            | "minecraft:weeping_vines"
            | "minecraft:weeping_vines_plant"
            | "minecraft:cave_vines"
            | "minecraft:cave_vines_plant"
    )
}

/// Vitesse verticale quand on grimpe.
pub const CLIMB_SPEED: f32 = 0.2;

/// Vitesse descente sur scaffolding (quand accroupi).
pub const SCAFFOLDING_DESCEND_SPEED: f32 = 0.1;

/// Cancels fall damage quand on tombe de <limit> blocs sur ladder/vine.
pub const CLIMB_FALL_DAMAGE_IMMUNITY_DIST: f32 = 0.0; // ladders = total immunity

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ladder_climbable() {
        assert!(is_climbable("minecraft:ladder"));
    }

    #[test]
    fn stone_not_climbable() {
        assert!(!is_climbable("minecraft:stone"));
    }
}
