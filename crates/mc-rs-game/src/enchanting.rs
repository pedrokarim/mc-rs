//! Enchanting table mechanics: enchantability, bookshelf counting, option generation.

use crate::combat::{enchantment_id, ENCHANTMENT_LIST};

/// An enchantment option offered at the enchanting table.
#[derive(Debug, Clone)]
pub struct EnchantOption {
    /// Slot index (0, 1, or 2).
    pub slot: u8,
    /// XP level cost and lapis cost (1, 2, or 3).
    pub xp_cost: u8,
    /// Required enchantment level (from bookshelf formula).
    pub required_level: i32,
    /// Enchantments: (enchantment_id, level).
    pub enchantments: Vec<(i16, i16)>,
    /// Unique option ID for ItemStackRequest matching.
    pub option_id: u32,
}

/// Item category for enchantment table compatibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemCategory {
    Sword,
    Pickaxe,
    Axe,
    Shovel,
    Hoe,
    Helmet,
    Chestplate,
    Leggings,
    Boots,
    Bow,
    Crossbow,
    FishingRod,
    Trident,
    Book,
}

/// Get item enchantability value based on material type.
pub fn enchantability(item_name: &str) -> u8 {
    let name = item_name.strip_prefix("minecraft:").unwrap_or(item_name);

    // Material detection by prefix/name
    if name.contains("golden") || name.contains("gold") {
        return if is_armor_name(name) { 25 } else { 22 };
    }
    if name.contains("leather") {
        return 15;
    }
    if name.contains("netherite") {
        return 15;
    }
    if name.contains("wooden") || name.contains("wood") {
        return 15;
    }
    if name.contains("iron") {
        return if is_armor_name(name) { 9 } else { 14 };
    }
    if name.contains("chainmail") || name.contains("chain") {
        return 12;
    }
    if name.contains("diamond") {
        return 10;
    }
    if name.contains("stone") {
        return 5;
    }
    if name == "book" || name == "enchanted_book" {
        return 1;
    }
    if name == "bow" {
        return 1;
    }
    if name == "crossbow" {
        return 1;
    }
    if name == "fishing_rod" {
        return 1;
    }
    if name == "trident" {
        return 1;
    }
    if name.contains("turtle") {
        return 9;
    }

    0 // Not enchantable
}

fn is_armor_name(name: &str) -> bool {
    name.ends_with("helmet")
        || name.ends_with("chestplate")
        || name.ends_with("leggings")
        || name.ends_with("boots")
}

/// Check if an item is enchantable at the enchanting table.
pub fn is_enchantable(item_name: &str) -> bool {
    enchantability(item_name) > 0
}

fn categorize_item(item_name: &str) -> Option<ItemCategory> {
    let name = item_name.strip_prefix("minecraft:").unwrap_or(item_name);
    if name.ends_with("sword") {
        Some(ItemCategory::Sword)
    } else if name.ends_with("pickaxe") {
        Some(ItemCategory::Pickaxe)
    } else if name.ends_with("axe") && !name.ends_with("pickaxe") {
        Some(ItemCategory::Axe)
    } else if name.ends_with("shovel") {
        Some(ItemCategory::Shovel)
    } else if name.ends_with("hoe") {
        Some(ItemCategory::Hoe)
    } else if name.ends_with("helmet") || name.contains("turtle") {
        Some(ItemCategory::Helmet)
    } else if name.ends_with("chestplate") {
        Some(ItemCategory::Chestplate)
    } else if name.ends_with("leggings") {
        Some(ItemCategory::Leggings)
    } else if name.ends_with("boots") {
        Some(ItemCategory::Boots)
    } else if name == "bow" {
        Some(ItemCategory::Bow)
    } else if name == "crossbow" {
        Some(ItemCategory::Crossbow)
    } else if name == "fishing_rod" {
        Some(ItemCategory::FishingRod)
    } else if name == "trident" {
        Some(ItemCategory::Trident)
    } else if name == "book" || name == "enchanted_book" {
        Some(ItemCategory::Book)
    } else {
        None
    }
}

