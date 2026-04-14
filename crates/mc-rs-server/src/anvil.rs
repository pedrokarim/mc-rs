//! Anvil — port PMMP `src/block/Anvil.php` + enchant/rename logic.

use crate::enchantments::{EnchantmentInstance, EnchantmentKind};

/// Coût XP pour une opération anvil.
#[derive(Debug, Clone, Default)]
pub struct AnvilCost {
    pub xp_levels: u32,
    pub output_use_count: u32, // incrémenté à chaque use pour rendre plus cher
}

/// Résultat d'une opération anvil.
#[derive(Debug, Clone)]
pub struct AnvilResult {
    pub output_enchants: Vec<EnchantmentInstance>,
    pub output_damage: u32,
    pub cost: AnvilCost,
    pub valid: bool,
}

/// Combine 2 items durables au niveau des enchantements.
/// PMMP `EnchantHelper::combineItems()` logique simplifiée.
pub fn combine_items(
    left_enchants: &[EnchantmentInstance],
    right_enchants: &[EnchantmentInstance],
) -> Vec<EnchantmentInstance> {
    let mut out: Vec<EnchantmentInstance> = left_enchants.to_vec();
    for right in right_enchants {
        // Check compatibility.
        if out.iter().any(|e| e.kind.incompatible_with(right.kind)) {
            continue;
        }
        match out.iter_mut().find(|e| e.kind == right.kind) {
            Some(existing) => {
                let new_level = if existing.level == right.level {
                    (existing.level + 1).min(right.kind.max_level())
                } else {
                    existing.level.max(right.level)
                };
                existing.level = new_level;
            }
            None => {
                out.push(*right);
            }
        }
    }
    out
}

/// Répare un item durable : prend un autre item du même type pour fixer
/// la durability. Bonus : 12% de durability max par item utilisé.
pub fn repair_damage(current_damage: u32, max_durability: u32) -> u32 {
    let bonus = (max_durability as f32 * 0.12).round() as u32;
    current_damage.saturating_sub(bonus)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn combine_same_level_increments() {
        let left = vec![EnchantmentInstance::new(EnchantmentKind::Sharpness, 2)];
        let right = vec![EnchantmentInstance::new(EnchantmentKind::Sharpness, 2)];
        let combined = combine_items(&left, &right);
        assert_eq!(combined.len(), 1);
        assert_eq!(combined[0].level, 3);
    }

    #[test]
    fn combine_incompatible_keeps_left() {
        let left = vec![EnchantmentInstance::new(EnchantmentKind::Sharpness, 3)];
        let right = vec![EnchantmentInstance::new(EnchantmentKind::Smite, 2)];
        let combined = combine_items(&left, &right);
        assert_eq!(combined.len(), 1);
        assert_eq!(combined[0].kind, EnchantmentKind::Sharpness);
    }

    #[test]
    fn combine_higher_level_takes_higher() {
        let left = vec![EnchantmentInstance::new(EnchantmentKind::Sharpness, 2)];
        let right = vec![EnchantmentInstance::new(EnchantmentKind::Sharpness, 4)];
        let combined = combine_items(&left, &right);
        assert_eq!(combined[0].level, 4);
    }

    #[test]
    fn repair_reduces_damage_by_12pct() {
        let repaired = repair_damage(100, 250);
        // 12% of 250 = 30. 100 - 30 = 70.
        assert_eq!(repaired, 70);
    }
}
