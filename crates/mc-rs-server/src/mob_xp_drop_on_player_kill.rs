//! Mob XP drops only when killed by player (no drops from fall damage etc.).

/// Was the kill attributed to a player (in the last 5 seconds)?
pub fn kill_by_player_window_ticks() -> u32 { 100 }

/// XP only drops from mobs killed by player (or their wolf/named pet).
pub fn eligible_killer(kind: &str) -> bool {
    matches!(kind, "player" | "wolf" | "cat" | "named_mob")
}

#[cfg(test)]
mod tests {
    #[test]
    fn player_eligible() {
        assert!(super::eligible_killer("player"));
    }
}
