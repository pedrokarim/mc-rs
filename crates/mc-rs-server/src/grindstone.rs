//! Grindstone — remove enchantments + repair.

use crate::enchantments::EnchantmentKind;

#[derive(Debug, Clone)]
pub struct GrindstoneOperation {
    pub input_a: u16,
    pub input_b: Option<u16>,
    pub enchants_a: Vec<(EnchantmentKind, u8)>,
    pub enchants_b: Vec<(EnchantmentKind, u8)>,
}

impl GrindstoneOperation {
    /// Compute output enchantments (keep only curses).
    pub fn result_enchantments(&self) -> Vec<(EnchantmentKind, u8)> {
        let keep = |e: &(EnchantmentKind, u8)| matches!(e.0,
            EnchantmentKind::BindingCurse | EnchantmentKind::VanishingCurse
        );
        let mut out: Vec<_> = self.enchants_a.iter().copied().filter(keep).collect();
        out.extend(self.enchants_b.iter().copied().filter(keep));
        out
    }

    /// XP dropped when grinding (based on enchantments).
    pub fn xp_dropped(&self) -> u32 {
        let count = self.enchants_a.iter().filter(|e|
            !matches!(e.0, EnchantmentKind::BindingCurse | EnchantmentKind::VanishingCurse)
        ).count()
            + self.enchants_b.iter().filter(|e|
            !matches!(e.0, EnchantmentKind::BindingCurse | EnchantmentKind::VanishingCurse)
        ).count();
        (count * 5) as u32
    }

    /// Repair durability (sum of a+b + 5% bonus).
    pub fn repair_durability(dur_a: u16, dur_b: u16, max: u16) -> u16 {
        let total = dur_a as u32 + dur_b as u32 + (max as u32 * 5 / 100);
        (total as u16).min(max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn curses_survive_grind() {
        let op = GrindstoneOperation {
            input_a: 1,
            input_b: None,
            enchants_a: vec![
                (EnchantmentKind::Sharpness, 5),
                (EnchantmentKind::BindingCurse, 1),
            ],
            enchants_b: vec![],
        };
        let out = op.result_enchantments();
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].0, EnchantmentKind::BindingCurse);
    }

    #[test]
    fn grinding_drops_xp() {
        let op = GrindstoneOperation {
            input_a: 1,
            input_b: None,
            enchants_a: vec![(EnchantmentKind::Sharpness, 5)],
            enchants_b: vec![],
        };
        assert!(op.xp_dropped() > 0);
    }
}
