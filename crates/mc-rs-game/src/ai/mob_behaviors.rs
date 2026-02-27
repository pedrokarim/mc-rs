//! Per-mob-type behavior lists.

use super::behavior::Behavior;
use super::behaviors::*;

/// Create the behavior list for a given mob type.
pub fn create_behaviors(type_id: &str) -> Vec<Box<dyn Behavior>> {
    match type_id {
        "minecraft:zombie" | "minecraft:skeleton" => vec![
            Box::new(Float::new()),
            Box::new(HurtByTarget::new()),
            Box::new(NearestAttackableTarget::new(16.0)),
            Box::new(MeleeAttack::new(20)),
            Box::new(Panic::new()), // won't activate: attack_damage > 0
            Box::new(RandomStroll::new()),
            Box::new(LookAtPlayer::new(8.0)),
        ],
        "minecraft:cow" | "minecraft:pig" | "minecraft:chicken" => vec![
            Box::new(Float::new()),
            Box::new(Panic::new()),
            Box::new(TemptGoal::new()),
            Box::new(BreedGoal::new()),
            Box::new(RandomStroll::new()),
            Box::new(LookAtPlayer::new(8.0)),
        ],
        _ => vec![
            Box::new(RandomStroll::new()),
            Box::new(LookAtPlayer::new(8.0)),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::behavior::BehaviorType;

    #[test]
    fn zombie_has_target_selector() {
        let behaviors = create_behaviors("minecraft:zombie");
        assert!(behaviors
            .iter()
            .any(|b| b.behavior_type() == BehaviorType::TargetSelector));
    }

    #[test]
    fn cow_has_no_target_selector() {
        let behaviors = create_behaviors("minecraft:cow");
        assert!(!behaviors
            .iter()
            .any(|b| b.behavior_type() == BehaviorType::TargetSelector));
    }

    #[test]
    fn cow_has_tempt_and_breed() {
        let behaviors = create_behaviors("minecraft:cow");
        // Float, Panic, TemptGoal, BreedGoal, RandomStroll, LookAtPlayer = 6
        assert_eq!(behaviors.len(), 6);
    }

    #[test]
    fn unknown_gets_default_behaviors() {
        let behaviors = create_behaviors("minecraft:unknown");
        assert_eq!(behaviors.len(), 2);
    }
}
