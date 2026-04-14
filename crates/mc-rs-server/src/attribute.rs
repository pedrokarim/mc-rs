//! Port de `.reference/PocketMine-MP/src/entity/Attribute.php` +
//! `AttributeFactory.php` + `AttributeMap.php`.
//!
//! Les attributs sont des valeurs numériques syncées au client via
//! `UpdateAttributesPacket` : santé, faim, vitesse, niveau d'XP, etc.

use std::collections::HashMap;

/// IDs d'attributs standards. PMMP `Attribute::XXX` (string constants
/// "minecraft:...").
pub mod ids {
    pub const ABSORPTION: &str = "minecraft:absorption";
    pub const SATURATION: &str = "minecraft:player.saturation";
    pub const EXHAUSTION: &str = "minecraft:player.exhaustion";
    pub const KNOCKBACK_RESISTANCE: &str = "minecraft:knockback_resistance";
    pub const HEALTH: &str = "minecraft:health";
    pub const MOVEMENT_SPEED: &str = "minecraft:movement";
    pub const FOLLOW_RANGE: &str = "minecraft:follow_range";
    pub const HUNGER: &str = "minecraft:player.hunger";
    pub const ATTACK_DAMAGE: &str = "minecraft:attack_damage";
    pub const EXPERIENCE_LEVEL: &str = "minecraft:player.level";
    pub const EXPERIENCE: &str = "minecraft:player.experience";
    pub const UNDERWATER_MOVEMENT: &str = "minecraft:underwater_movement";
    pub const LUCK: &str = "minecraft:luck";
    pub const FALL_DAMAGE: &str = "minecraft:fall_damage";
    pub const LAVA_MOVEMENT: &str = "minecraft:lava_movement";
    pub const HORSE_JUMP_STRENGTH: &str = "minecraft:horse.jump_strength";
    pub const ZOMBIE_SPAWN_REINFORCEMENTS: &str = "minecraft:zombie.spawn_reinforcements";
}

/// Port de `Attribute.php`. Représente un attribut unique avec bornes et
/// valeur courante, suivant s'il est désynchronisé (à re-envoyer au client).
#[derive(Debug, Clone)]
pub struct Attribute {
    pub id: String,
    pub min_value: f32,
    pub max_value: f32,
    pub default_value: f32,
    pub current_value: f32,
    /// Si true, cet attribut est envoyé au client via UpdateAttributesPacket.
    pub should_send: bool,
    pub desynchronized: bool,
}

impl Attribute {
    pub fn new(id: &str, min: f32, max: f32, default: f32, should_send: bool) -> Self {
        assert!(
            min <= max && default >= min && default <= max,
            "Invalid attribute range for {id}: min={min}, max={max}, default={default}"
        );
        Self {
            id: id.to_string(),
            min_value: min,
            max_value: max,
            default_value: default,
            current_value: default,
            should_send,
            desynchronized: true,
        }
    }

    pub fn set_value(&mut self, value: f32, fit: bool) {
        let v = if !fit {
            if value < self.min_value || value > self.max_value {
                panic!(
                    "Value {value} outside range [{}, {}] for {}",
                    self.min_value, self.max_value, self.id
                );
            }
            value
        } else {
            value.clamp(self.min_value, self.max_value)
        };
        if (self.current_value - v).abs() > f32::EPSILON {
            self.current_value = v;
            self.desynchronized = true;
        }
    }

    pub fn reset_to_default(&mut self) {
        let d = self.default_value;
        self.set_value(d, true);
    }

    pub fn mark_synchronized(&mut self) {
        self.desynchronized = false;
    }

    pub fn is_desynchronized(&self) -> bool {
        self.should_send && self.desynchronized
    }
}

/// Port de `AttributeMap.php`. Conteneur d'attributs par-entité.
#[derive(Debug, Clone, Default)]
pub struct AttributeMap {
    pub attrs: HashMap<String, Attribute>,
}

