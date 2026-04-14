//! Systèmes de survie — fall damage, fire tick, drowning, starvation.
//!
//! Ports sélectifs de `.reference/PocketMine-MP/src/entity/Living.php` et
//! `.reference/PocketMine-MP/src/entity/Entity.php` (tick physics helpers).
//!
//! Toutes les fonctions sont pures et prennent `&mut AttributeMap` + `&mut
//! CombatState` ou renvoient un `DamageRequest` que l'appelant convertit en
//! `combat::attack_entity` call.

use crate::attribute::{ids, AttributeMap};
use crate::event::entity::DamageCause;

// ── Fall damage ──────────────────────────────────────────────────────────────

/// Port de `Living.php::fall()` / `onHitGround()`.
/// Calcule le damage de chute à partir de la distance de chute verticale.
/// Formule PMMP : damage = max(0, fall_distance - 3.0).
pub fn fall_damage(fall_distance: f32) -> f32 {
    (fall_distance - 3.0).max(0.0)
}

/// État fall tracking par-entité.
#[derive(Debug, Clone, Default)]
pub struct FallState {
    /// Distance accumulée depuis la dernière collision sol.
    pub fall_distance: f32,
    /// Y précédent (pour calculer delta).
    pub last_y: f32,
    pub on_ground: bool,
    pub was_on_ground: bool,
}

impl FallState {
    pub fn new(initial_y: f32) -> Self {
        Self {
            fall_distance: 0.0,
            last_y: initial_y,
            on_ground: true,
            was_on_ground: true,
        }
    }

    /// À appeler quand la position change. Détecte landing → damage.
    /// Retourne `Some(damage)` si landing avec dégât de chute.
    pub fn update(&mut self, new_y: f32, on_ground: bool) -> Option<f32> {
        self.was_on_ground = self.on_ground;
        self.on_ground = on_ground;

        let delta_y = self.last_y - new_y;
        if delta_y > 0.0 && !self.on_ground {
            self.fall_distance += delta_y;
        }

        self.last_y = new_y;

        // Landing : fall_distance > 0 et on_ground passe false → true.
        if self.on_ground && !self.was_on_ground && self.fall_distance > 0.0 {
            let damage = fall_damage(self.fall_distance);
            self.fall_distance = 0.0;
            if damage > 0.0 {
                return Some(damage);
            }
        }
        // Reset accumulator si on_ground.
        if self.on_ground {
            self.fall_distance = 0.0;
        }
        None
    }
}

// ── Fire / burning ───────────────────────────────────────────────────────────

/// Port de `Entity.php::fireTicks` + `Living::fireTick()`.
/// Quand une entité est en feu, elle prend des dégâts périodiques.
#[derive(Debug, Clone, Default)]
pub struct FireState {
    /// Ticks restants avant fin du feu. 0 = pas en feu.
    pub fire_ticks: u32,
    /// Ticks depuis le dernier damage de feu.
    pub fire_damage_timer: u32,
}

impl FireState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Met le feu à l'entité pour `seconds * 20` ticks.
    pub fn set_fire_seconds(&mut self, seconds: u32) {
        let new_ticks = seconds * 20;
        if new_ticks > self.fire_ticks {
            self.fire_ticks = new_ticks;
        }
    }

    /// Éteint le feu.
    pub fn extinguish(&mut self) {
        self.fire_ticks = 0;
        self.fire_damage_timer = 0;
    }

    pub fn is_on_fire(&self) -> bool {
        self.fire_ticks > 0
    }

    /// À appeler chaque game tick (20 TPS). Retourne `Some(1.0)` si dégât feu
    /// tombe ce tick (toutes les 20 ticks = 1s).
    pub fn tick(&mut self) -> Option<f32> {
        if self.fire_ticks == 0 {
            return None;
        }
        self.fire_ticks -= 1;
        self.fire_damage_timer = self.fire_damage_timer.wrapping_add(1);
        if self.fire_damage_timer >= 20 {
            self.fire_damage_timer = 0;
            return Some(1.0); // 1 demi-cœur / seconde
        }
        None
    }
}

// ── Drowning ─────────────────────────────────────────────────────────────────

/// Port de `Living.php::breathing` + `airSupplyTicks`.
/// Quand une entité ne peut pas respirer (sous l'eau, dans du sable), elle
/// perd de l'air ; quand l'air est à 0 elle prend des dégâts de noyade.
#[derive(Debug, Clone)]
pub struct AirState {
    /// Ticks d'air restants. Max = 300 (15 secondes).
    pub air_ticks: i32,
    /// Max configurable (respiration enchant peut augmenter).
    pub max_air_ticks: i32,
    /// Timer pour le tick de damage (toutes les 20 ticks).
    pub drown_damage_timer: u32,
}

