//! AI goal primitives — port PMMP (partial) + Minecraft goal system.
//! Chaque goal a une priorité ; le sélecteur choisit le plus prioritaire runnable.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiGoalKind {
    Panic, // flee quand damagé
    MeleeAttack,
    RangedAttack,
    FloatInWater,
    Tempt, // suivre joueur avec food
    AvoidEntity,
    LookAtPlayer,
    LookAround,
    MoveToBlock,
    WanderRandom,
    FollowOwner,
    Sit,
    Sleep,
    Breed,
    Eat,
    Croak,       // frog
    JumpInWater, // dolphin
    LayEgg,      // turtle
    Hurt,
    Target,
}

impl AiGoalKind {
    /// Priorité (0 = highest).
    pub fn priority(&self) -> u32 {
        match self {
            Self::Hurt | Self::Panic => 0,
            Self::Target => 1,
            Self::FloatInWater => 2,
            Self::MeleeAttack | Self::RangedAttack => 3,
            Self::AvoidEntity => 4,
            Self::Sit => 5,
            Self::Sleep => 5,
            Self::FollowOwner => 6,
            Self::Breed => 7,
            Self::Eat => 8,
            Self::Tempt => 9,
            Self::MoveToBlock => 10,
            Self::LayEgg | Self::JumpInWater | Self::Croak => 11,
            Self::WanderRandom => 12,
            Self::LookAtPlayer | Self::LookAround => 13,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GoalSelector {
    pub available_goals: Vec<AiGoalKind>,
    pub current_goal: Option<AiGoalKind>,
}

impl GoalSelector {
    pub fn new(goals: Vec<AiGoalKind>) -> Self {
        Self {
            available_goals: goals,
            current_goal: None,
        }
    }

    /// Choisi le goal le plus prioritaire parmi `runnable_goals`.
    pub fn select(&mut self, runnable_goals: Vec<AiGoalKind>) -> Option<AiGoalKind> {
        let best = runnable_goals
            .into_iter()
            .filter(|g| self.available_goals.contains(g))
            .min_by_key(|g| g.priority());
        self.current_goal = best;
        best
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hurt_is_highest_priority() {
        assert_eq!(AiGoalKind::Hurt.priority(), 0);
    }

    #[test]
    fn selects_higher_priority() {
        let mut gs = GoalSelector::new(vec![AiGoalKind::MeleeAttack, AiGoalKind::WanderRandom]);
        let g = gs.select(vec![AiGoalKind::WanderRandom, AiGoalKind::MeleeAttack]);
        assert_eq!(g, Some(AiGoalKind::MeleeAttack));
    }
}
