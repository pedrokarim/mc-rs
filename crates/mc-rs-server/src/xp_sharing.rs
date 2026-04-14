//! XP sharing — mending enchant repair priority.

/// Total mending enchants on equipment (cap 1 per slot).
pub const MENDING_SLOTS_MAX: u8 = 6; // armor 4 + main + off
/// XP orb collected → repair chance if mending equipped.
pub const MENDING_REPAIR_RATIO: f32 = 2.0; // 2 durability per XP

/// Randomly select item with mending to repair.
pub fn pick_item_for_mending(items_with_mending: &[usize]) -> Option<usize> {
    use rand::seq::SliceRandom;
    let mut rng = rand::thread_rng();
    items_with_mending.choose(&mut rng).copied()
}

/// XP consumed → durability added.
pub fn repair_amount(xp_orbs: u32) -> u32 {
    (xp_orbs as f32 * MENDING_REPAIR_RATIO) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mending_doubles_xp() {
        assert!(repair_amount(10) > 10);
    }
}