impl Default for AirState {
    fn default() -> Self {
        Self::new()
    }
}

impl AirState {
    pub const DEFAULT_MAX_AIR: i32 = 300;

    pub fn new() -> Self {
        Self {
            air_ticks: Self::DEFAULT_MAX_AIR,
            max_air_ticks: Self::DEFAULT_MAX_AIR,
            drown_damage_timer: 0,
        }
    }

    /// À appeler chaque game tick. `can_breathe` = true si l'entité est dans
    /// l'air libre. Retourne `Some(2.0)` si dégât de noyade ce tick.
    pub fn tick(&mut self, can_breathe: bool) -> Option<f32> {
        if can_breathe {
            // Recharge l'air par 4 ticks/tick (PMMP).
            self.air_ticks = (self.air_ticks + 4).min(self.max_air_ticks);
            self.drown_damage_timer = 0;
            return None;
        }
        self.air_ticks -= 1;
        if self.air_ticks < -20 {
            // Tick de drowning toutes les 20 ticks en dessous de 0.
            self.drown_damage_timer = self.drown_damage_timer.wrapping_add(1);
            if self.drown_damage_timer >= 20 {
                self.drown_damage_timer = 0;
                return Some(2.0); // 1 cœur / seconde
            }
        }
        None
    }

    pub fn reset(&mut self) {
        self.air_ticks = self.max_air_ticks;
        self.drown_damage_timer = 0;
    }
}

// ── Food / Consumable ───────────────────────────────────────────────────────

/// Port de `src/item/Food.php` + `Consumable.php`.
/// Description d'un item consommable (food, potion).
#[derive(Debug, Clone, Copy)]
pub struct ConsumableInfo {
    /// Nourriture ajoutée à HUNGER (0..20).
    pub food: f32,
    /// Saturation ajoutée (0..20).
    pub saturation: f32,
    /// Peut toujours être mangé (golden apple etc.), sinon seulement si faim < 20.
    pub can_always_eat: bool,
}

impl ConsumableInfo {
    pub fn food(food: f32, saturation: f32) -> Self {
        Self {
            food,
            saturation,
            can_always_eat: false,
        }
    }

    pub fn food_always(food: f32, saturation: f32) -> Self {
        Self {
            food,
            saturation,
            can_always_eat: true,
        }
    }
}

/// Lookup PMMP food items → nutrition + saturation.
/// Port de `ItemFactory` food registrations.
pub fn consumable_for(item_network_id: i32) -> Option<ConsumableInfo> {
    use crate::item_registry::required_item_id;
    let items: &[(&str, f32, f32, bool)] = &[
        // (name, food, saturation, always_eat)
        ("minecraft:apple", 4.0, 2.4, false),
        ("minecraft:baked_potato", 5.0, 6.0, false),
        ("minecraft:beef", 3.0, 1.8, false),
        ("minecraft:beetroot", 1.0, 1.2, false),
        ("minecraft:beetroot_soup", 6.0, 7.2, false),
        ("minecraft:bread", 5.0, 6.0, false),
        ("minecraft:carrot", 3.0, 3.6, false),
        ("minecraft:chicken", 2.0, 1.2, false),
        ("minecraft:chorus_fruit", 4.0, 2.4, true),
        ("minecraft:cooked_beef", 8.0, 12.8, false),
        ("minecraft:cooked_chicken", 6.0, 7.2, false),
        ("minecraft:cooked_cod", 5.0, 6.0, false),
        ("minecraft:cooked_mutton", 6.0, 9.6, false),
        ("minecraft:cooked_porkchop", 8.0, 12.8, false),
        ("minecraft:cooked_rabbit", 5.0, 6.0, false),
        ("minecraft:cooked_salmon", 6.0, 9.6, false),
        ("minecraft:cookie", 2.0, 0.4, false),
        ("minecraft:dried_kelp", 1.0, 0.6, false),
        ("minecraft:enchanted_golden_apple", 4.0, 9.6, true),
        ("minecraft:golden_apple", 4.0, 9.6, true),
        ("minecraft:golden_carrot", 6.0, 14.4, false),
        ("minecraft:melon_slice", 2.0, 1.2, false),
        ("minecraft:mushroom_stew", 6.0, 7.2, false),
        ("minecraft:mutton", 2.0, 1.2, false),
        ("minecraft:poisonous_potato", 2.0, 1.2, false),
        ("minecraft:porkchop", 3.0, 1.8, false),
        ("minecraft:potato", 1.0, 0.6, false),
        ("minecraft:pumpkin_pie", 8.0, 4.8, false),
        ("minecraft:rabbit", 3.0, 1.8, false),
        ("minecraft:rabbit_stew", 10.0, 12.0, false),
        ("minecraft:raw_cod", 2.0, 0.4, false),
        ("minecraft:raw_salmon", 2.0, 0.4, false),
        ("minecraft:rotten_flesh", 4.0, 0.8, false),
        ("minecraft:spider_eye", 2.0, 3.2, false),
        ("minecraft:steak", 8.0, 12.8, false),
        ("minecraft:sweet_berries", 2.0, 1.2, false),
        ("minecraft:tropical_fish", 1.0, 0.2, false),
    ];
    for (name, food, sat, always) in items {
        if required_item_id(name) == item_network_id {
            return Some(ConsumableInfo {
                food: *food,
                saturation: *sat,
                can_always_eat: *always,
            });
        }
    }
    None
}

