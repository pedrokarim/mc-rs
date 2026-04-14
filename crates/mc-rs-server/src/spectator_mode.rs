//! Spectator mode rules.

/// Max flight speed multiplier.
pub const FLIGHT_SPEED_BASE: f32 = 0.1;

/// Can pass through blocks.
pub const NOCLIP: bool = true;

/// No physics / gravity.
pub const NO_GRAVITY: bool = true;

/// Invisible to other spectators (still see each other with transparent outline).
pub const INVISIBLE_TO_PLAYERS: bool = true;

/// Can't interact with blocks or entities.
pub const NO_INTERACTION: bool = true;

/// Can spectate any entity.
pub fn can_spectate_entity(entity_type: &str) -> bool {
    !matches!(entity_type, "minecraft:player") // can't spectate other players in same team
}

/// Shift to exit spectating.
pub const EXIT_KEY: &str = "shift";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cant_spectate_player() {
        assert!(!can_spectate_entity("minecraft:player"));
    }
}