/// Enchantment weight (higher = more common in random selection).
struct EnchantWeight {
    id: i16,
    max_level: i16,
    weight: u32,
    /// Min modified level for level 1.
    min_level: i32,
    /// Range per level (min_level + (lvl-1) * level_step).
    level_step: i32,
}

/// Get the enchantments valid for a given item category.
fn valid_enchantments(cat: ItemCategory) -> Vec<EnchantWeight> {
    use enchantment_id::*;

    let mut list = Vec::new();

    // Helper to add an enchant
    macro_rules! add {
        ($id:expr, $max:expr, $w:expr, $min:expr, $step:expr) => {
            list.push(EnchantWeight {
                id: $id,
                max_level: $max,
                weight: $w,
                min_level: $min,
                level_step: $step,
            });
        };
    }

    match cat {
        ItemCategory::Sword => {
            add!(SHARPNESS, 5, 10, 1, 11);
            add!(SMITE, 5, 5, 5, 8);
            add!(BANE_OF_ARTHROPODS, 5, 5, 5, 8);
            add!(KNOCKBACK, 2, 5, 5, 20);
            add!(FIRE_ASPECT, 2, 2, 10, 20);
            add!(LOOTING, 3, 2, 15, 9);
            add!(UNBREAKING, 3, 5, 5, 8);
            add!(MENDING, 1, 2, 25, 25);
            add!(VANISHING_CURSE, 1, 1, 25, 25);
        }
        ItemCategory::Pickaxe | ItemCategory::Shovel | ItemCategory::Hoe => {
            add!(EFFICIENCY, 5, 10, 1, 10);
            add!(SILK_TOUCH, 1, 1, 15, 50);
            add!(FORTUNE, 3, 2, 15, 9);
            add!(UNBREAKING, 3, 5, 5, 8);
            add!(MENDING, 1, 2, 25, 25);
            add!(VANISHING_CURSE, 1, 1, 25, 25);
        }
        ItemCategory::Axe => {
            add!(SHARPNESS, 5, 10, 1, 11);
            add!(SMITE, 5, 5, 5, 8);
            add!(BANE_OF_ARTHROPODS, 5, 5, 5, 8);
            add!(EFFICIENCY, 5, 10, 1, 10);
            add!(SILK_TOUCH, 1, 1, 15, 50);
            add!(FORTUNE, 3, 2, 15, 9);
            add!(UNBREAKING, 3, 5, 5, 8);
            add!(MENDING, 1, 2, 25, 25);
            add!(VANISHING_CURSE, 1, 1, 25, 25);
        }
        ItemCategory::Helmet => {
            add!(PROTECTION, 4, 10, 1, 11);
            add!(FIRE_PROTECTION, 4, 5, 10, 8);
            add!(BLAST_PROTECTION, 4, 2, 5, 8);
            add!(PROJECTILE_PROTECTION, 4, 5, 3, 6);
            add!(RESPIRATION, 3, 2, 10, 10);
            add!(AQUA_AFFINITY, 1, 2, 1, 40);
            add!(THORNS, 3, 1, 10, 20);
            add!(UNBREAKING, 3, 5, 5, 8);
            add!(MENDING, 1, 2, 25, 25);
            add!(VANISHING_CURSE, 1, 1, 25, 25);
            add!(BINDING_CURSE, 1, 1, 25, 25);
        }
        ItemCategory::Chestplate | ItemCategory::Leggings => {
            add!(PROTECTION, 4, 10, 1, 11);
            add!(FIRE_PROTECTION, 4, 5, 10, 8);
            add!(BLAST_PROTECTION, 4, 2, 5, 8);
            add!(PROJECTILE_PROTECTION, 4, 5, 3, 6);
            add!(THORNS, 3, 1, 10, 20);
            add!(UNBREAKING, 3, 5, 5, 8);
            add!(MENDING, 1, 2, 25, 25);
            add!(VANISHING_CURSE, 1, 1, 25, 25);
            add!(BINDING_CURSE, 1, 1, 25, 25);
        }
        ItemCategory::Boots => {
            add!(PROTECTION, 4, 10, 1, 11);
            add!(FIRE_PROTECTION, 4, 5, 10, 8);
            add!(BLAST_PROTECTION, 4, 2, 5, 8);
            add!(PROJECTILE_PROTECTION, 4, 5, 3, 6);
            add!(FEATHER_FALLING, 4, 5, 5, 6);
            add!(DEPTH_STRIDER, 3, 2, 10, 10);
            add!(FROST_WALKER, 2, 2, 10, 10);
            add!(SOUL_SPEED, 3, 1, 10, 10);
            add!(THORNS, 3, 1, 10, 20);
            add!(UNBREAKING, 3, 5, 5, 8);
            add!(MENDING, 1, 2, 25, 25);
            add!(VANISHING_CURSE, 1, 1, 25, 25);
            add!(BINDING_CURSE, 1, 1, 25, 25);
        }
        ItemCategory::Bow => {
            add!(POWER, 5, 10, 1, 10);
            add!(PUNCH, 2, 2, 12, 20);
            add!(FLAME, 1, 2, 20, 30);
            add!(INFINITY, 1, 1, 20, 30);
            add!(UNBREAKING, 3, 5, 5, 8);
            add!(MENDING, 1, 2, 25, 25);
            add!(VANISHING_CURSE, 1, 1, 25, 25);
        }
        ItemCategory::Crossbow => {
            add!(MULTISHOT, 1, 2, 20, 25);
            add!(PIERCING, 4, 10, 1, 10);
            add!(QUICK_CHARGE, 3, 5, 12, 20);
            add!(UNBREAKING, 3, 5, 5, 8);
            add!(MENDING, 1, 2, 25, 25);
            add!(VANISHING_CURSE, 1, 1, 25, 25);
        }
        ItemCategory::FishingRod => {
            add!(LUCK_OF_THE_SEA, 3, 2, 15, 9);
            add!(LURE, 3, 2, 15, 9);
            add!(UNBREAKING, 3, 5, 5, 8);
            add!(MENDING, 1, 2, 25, 25);
            add!(VANISHING_CURSE, 1, 1, 25, 25);
        }
        ItemCategory::Trident => {
            add!(IMPALING, 5, 2, 1, 8);
            add!(RIPTIDE, 3, 2, 17, 7);
            add!(LOYALTY, 3, 5, 12, 7);
            add!(CHANNELING, 1, 1, 25, 25);
            add!(UNBREAKING, 3, 5, 5, 8);
            add!(MENDING, 1, 2, 25, 25);
            add!(VANISHING_CURSE, 1, 1, 25, 25);
        }
        ItemCategory::Book => {
            // Books can get any enchantment
            for info in ENCHANTMENT_LIST {
                add!(info.id, info.max_level, 5, 1, 10);
            }
        }
    }

    list
}

