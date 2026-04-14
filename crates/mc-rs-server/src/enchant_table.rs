//! Enchantment table — bookshelf count, XP cost calculation.

use rand::Rng;

/// Max bookshelves counted (15).
pub const MAX_BOOKSHELVES: u32 = 15;
/// Bookshelves provide max level 30.
pub const MAX_LEVEL_WITH_BOOKSHELVES: u32 = 30;

/// Count bookshelves in a 5x5x5 ring around the table (simplified).
pub fn bookshelf_power(bookshelf_positions: &[(i32, i32, i32)]) -> u32 {
    (bookshelf_positions.len() as u32).min(MAX_BOOKSHELVES)
}

/// Compute base enchantment levels for 3 slots given power.
/// Vanilla formula roughly.
pub fn slot_levels(power: u32) -> [u32; 3] {
    let mut rng = rand::thread_rng();
    let base: i32 = rng.gen_range(1..=8) + (power as i32) / 2 + rng.gen_range(0..=power as i32);
    let top = base.max(1) as u32;
    let mid = ((top as f32 * 2.0 / 3.0) + 1.0) as u32;
    let bottom = (top / 3).max(1);
    [bottom, mid, top]
}

/// XP cost = slot index + 1 (1, 2, 3).
pub fn xp_cost_for_slot(slot: usize) -> u32 {
    slot as u32 + 1
}

/// Lapis cost = slot index + 1.
pub fn lapis_cost_for_slot(slot: usize) -> u32 {
    slot as u32 + 1
}

/// Books can get Treasure enchants at enchant table too (Soul Speed, Swift Sneak — nope, these are library only).
pub fn treasure_table_enchants() -> &'static [&'static str] {
    &["mending", "frost_walker"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn max_15_bookshelves() {
        let many = vec![(0, 0, 0); 30];
        assert_eq!(bookshelf_power(&many), MAX_BOOKSHELVES);
    }

    #[test]
    fn slot_3_costs_3_lapis() {
        assert_eq!(lapis_cost_for_slot(2), 3);
    }
}
