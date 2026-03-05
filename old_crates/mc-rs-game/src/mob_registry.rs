//! Mob type definitions.
//!
//! Provides a registry of known mob types with their stats and hitbox dimensions.
//! Supports both vanilla mobs (hardcoded) and custom mobs from behavior packs.

/// Mob category for spawn cap grouping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MobCategory {
    Passive,
    Hostile,
}

/// Definition of a mob type.
#[derive(Debug, Clone)]
pub struct MobDefinition {
    /// Bedrock identifier, e.g. `"minecraft:zombie"`.
    pub type_id: String,
    /// Display name, e.g. `"Zombie"`.
    pub display_name: String,
    pub category: MobCategory,
    pub max_health: f32,
    /// Base attack damage (0 for passive mobs).
    pub attack_damage: f32,
    pub movement_speed: f32,
    /// Bounding box width.
    pub bb_width: f32,
    /// Bounding box height.
    pub bb_height: f32,
}

/// Registry of supported mob types.
pub struct MobRegistry {
    mobs: Vec<MobDefinition>,
}

impl Default for MobRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl MobRegistry {
    /// Build the registry with all known mob types.
    pub fn new() -> Self {
        Self {
            mobs: vec![
                MobDefinition {
                    type_id: "minecraft:zombie".into(),
                    display_name: "Zombie".into(),
                    category: MobCategory::Hostile,
                    max_health: 20.0,
                    attack_damage: 3.0,
                    movement_speed: 0.23,
                    bb_width: 0.6,
                    bb_height: 1.95,
                },
                MobDefinition {
                    type_id: "minecraft:skeleton".into(),
                    display_name: "Skeleton".into(),
                    category: MobCategory::Hostile,
                    max_health: 20.0,
                    attack_damage: 2.0,
                    movement_speed: 0.25,
                    bb_width: 0.6,
                    bb_height: 1.99,
                },
                MobDefinition {
                    type_id: "minecraft:cow".into(),
                    display_name: "Cow".into(),
                    category: MobCategory::Passive,
                    max_health: 10.0,
                    attack_damage: 0.0,
                    movement_speed: 0.2,
                    bb_width: 0.9,
                    bb_height: 1.4,
                },
                MobDefinition {
                    type_id: "minecraft:pig".into(),
                    display_name: "Pig".into(),
                    category: MobCategory::Passive,
                    max_health: 10.0,
                    attack_damage: 0.0,
                    movement_speed: 0.25,
                    bb_width: 0.9,
                    bb_height: 0.9,
                },
                MobDefinition {
                    type_id: "minecraft:chicken".into(),
                    display_name: "Chicken".into(),
                    category: MobCategory::Passive,
                    max_health: 4.0,
                    attack_damage: 0.0,
                    movement_speed: 0.25,
                    bb_width: 0.4,
                    bb_height: 0.7,
                },
            ],
        }
    }

    /// Look up a mob definition by its Bedrock type identifier.
    pub fn get(&self, type_id: &str) -> Option<&MobDefinition> {
        self.mobs.iter().find(|m| m.type_id == type_id)
    }

    /// All known mob definitions.
    pub fn all(&self) -> &[MobDefinition] {
        &self.mobs
    }

    /// Register a custom mob type (e.g. from a behavior pack).
    pub fn register_mob(&mut self, def: MobDefinition) {
        self.mobs.push(def);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_has_5_mobs() {
        let reg = MobRegistry::new();
        assert_eq!(reg.all().len(), 5);
    }

    #[test]
    fn get_zombie() {
        let reg = MobRegistry::new();
        let z = reg.get("minecraft:zombie").unwrap();
        assert_eq!(z.display_name, "Zombie");
        assert_eq!(z.max_health, 20.0);
    }

    #[test]
    fn get_unknown_none() {
        let reg = MobRegistry::new();
        assert!(reg.get("minecraft:enderman").is_none());
    }

    #[test]
    fn zombie_hostile() {
        let reg = MobRegistry::new();
        let z = reg.get("minecraft:zombie").unwrap();
        assert_eq!(z.category, MobCategory::Hostile);
    }

    #[test]
    fn cow_passive() {
        let reg = MobRegistry::new();
        let c = reg.get("minecraft:cow").unwrap();
        assert_eq!(c.category, MobCategory::Passive);
    }

    #[test]
    fn register_custom_mob() {
        let mut reg = MobRegistry::new();
        reg.register_mob(MobDefinition {
            type_id: "custom:guard".into(),
            display_name: "Guard".into(),
            category: MobCategory::Hostile,
            max_health: 40.0,
            attack_damage: 5.0,
            movement_speed: 0.3,
            bb_width: 0.6,
            bb_height: 1.95,
        });
        assert_eq!(reg.all().len(), 6);
        let g = reg.get("custom:guard").unwrap();
        assert_eq!(g.max_health, 40.0);
    }
}