/// Mutually exclusive enchantment groups.
fn conflicts(a: i16, b: i16) -> bool {
    use enchantment_id::*;
    let protection_group = [
        PROTECTION,
        FIRE_PROTECTION,
        BLAST_PROTECTION,
        PROJECTILE_PROTECTION,
    ];
    let damage_group = [SHARPNESS, SMITE, BANE_OF_ARTHROPODS];
    let fortune_silk = [FORTUNE, SILK_TOUCH];
    let depth_frost = [DEPTH_STRIDER, FROST_WALKER];
    let infinity_mending = [INFINITY, MENDING];
    let riptide_loyalty = [RIPTIDE, LOYALTY, CHANNELING];
    let multishot_piercing = [MULTISHOT, PIERCING];

    for group in [
        &protection_group[..],
        &damage_group[..],
        &fortune_silk[..],
        &depth_frost[..],
        &infinity_mending[..],
        &riptide_loyalty[..],
        &multishot_piercing[..],
    ] {
        if group.contains(&a) && group.contains(&b) && a != b {
            return true;
        }
    }
    false
}

/// Simple seeded PRNG (xorshift32).
struct SeededRng {
    state: u32,
}

impl SeededRng {
    fn new(seed: i32) -> Self {
        let s = if seed == 0 { 1 } else { seed as u32 };
        Self { state: s }
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Random i32 in [min, max] inclusive.
    fn range(&mut self, min: i32, max: i32) -> i32 {
        if min >= max {
            return min;
        }
        let span = (max - min + 1) as u32;
        min + (self.next_u32() % span) as i32
    }
}

/// Count bookshelves in a 5×5 ring around the enchanting table.
///
/// Bookshelves must be exactly 2 blocks away (horizontally), at the same level
/// or one block above. The space between must be air.
pub fn count_bookshelves(
    tx: i32,
    ty: i32,
    tz: i32,
    get_block: impl Fn(i32, i32, i32) -> Option<u32>,
    is_bookshelf: impl Fn(u32) -> bool,
    is_air: impl Fn(u32) -> bool,
) -> u8 {
    let mut count = 0u8;

    for dy in 0..=1i32 {
        let y = ty + dy;
        for dx in -2..=2i32 {
            for dz in -2..=2i32 {
                // Only positions exactly on the 5×5 ring perimeter (distance 2)
                if dx.abs() != 2 && dz.abs() != 2 {
                    continue;
                }

                let bx = tx + dx;
                let bz = tz + dz;

                // Check if this position has a bookshelf
                if let Some(rid) = get_block(bx, y, bz) {
                    if !is_bookshelf(rid) {
                        continue;
                    }

                    // Check that the space between is air (at bookshelf height)
                    let mid_x = tx + dx.signum() * (dx.abs() - 1);
                    let mid_z = tz + dz.signum() * (dz.abs() - 1);
                    let air_ok = if dx.abs() == 2 && dz.abs() == 2 {
                        // Corner: check the diagonal path (just the inner cell)
                        get_block(mid_x, y, tz + dz.signum()).is_some_and(&is_air)
                            || get_block(tx + dx.signum(), y, mid_z).is_some_and(&is_air)
                    } else if dx.abs() == 2 {
                        get_block(mid_x, y, bz).is_some_and(&is_air)
                    } else {
                        get_block(bx, y, mid_z).is_some_and(&is_air)
                    };

                    if air_ok {
                        count += 1;
                        if count >= 15 {
                            return 15;
                        }
                    }
                }
            }
        }
    }

    count
}

/// Generate 3 enchantment options for a given item and enchantment context.
pub fn generate_options(seed: i32, bookshelves: u8, item_name: &str) -> Vec<EnchantOption> {
    let cat = match categorize_item(item_name) {
        Some(c) => c,
        None => return Vec::new(),
    };

    let ench = enchantability(item_name) as i32;
    if ench == 0 {
        return Vec::new();
    }

    let b = bookshelves.min(15) as i32;
    let mut rng = SeededRng::new(seed);

    // Base level calculation
    let base = rng.range(1, 8) + (b / 2) + rng.range(0, b);

    // Three slot level requirements
    let levels = [(base / 3).max(1), ((base * 2) / 3) + 1, base.max(b * 2)];

    let valid = valid_enchantments(cat);
    let mut options = Vec::new();
    let mut next_option_id = (seed.unsigned_abs() % 10000) + 1;

    for (i, &slot_level) in levels.iter().enumerate() {
        let xp_cost = (i as u8) + 1;

        // Modified level with enchantability bonus
        let bonus = rng.range(0, ench / 4) + rng.range(0, ench / 4) + 1;
        let modified = slot_level + bonus;

        // Triangular distribution factor (0.85 to 1.15)
        let factor_raw = rng.range(850, 1150);
        let final_level = ((modified as i64 * factor_raw as i64) / 1000) as i32;

        // Pick enchantments for this slot
        let enchantments = pick_enchantments(&mut rng, &valid, final_level);

        options.push(EnchantOption {
            slot: i as u8,
            xp_cost,
            required_level: slot_level,
            enchantments,
            option_id: next_option_id,
        });
        next_option_id += 1;
    }

    options
}

/// Pick enchantments using weighted random selection.
fn pick_enchantments(
    rng: &mut SeededRng,
    valid: &[EnchantWeight],
    modified_level: i32,
) -> Vec<(i16, i16)> {
    // Filter to enchantments that match this level
    let mut candidates: Vec<(i16, i16, u32)> = Vec::new(); // (id, level, weight)

    for ew in valid {
        // Find the highest level that fits
        for lvl in (1..=ew.max_level).rev() {
            let min_for_level = ew.min_level + (lvl as i32 - 1) * ew.level_step;
            let max_for_level = min_for_level + ew.level_step.max(15);
            if modified_level >= min_for_level && modified_level <= max_for_level {
                candidates.push((ew.id, lvl, ew.weight));
                break;
            }
        }
    }

    if candidates.is_empty() {
        // Fallback: pick the lowest level of a random enchantment
        if let Some(ew) = valid.first() {
            return vec![(ew.id, 1)];
        }
        return Vec::new();
    }

    let mut result = Vec::new();

    // First pick: weighted random
    let total_weight: u32 = candidates.iter().map(|c| c.2).sum();
    if total_weight == 0 {
        return Vec::new();
    }

    let mut roll = rng.next_u32() % total_weight;
    let mut first_idx = 0;
    for (i, c) in candidates.iter().enumerate() {
        if roll < c.2 {
            first_idx = i;
            break;
        }
        roll -= c.2;
    }
    let first = candidates.remove(first_idx);
    result.push((first.0, first.1));

    // Additional picks (50% chance per additional, max 2 extra)
    for _ in 0..2 {
        if rng.range(0, 1) == 0 || candidates.is_empty() {
            break;
        }

        // Remove conflicting candidates
        candidates
            .retain(|c| !conflicts(c.0, first.0) && !result.iter().any(|r| conflicts(r.0, c.0)));

        if candidates.is_empty() {
            break;
        }

        let total: u32 = candidates.iter().map(|c| c.2).sum();
        if total == 0 {
            break;
        }
        let mut r = rng.next_u32() % total;
        let mut idx = 0;
        for (i, c) in candidates.iter().enumerate() {
            if r < c.2 {
                idx = i;
                break;
            }
            r -= c.2;
        }
        let pick = candidates.remove(idx);
        result.push((pick.0, pick.1));
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enchantability_materials() {
        assert_eq!(enchantability("minecraft:diamond_sword"), 10);
        assert_eq!(enchantability("minecraft:golden_helmet"), 25);
        assert_eq!(enchantability("minecraft:iron_pickaxe"), 14);
        assert_eq!(enchantability("minecraft:wooden_axe"), 15);
        assert_eq!(enchantability("minecraft:stone_shovel"), 5);
        assert_eq!(enchantability("minecraft:leather_chestplate"), 15);
        assert_eq!(enchantability("minecraft:chainmail_boots"), 12);
        assert_eq!(enchantability("minecraft:netherite_sword"), 15);
        assert_eq!(enchantability("minecraft:book"), 1);
        assert_eq!(enchantability("minecraft:bow"), 1);
        assert_eq!(enchantability("minecraft:dirt"), 0);
    }

    #[test]
    fn is_enchantable_items() {
        assert!(is_enchantable("minecraft:diamond_sword"));
        assert!(is_enchantable("minecraft:book"));
        assert!(is_enchantable("minecraft:bow"));
        assert!(!is_enchantable("minecraft:stick"));
        assert!(!is_enchantable("minecraft:dirt"));
    }

    #[test]
    fn categorize_items() {
        assert_eq!(
            categorize_item("minecraft:diamond_sword"),
            Some(ItemCategory::Sword)
        );
        assert_eq!(
            categorize_item("minecraft:iron_pickaxe"),
            Some(ItemCategory::Pickaxe)
        );
        assert_eq!(
            categorize_item("minecraft:diamond_axe"),
            Some(ItemCategory::Axe)
        );
        assert_eq!(
            categorize_item("minecraft:iron_helmet"),
            Some(ItemCategory::Helmet)
        );
        assert_eq!(
            categorize_item("minecraft:leather_boots"),
            Some(ItemCategory::Boots)
        );
        assert_eq!(categorize_item("minecraft:book"), Some(ItemCategory::Book));
        assert_eq!(
            categorize_item("minecraft:trident"),
            Some(ItemCategory::Trident)
        );
        assert_eq!(categorize_item("minecraft:dirt"), None);
    }

    #[test]
    fn seeded_rng_deterministic() {
        let mut rng1 = SeededRng::new(42);
        let mut rng2 = SeededRng::new(42);
        for _ in 0..10 {
            assert_eq!(rng1.next_u32(), rng2.next_u32());
        }
    }

    #[test]
    fn seeded_rng_range() {
        let mut rng = SeededRng::new(123);
        for _ in 0..100 {
            let v = rng.range(1, 8);
            assert!((1..=8).contains(&v));
        }
    }

    #[test]
    fn count_bookshelves_empty() {
        // No bookshelves around
        let count = count_bookshelves(0, 64, 0, |_, _, _| Some(0), |_| false, |_| true);
        assert_eq!(count, 0);
    }

    #[test]
    fn count_bookshelves_max() {
        // Bookshelves everywhere (bookshelf=1, air=0)
        let count = count_bookshelves(0, 64, 0, |_, _, _| Some(1), |rid| rid == 1, |rid| rid == 0);
        // The ring has positions at distance 2, but not all have clear paths
        // With everything being bookshelf except nothing being air, 0 shelves counted
        // because the space between is also bookshelf (not air)
        assert_eq!(count, 0);
    }

    #[test]
    fn count_bookshelves_with_air() {
        // Bookshelves at distance 2 with air in between
        let count = count_bookshelves(
            0,
            64,
            0,
            |x, _y, z| {
                let dx = x.abs();
                let dz = z.abs();
                if dx == 2 || dz == 2 {
                    if dx <= 2 && dz <= 2 {
                        return Some(1); // bookshelf
                    }
                }
                Some(0) // air
            },
            |rid| rid == 1,
            |rid| rid == 0,
        );
        assert!(count > 0);
        assert!(count <= 15);
    }

    #[test]
    fn generate_options_valid() {
        let opts = generate_options(42, 15, "minecraft:diamond_sword");
        assert_eq!(opts.len(), 3);
        assert_eq!(opts[0].xp_cost, 1);
        assert_eq!(opts[1].xp_cost, 2);
        assert_eq!(opts[2].xp_cost, 3);
        for opt in &opts {
            assert!(!opt.enchantments.is_empty());
            for &(id, lvl) in &opt.enchantments {
                assert!(id >= 0);
                assert!(lvl >= 1);
            }
        }
    }

    #[test]
    fn generate_options_deterministic() {
        let opts1 = generate_options(42, 15, "minecraft:diamond_sword");
        let opts2 = generate_options(42, 15, "minecraft:diamond_sword");
        assert_eq!(opts1.len(), opts2.len());
        for (a, b) in opts1.iter().zip(opts2.iter()) {
            assert_eq!(a.enchantments, b.enchantments);
        }
    }

    #[test]
    fn generate_options_non_enchantable() {
        let opts = generate_options(42, 15, "minecraft:dirt");
        assert!(opts.is_empty());
    }

    #[test]
    fn generate_options_book() {
        let opts = generate_options(99, 10, "minecraft:book");
        assert_eq!(opts.len(), 3);
        // Books should get enchantments too
        for opt in &opts {
            assert!(!opt.enchantments.is_empty());
        }
    }

    #[test]
    fn conflicts_detection() {
        assert!(conflicts(
            enchantment_id::PROTECTION,
            enchantment_id::FIRE_PROTECTION
        ));
        assert!(conflicts(enchantment_id::SHARPNESS, enchantment_id::SMITE));
        assert!(conflicts(
            enchantment_id::FORTUNE,
            enchantment_id::SILK_TOUCH
        ));
        assert!(!conflicts(
            enchantment_id::SHARPNESS,
            enchantment_id::UNBREAKING
        ));
        assert!(!conflicts(
            enchantment_id::PROTECTION,
            enchantment_id::PROTECTION
        ));
    }

    #[test]
    fn valid_enchantments_sword() {
        let enchs = valid_enchantments(ItemCategory::Sword);
        let ids: Vec<i16> = enchs.iter().map(|e| e.id).collect();
        assert!(ids.contains(&enchantment_id::SHARPNESS));
        assert!(ids.contains(&enchantment_id::UNBREAKING));
        assert!(!ids.contains(&enchantment_id::EFFICIENCY)); // tools only
        assert!(!ids.contains(&enchantment_id::PROTECTION)); // armor only
    }
}
