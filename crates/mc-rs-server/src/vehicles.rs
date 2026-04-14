//! Vehicles — port PMMP `src/entity/Minecart.php` + `Boat.php`.
//! Partial : position tick + rider mounting.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VehicleKind {
    Minecart,
    ChestMinecart,
    FurnaceMinecart,
    HopperMinecart,
    TntMinecart,
    CommandBlockMinecart,
    Boat,
    ChestBoat,
    Horse,
    Donkey,
    Mule,
    Llama,
    TraderLlama,
    Pig, // saddled
    Strider,
    Camel,
}

impl VehicleKind {
    pub fn max_passengers(&self) -> usize {
        match self {
            Self::Boat | Self::ChestBoat => 2,
            Self::Camel => 2,
            _ => 1,
        }
    }

    pub fn max_speed(&self) -> f32 {
        match self {
            Self::Minecart | Self::ChestMinecart | Self::FurnaceMinecart
            | Self::HopperMinecart | Self::TntMinecart | Self::CommandBlockMinecart => 0.4,
            Self::Boat | Self::ChestBoat => 0.35,
            Self::Horse | Self::Donkey | Self::Mule => 0.34, // varies with breed
            Self::Llama | Self::TraderLlama => 0.2,
            Self::Pig => 0.15,
            Self::Strider => 0.23,
            Self::Camel => 0.35,
        }
    }

    pub fn is_minecart(&self) -> bool {
        matches!(
            self,
            Self::Minecart
                | Self::ChestMinecart
                | Self::FurnaceMinecart
                | Self::HopperMinecart
                | Self::TntMinecart
                | Self::CommandBlockMinecart
        )
    }
}

#[derive(Debug, Clone)]
pub struct Vehicle {
    pub entity_unique_id: i64,
    pub entity_runtime_id: u64,
    pub kind: VehicleKind,
    pub position: [f32; 3],
    pub motion: [f32; 3],
    /// Runtime IDs des passagers (order = seat position).
    pub passengers: Vec<u64>,
}

impl Vehicle {
    pub fn new(
        entity_unique_id: i64,
        entity_runtime_id: u64,
        kind: VehicleKind,
        position: [f32; 3],
    ) -> Self {
        Self {
            entity_unique_id,
            entity_runtime_id,
            kind,
            position,
            motion: [0.0, 0.0, 0.0],
            passengers: Vec::new(),
        }
    }

    pub fn mount(&mut self, passenger: u64) -> bool {
        if self.passengers.len() >= self.kind.max_passengers() {
            return false;
        }
        if !self.passengers.contains(&passenger) {
            self.passengers.push(passenger);
        }
        true
    }

    pub fn dismount(&mut self, passenger: u64) {
        self.passengers.retain(|p| *p != passenger);
    }

    pub fn is_full(&self) -> bool {
        self.passengers.len() >= self.kind.max_passengers()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boat_holds_2() {
        let mut b = Vehicle::new(1, 1, VehicleKind::Boat, [0.0, 64.0, 0.0]);
        assert!(b.mount(10));
        assert!(b.mount(11));
        assert!(!b.mount(12));
    }

    #[test]
    fn minecart_holds_1() {
        let mut c = Vehicle::new(1, 1, VehicleKind::Minecart, [0.0, 64.0, 0.0]);
        assert!(c.mount(10));
        assert!(!c.mount(11));
    }

    #[test]
    fn dismount_frees_seat() {
        let mut b = Vehicle::new(1, 1, VehicleKind::Boat, [0.0, 64.0, 0.0]);
        b.mount(10);
        b.mount(11);
        assert!(b.is_full());
        b.dismount(10);
        assert!(!b.is_full());
    }
}