/// Port PMMP `HungerManager::addFood()` + saturation.
/// Applique la consommation d'un food : ajoute food + saturation aux attrs.
pub fn consume_food(attrs: &mut AttributeMap, info: ConsumableInfo) -> bool {
    let hunger = attrs.must_get(ids::HUNGER).current_value;
    if hunger >= 20.0 && !info.can_always_eat {
        return false;
    }
    let new_hunger = (hunger + info.food).min(20.0);
    attrs.must_get_mut(ids::HUNGER).set_value(new_hunger, true);
    let sat = attrs.must_get(ids::SATURATION).current_value;
    let new_sat = (sat + info.saturation).min(new_hunger);
    attrs
        .must_get_mut(ids::SATURATION)
        .set_value(new_sat, true);
    true
}

// ── Helpers pour appliquer un DamageRequest au combat ────────────────────────

pub struct DamageRequest {
    pub cause: DamageCause,
    pub base_damage: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fall_damage_below_3_blocks_no_damage() {
        assert_eq!(fall_damage(0.0), 0.0);
        assert_eq!(fall_damage(1.5), 0.0);
        assert_eq!(fall_damage(3.0), 0.0);
    }

    #[test]
    fn fall_damage_above_3_blocks() {
        assert_eq!(fall_damage(4.0), 1.0);
        assert_eq!(fall_damage(10.0), 7.0);
        assert_eq!(fall_damage(23.0), 20.0);
    }

    #[test]
    fn fall_state_detects_landing() {
        // Chute verticale : 100 → 95 → 90 en l'air → 90 au sol.
        // Fall_distance total accumulé = 10. Damage = 10 - 3 = 7.
        let mut fs = FallState::new(100.0);
        assert_eq!(fs.update(95.0, false), None);
        assert_eq!(fs.update(90.0, false), None);
        let d = fs.update(90.0, true);
        assert_eq!(d, Some(7.0));
    }

    #[test]
    fn fire_ticks_apply_damage_every_second() {
        let mut fire = FireState::new();
        fire.set_fire_seconds(5);
        assert!(fire.is_on_fire());
        let mut damage_ticks = 0;
        for _ in 0..100 {
            if fire.tick().is_some() {
                damage_ticks += 1;
            }
        }
        assert_eq!(damage_ticks, 5);
    }

    #[test]
    fn drowning_starts_after_air_depletion() {
        let mut air = AirState::new();
        for _ in 0..300 {
            let _ = air.tick(false);
        }
        // Air should be 0 after 300 ticks no-breathe.
        assert_eq!(air.air_ticks, 0);
        // Continue 40 ticks; damage should trigger once after -20 ticks.
        let mut damage_count = 0;
        for _ in 0..60 {
            if air.tick(false).is_some() {
                damage_count += 1;
            }
        }
        assert!(damage_count >= 1);
    }

    #[test]
    fn consume_food_caps_at_20_hunger() {
        let mut attrs = AttributeMap::default_for_player();
        attrs.must_get_mut(ids::HUNGER).set_value(18.0, true);
        let consumed = consume_food(&mut attrs, ConsumableInfo::food(5.0, 6.0));
        assert!(consumed);
        assert_eq!(attrs.must_get(ids::HUNGER).current_value, 20.0);
    }

    #[test]
    fn consume_food_rejected_if_full() {
        let mut attrs = AttributeMap::default_for_player();
        assert_eq!(attrs.must_get(ids::HUNGER).current_value, 20.0);
        let consumed = consume_food(&mut attrs, ConsumableInfo::food(5.0, 6.0));
        assert!(!consumed);
    }

    #[test]
    fn consume_food_always_eat_bypasses() {
        let mut attrs = AttributeMap::default_for_player();
        let consumed = consume_food(&mut attrs, ConsumableInfo::food_always(4.0, 9.6));
        assert!(consumed);
    }
}