impl AttributeMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Équivalent `AttributeFactory::getInstance()->get()` : construit un
    /// AttributeMap avec les attributs par défaut PMMP pour un `Human` (joueur).
    pub fn default_for_player() -> Self {
        let mut map = Self::new();
        let f = f32::MAX;
        map.add(Attribute::new(ids::ABSORPTION, 0.0, f, 0.0, true));
        map.add(Attribute::new(ids::SATURATION, 0.0, 20.0, 20.0, true));
        map.add(Attribute::new(ids::EXHAUSTION, 0.0, 5.0, 0.0, false));
        map.add(Attribute::new(ids::KNOCKBACK_RESISTANCE, 0.0, 1.0, 0.0, true));
        map.add(Attribute::new(ids::HEALTH, 0.0, 20.0, 20.0, true));
        map.add(Attribute::new(ids::MOVEMENT_SPEED, 0.0, f, 0.1, true));
        map.add(Attribute::new(ids::FOLLOW_RANGE, 0.0, 2048.0, 16.0, false));
        map.add(Attribute::new(ids::HUNGER, 0.0, 20.0, 20.0, true));
        map.add(Attribute::new(ids::ATTACK_DAMAGE, 0.0, f, 1.0, false));
        map.add(Attribute::new(ids::EXPERIENCE_LEVEL, 0.0, 24791.0, 0.0, true));
        map.add(Attribute::new(ids::EXPERIENCE, 0.0, 1.0, 0.0, true));
        map.add(Attribute::new(ids::UNDERWATER_MOVEMENT, 0.0, f, 0.02, true));
        map.add(Attribute::new(ids::LUCK, -1024.0, 1024.0, 0.0, true));
        map.add(Attribute::new(ids::FALL_DAMAGE, 0.0, f, 1.0, true));
        map.add(Attribute::new(ids::LAVA_MOVEMENT, 0.0, f, 0.02, true));
        map
    }

    pub fn add(&mut self, attr: Attribute) {
        self.attrs.insert(attr.id.clone(), attr);
    }

    pub fn get(&self, id: &str) -> Option<&Attribute> {
        self.attrs.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Attribute> {
        self.attrs.get_mut(id)
    }

    pub fn must_get(&self, id: &str) -> &Attribute {
        self.attrs
            .get(id)
            .unwrap_or_else(|| panic!("Attribute {id} not registered"))
    }

    pub fn must_get_mut(&mut self, id: &str) -> &mut Attribute {
        self.attrs
            .get_mut(id)
            .unwrap_or_else(|| panic!("Attribute {id} not registered"))
    }

    /// Retourne les attributs désynchronisés (à envoyer via UpdateAttributes),
    /// et les marque comme synchronisés.
    pub fn drain_desync(&mut self) -> Vec<Attribute> {
        let mut out = Vec::new();
        for attr in self.attrs.values_mut() {
            if attr.is_desynchronized() {
                out.push(attr.clone());
                attr.mark_synchronized();
            }
        }
        out
    }
}

// ── Hunger management — port de `HungerManager.php` ──

/// Port de `HungerManager.php`.
/// Gère faim, saturation, exhaustion — décrément et régénération/damage.
#[derive(Debug, Clone)]
pub struct HungerManager {
    pub enabled: bool,
    /// Ticks depuis dernier food update (exhaustion accumulée en unités).
    pub food_tick_timer: i32,
}

impl HungerManager {
    pub fn new() -> Self {
        Self {
            enabled: true,
            food_tick_timer: 0,
        }
    }

    /// PMMP `HungerManager::exhaust()` : ajoute de l'exhaustion. À 5.0, diminue
    /// la saturation ou la faim.
    pub fn exhaust(&self, attrs: &mut AttributeMap, amount: f32) {
        let ex = attrs.must_get_mut(ids::EXHAUSTION);
        let mut new_ex = ex.current_value + amount;
        while new_ex >= 5.0 {
            new_ex -= 5.0;
            // Saturation d'abord.
            let sat = attrs.must_get(ids::SATURATION).current_value;
            if sat > 0.0 {
                attrs
                    .must_get_mut(ids::SATURATION)
                    .set_value((sat - 1.0).max(0.0), true);
            } else {
                let hunger = attrs.must_get(ids::HUNGER).current_value;
                attrs
                    .must_get_mut(ids::HUNGER)
                    .set_value((hunger - 1.0).max(0.0), true);
            }
        }
        attrs.must_get_mut(ids::EXHAUSTION).set_value(new_ex, true);
    }

    /// À appeler chaque game tick (20 TPS). Décrémente passivement, ou
    /// regénère la santé si saturé.
    /// PMMP `HungerManager::onTick()` (partiel).
    pub fn tick(&mut self, attrs: &mut AttributeMap, difficulty: i32) {
        if !self.enabled {
            return;
        }
        self.food_tick_timer = (self.food_tick_timer + 1).min(100);
        if self.food_tick_timer < 80 {
            return;
        }
        self.food_tick_timer = 0;

        let hunger = attrs.must_get(ids::HUNGER).current_value;
        let _sat = attrs.must_get(ids::SATURATION).current_value;
        let health = attrs.must_get(ids::HEALTH).current_value;
        let max_health = attrs.must_get(ids::HEALTH).max_value;

        // Regen si faim ≥ 18 et vie < max.
        if hunger >= 18.0 && health < max_health && health > 0.0 {
            attrs
                .must_get_mut(ids::HEALTH)
                .set_value(health + 1.0, true);
            self.exhaust(attrs, 6.0); // 6 exhaustion per heal
        } else if hunger <= 0.0 && health > 1.0 {
            // Starvation damage si faim nulle.
            let min_health = match difficulty {
                0 => f32::MAX, // peaceful: no starvation damage beyond 10
                1 => 10.0,     // easy
                2 => 1.0,      // normal
                _ => 0.0,      // hard
            };
            if health > min_health {
                attrs
                    .must_get_mut(ids::HEALTH)
                    .set_value((health - 1.0).max(0.0), true);
            }
        }
    }
}

impl Default for HungerManager {
    fn default() -> Self {
        Self::new()
    }
}

// ── Experience management — port de `ExperienceManager.php` (simplifié) ──

