//! Movement calculations — port PMMP `src/entity/Entity.php::tryChangeMovement`.

pub const GRAVITY: f32 = 0.08;
pub const DRAG: f32 = 0.02;
pub const FRICTION_DEFAULT: f32 = 0.6;
pub const FRICTION_ICE: f32 = 0.98;
pub const FRICTION_PACKED_ICE: f32 = 0.98;
pub const FRICTION_BLUE_ICE: f32 = 0.989;

/// Applique gravité + drag à la motion. Modifie motion en place.
pub fn apply_gravity_and_drag(motion: &mut [f32; 3], gravity: f32, drag: f32, drag_before_gravity: bool) {
    if drag_before_gravity {
        motion[1] *= 1.0 - drag;
        motion[1] -= gravity;
    } else {
        motion[1] -= gravity;
        motion[1] *= 1.0 - drag;
    }
    motion[0] *= 1.0 - drag;
    motion[2] *= 1.0 - drag;
}

/// Applique friction horizontale (sol).
pub fn apply_friction(motion: &mut [f32; 3], friction: f32) {
    motion[0] *= friction;
    motion[2] *= friction;
}

/// Base friction selon le type de bloc sous l'entité.
pub fn block_friction(block_name: &str) -> f32 {
    match block_name {
        "minecraft:ice" | "minecraft:frosted_ice" => FRICTION_ICE,
        "minecraft:packed_ice" => FRICTION_PACKED_ICE,
        "minecraft:blue_ice" => FRICTION_BLUE_ICE,
        "minecraft:slime" => 0.8,
        _ => FRICTION_DEFAULT,
    }
}

/// Jump strength (vertical motion add) selon effets.
pub fn jump_strength(base: f32, jump_boost_amplifier: u8) -> f32 {
    base + 0.1 * jump_boost_amplifier as f32
}

/// Standard jump add pour joueur : 0.42.
pub const PLAYER_JUMP_MOTION_Y: f32 = 0.42;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ice_slippery() {
        assert!(block_friction("minecraft:ice") > FRICTION_DEFAULT);
    }

    #[test]
    fn slime_less_friction_than_stone() {
        // slime is stickier (higher friction means less slip).
        assert!(block_friction("minecraft:slime") > FRICTION_DEFAULT);
    }

    #[test]
    fn jump_boost_adds_vertical() {
        assert_eq!(jump_strength(PLAYER_JUMP_MOTION_Y, 2), 0.42 + 0.2);
    }
}
