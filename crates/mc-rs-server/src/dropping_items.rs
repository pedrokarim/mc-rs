//! Dropping items from player inventory.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropMode {
    SingleItem,    // Q
    WholeStack,    // Ctrl+Q
    DropQ,         // Q from cursor
    DropOnDeath,   // Death drop
}

/// Throw velocity when dropping.
pub const DROP_HORIZONTAL_VELOCITY: f64 = 0.3;
/// Throw upward velocity.
pub const DROP_VERTICAL_VELOCITY: f64 = 0.2;
/// Pickup delay after drop (40 ticks = 2s).
pub const PICKUP_DELAY_AFTER_DROP: u32 = 40;

/// Random offset applied on drop for multiple items.
pub fn random_drop_offset() -> f64 {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (rng.gen::<f64>() - 0.5) * 0.02
}

#[cfg(test)]
mod tests {
    #[test]
    fn constants_make_sense() {
        assert!(super::DROP_HORIZONTAL_VELOCITY > 0.0);
    }
}
