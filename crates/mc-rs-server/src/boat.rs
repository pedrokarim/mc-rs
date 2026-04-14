//! Boat + ChestBoat — variants par type de bois.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WoodType {
    Oak, Spruce, Birch, Jungle, Acacia, DarkOak, Mangrove, Cherry, Bamboo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoatKind {
    Normal,
    Chest,
}

#[derive(Debug, Clone)]
pub struct Boat {
    pub wood: WoodType,
    pub kind: BoatKind,
    pub passengers: [Option<u64>; 2],
    pub inventory: Vec<Option<(u16, u16)>>,
    pub paddling_cooldown_left: u32,
    pub paddling_cooldown_right: u32,
}

/// Chest boat: 27 slots.
pub const CHEST_BOAT_SLOTS: usize = 27;

impl Boat {
    pub fn new(wood: WoodType, kind: BoatKind) -> Self {
        let slots = if kind == BoatKind::Chest { CHEST_BOAT_SLOTS } else { 0 };
        Self {
            wood,
            kind,
            passengers: [None; 2],
            inventory: vec![None; slots],
            paddling_cooldown_left: 0,
            paddling_cooldown_right: 0,
        }
    }

    pub fn add_passenger(&mut self, player_id: u64) -> bool {
        for slot in self.passengers.iter_mut() {
            if slot.is_none() {
                *slot = Some(player_id);
                return true;
            }
        }
        false
    }

    pub fn remove_passenger(&mut self, player_id: u64) {
        for slot in self.passengers.iter_mut() {
            if *slot == Some(player_id) {
                *slot = None;
            }
        }
    }

    /// Bamboo raft has different model (no hull above water).
    pub fn is_bamboo_raft(&self) -> bool {
        self.wood == WoodType::Bamboo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boat_holds_two_passengers() {
        let mut b = Boat::new(WoodType::Oak, BoatKind::Normal);
        assert!(b.add_passenger(1));
        assert!(b.add_passenger(2));
        assert!(!b.add_passenger(3));
    }

    #[test]
    fn chest_boat_has_inventory() {
        let b = Boat::new(WoodType::Oak, BoatKind::Chest);
        assert_eq!(b.inventory.len(), CHEST_BOAT_SLOTS);
    }
}