/// Port partiel de `ExperienceManager.php`. Level + progress (0..1).
/// La formule PMMP pour xp-to-next-level est :
///   level < 16 : 2*level + 7
///   level < 31 : 5*level - 38
///   level >= 31: 9*level - 158
#[derive(Debug, Clone)]
pub struct ExperienceManager;

impl ExperienceManager {
    /// XP requis pour passer du niveau `level` au `level+1`.
    pub fn xp_to_next_level(level: i32) -> i32 {
        if level < 16 {
            2 * level + 7
        } else if level < 31 {
            5 * level - 38
        } else {
            9 * level - 158
        }
    }

    /// Ajoute `amount` XP à un joueur. Met à jour level + progress.
    /// Retourne le nouveau (level, progress).
    pub fn add_xp(attrs: &mut AttributeMap, amount: i32) -> (i32, f32) {
        if amount <= 0 {
            let lvl = attrs.must_get(ids::EXPERIENCE_LEVEL).current_value as i32;
            let p = attrs.must_get(ids::EXPERIENCE).current_value;
            return (lvl, p);
        }
        let mut level = attrs.must_get(ids::EXPERIENCE_LEVEL).current_value as i32;
        let mut progress = attrs.must_get(ids::EXPERIENCE).current_value;
        let mut xp_left = amount;
        loop {
            let to_next = Self::xp_to_next_level(level) as f32;
            let current_xp = progress * to_next;
            let remaining_for_level = (to_next - current_xp).ceil() as i32;
            if xp_left < remaining_for_level {
                progress = (current_xp + xp_left as f32) / to_next;
                break;
            }
            xp_left -= remaining_for_level;
            level += 1;
            progress = 0.0;
        }
        attrs
            .must_get_mut(ids::EXPERIENCE_LEVEL)
            .set_value(level as f32, true);
        attrs
            .must_get_mut(ids::EXPERIENCE)
            .set_value(progress, true);
        (level, progress)
    }

    pub fn remove_xp(attrs: &mut AttributeMap, amount: i32) -> (i32, f32) {
        if amount <= 0 {
            let lvl = attrs.must_get(ids::EXPERIENCE_LEVEL).current_value as i32;
            let p = attrs.must_get(ids::EXPERIENCE).current_value;
            return (lvl, p);
        }
        let mut level = attrs.must_get(ids::EXPERIENCE_LEVEL).current_value as i32;
        let mut progress = attrs.must_get(ids::EXPERIENCE).current_value;
        let mut xp_left = amount;
        loop {
            let to_curr = Self::xp_to_next_level((level - 1).max(0)) as f32;
            let current_xp = progress * Self::xp_to_next_level(level) as f32;
            if xp_left as f32 <= current_xp {
                progress -= xp_left as f32 / Self::xp_to_next_level(level) as f32;
                if progress < 0.0 {
                    progress = 0.0;
                }
                break;
            }
            xp_left -= current_xp as i32;
            level -= 1;
            if level < 0 {
                level = 0;
                progress = 0.0;
                break;
            }
            progress = 1.0 - (1.0 / to_curr);
        }
        attrs
            .must_get_mut(ids::EXPERIENCE_LEVEL)
            .set_value(level as f32, true);
        attrs
            .must_get_mut(ids::EXPERIENCE)
            .set_value(progress.clamp(0.0, 1.0), true);
        (level, progress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_player_attrs_set() {
        let attrs = AttributeMap::default_for_player();
        assert_eq!(attrs.must_get(ids::HEALTH).current_value, 20.0);
        assert_eq!(attrs.must_get(ids::HUNGER).current_value, 20.0);
        assert_eq!(attrs.must_get(ids::SATURATION).current_value, 20.0);
        assert_eq!(attrs.must_get(ids::MOVEMENT_SPEED).current_value, 0.1);
    }

    #[test]
    fn exhaust_decrements_saturation_first_then_hunger() {
        let mgr = HungerManager::new();
        let mut attrs = AttributeMap::default_for_player();
        mgr.exhaust(&mut attrs, 5.0);
        assert_eq!(attrs.must_get(ids::SATURATION).current_value, 19.0);
        assert_eq!(attrs.must_get(ids::HUNGER).current_value, 20.0);
        // drain saturation puis faim
        for _ in 0..25 {
            mgr.exhaust(&mut attrs, 5.0);
        }
        assert_eq!(attrs.must_get(ids::SATURATION).current_value, 0.0);
        assert!(attrs.must_get(ids::HUNGER).current_value < 20.0);
    }

    #[test]
    fn xp_to_next_level_matches_pmmp() {
        assert_eq!(ExperienceManager::xp_to_next_level(0), 7);
        assert_eq!(ExperienceManager::xp_to_next_level(15), 37);
        assert_eq!(ExperienceManager::xp_to_next_level(16), 42);
        assert_eq!(ExperienceManager::xp_to_next_level(30), 112);
        assert_eq!(ExperienceManager::xp_to_next_level(31), 121);
    }

    #[test]
    fn add_xp_levels_up() {
        let mut attrs = AttributeMap::default_for_player();
        let (lvl, _) = ExperienceManager::add_xp(&mut attrs, 7);
        assert_eq!(lvl, 1);
    }
}
