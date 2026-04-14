//! Arrow pickup — port PMMP `src/entity/projectile/Arrow.php::pickupMode`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrowPickupMode {
    /// Ne peut pas être pickup.
    Never,
    /// Pickup par n'importe qui (arrows normales).
    AnyOne,
    /// Seul le shooter peut pickup (arrows en créatif ou infinity).
    CreativeOnly,
}

pub fn pickup_mode_for_shooter_gamemode(gamemode: i32, has_infinity: bool) -> ArrowPickupMode {
    if gamemode == 1 {
        // creative
        ArrowPickupMode::CreativeOnly
    } else if has_infinity {
        ArrowPickupMode::CreativeOnly
    } else {
        ArrowPickupMode::AnyOne
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn survival_default_pickup_anyone() {
        assert_eq!(
            pickup_mode_for_shooter_gamemode(0, false),
            ArrowPickupMode::AnyOne
        );
    }

    #[test]
    fn infinity_restricted_to_shooter() {
        assert_eq!(
            pickup_mode_for_shooter_gamemode(0, true),
            ArrowPickupMode::CreativeOnly
        );
    }
}
