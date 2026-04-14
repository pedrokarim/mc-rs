//! Fox — mob qui pick up items et sleep in daytime.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FoxVariant {
    Red,
    Snow,
}

#[derive(Debug, Clone)]
pub struct Fox {
    pub variant: FoxVariant,
    pub age: i32,
    pub sleeping: bool,
    pub crouching: bool,
    pub sitting: bool,
    pub held_item: Option<u16>,
    pub trusted_players: Vec<u64>,
}

/// Breeding item = sweet berries.
pub const BREEDING_ITEM: u16 = 265;
/// Items that foxes prefer to pick up.
pub fn preferred_items() -> &'static [&'static str] {
    &[
        "minecraft:emerald",
        "minecraft:diamond",
        "minecraft:golden_apple",
        "minecraft:leather",
        "minecraft:egg",
        "minecraft:rabbit_foot",
    ]
}

impl Fox {
    pub fn new(variant: FoxVariant) -> Self {
        Self {
            variant,
            age: 0,
            sleeping: false,
            crouching: false,
            sitting: false,
            held_item: None,
            trusted_players: Vec::new(),
        }
    }

    pub fn variant_from_biome(biome_id: u8) -> FoxVariant {
        // Snowy = snow, everything else red (simplification).
        match biome_id {
            12 | 13 | 30 | 31 | 140 | 158 => FoxVariant::Snow,
            _ => FoxVariant::Red,
        }
    }

    pub fn trust(&mut self, player_id: u64) {
        if !self.trusted_players.contains(&player_id) {
            self.trusted_players.push(player_id);
        }
    }

    pub fn is_trusted(&self, player_id: u64) -> bool {
        self.trusted_players.contains(&player_id)
    }

    pub fn tick(&mut self, is_daytime: bool, nearby_player: bool) {
        if self.age < 0 {
            self.age += 1;
        }
        if is_daytime && !nearby_player && !self.sleeping {
            self.sleeping = true;
        } else if (!is_daytime || nearby_player) && self.sleeping {
            self.sleeping = false;
        }
    }

    pub fn pick_up(&mut self, item_id: u16) -> Option<u16> {
        let previous = self.held_item.take();
        self.held_item = Some(item_id);
        previous
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sleeps_during_day_no_player() {
        let mut f = Fox::new(FoxVariant::Red);
        f.tick(true, false);
        assert!(f.sleeping);
    }

    #[test]
    fn wakes_when_player_nearby() {
        let mut f = Fox::new(FoxVariant::Red);
        f.sleeping = true;
        f.tick(true, true);
        assert!(!f.sleeping);
    }

    #[test]
    fn pickup_replaces_held_item() {
        let mut f = Fox::new(FoxVariant::Red);
        f.pick_up(1);
        let old = f.pick_up(2);
        assert_eq!(old, Some(1));
        assert_eq!(f.held_item, Some(2));
    }
}
