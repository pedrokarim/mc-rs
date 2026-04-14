//! Player ability bitfield — protocol 944.

/// Abilities bit flags.
pub const ABILITY_BUILD: u32 = 1 << 0;
pub const ABILITY_MINE: u32 = 1 << 1;
pub const ABILITY_DOORS_AND_SWITCHES: u32 = 1 << 2;
pub const ABILITY_OPEN_CONTAINERS: u32 = 1 << 3;
pub const ABILITY_ATTACK_PLAYERS: u32 = 1 << 4;
pub const ABILITY_ATTACK_MOBS: u32 = 1 << 5;
pub const ABILITY_OPERATOR: u32 = 1 << 6;
pub const ABILITY_TEACHER: u32 = 1 << 7;
pub const ABILITY_FLYING: u32 = 1 << 8;
pub const ABILITY_MAY_FLY: u32 = 1 << 9;
pub const ABILITY_INSTABUILD: u32 = 1 << 10;
pub const ABILITY_LIGHTNING: u32 = 1 << 11;
pub const ABILITY_FLY_SPEED: u32 = 1 << 12;
pub const ABILITY_WALK_SPEED: u32 = 1 << 13;
pub const ABILITY_MUTED: u32 = 1 << 14;
pub const ABILITY_WORLD_BUILDER: u32 = 1 << 15;
pub const ABILITY_NO_CLIP: u32 = 1 << 16;
pub const ABILITY_PRIVILEGED_BUILDER: u32 = 1 << 17;
pub const ABILITY_VERTICAL_FLY_SPEED: u32 = 1 << 19; // Critical per notes!

/// Gamemode → default abilities.
pub fn default_abilities_for_gamemode(gm: u8) -> u32 {
    let base = ABILITY_BUILD
        | ABILITY_MINE
        | ABILITY_DOORS_AND_SWITCHES
        | ABILITY_OPEN_CONTAINERS
        | ABILITY_ATTACK_PLAYERS
        | ABILITY_ATTACK_MOBS;
    match gm {
        0 => base | ABILITY_WALK_SPEED, // survival
        1 => base | ABILITY_INSTABUILD | ABILITY_MAY_FLY | ABILITY_WALK_SPEED | ABILITY_FLY_SPEED, // creative
        2 => ABILITY_WALK_SPEED, // adventure
        3 => base | ABILITY_MAY_FLY | ABILITY_FLYING | ABILITY_NO_CLIP, // spectator
        _ => 0,
    }
}

/// Command permission level.
pub const PERMISSION_VISITOR: u8 = 0;
pub const PERMISSION_MEMBER: u8 = 1;
pub const PERMISSION_OPERATOR: u8 = 2;
pub const PERMISSION_AUTOMATION: u8 = 3;
pub const PERMISSION_ADMIN: u8 = 4;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creative_can_fly() {
        assert!(default_abilities_for_gamemode(1) & ABILITY_MAY_FLY != 0);
    }

    #[test]
    fn survival_cant_fly() {
        assert_eq!(default_abilities_for_gamemode(0) & ABILITY_MAY_FLY, 0);
    }
}
