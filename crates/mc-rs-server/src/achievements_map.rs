//! Achievement triggers — PMMP/Bedrock achievement hooks.
//! Mapping actions de gameplay → advancement unlocked.

use crate::advancements::AchievementKind;

/// Retourne un achievement à débloquer si l'action correspond.
pub fn achievement_for_action(action: GameplayAction) -> Option<AchievementKind> {
    match action {
        GameplayAction::OpenInventoryFirstTime => Some(AchievementKind::OpenInventory),
        GameplayAction::BreakWoodFirstTime => Some(AchievementKind::MineWood),
        GameplayAction::CraftWorkbench => Some(AchievementKind::BuildWorkbench),
        GameplayAction::CraftWoodPickaxe => Some(AchievementKind::BuildPickaxe),
        GameplayAction::CraftFurnace => Some(AchievementKind::BuildFurnace),
        GameplayAction::PickupIron => Some(AchievementKind::AcquireIron),
        GameplayAction::CraftHoe => Some(AchievementKind::BuildHoe),
        GameplayAction::MakeBread => Some(AchievementKind::MakeBread),
        GameplayAction::BakeCake => Some(AchievementKind::BakeCake),
        GameplayAction::CraftSword => Some(AchievementKind::BuildSword),
        GameplayAction::KillHostileMob => Some(AchievementKind::KillEnemy),
        GameplayAction::KillCow => Some(AchievementKind::KillCow),
        GameplayAction::DealDamageFar => Some(AchievementKind::DiamondsToYou),
        GameplayAction::CreateMap => Some(AchievementKind::MakeMap),
        GameplayAction::BuildPortal => Some(AchievementKind::PortalMaker),
        GameplayAction::DrinkPotion => Some(AchievementKind::PotionEffect),
        GameplayAction::SpawnWither => Some(AchievementKind::SpawnWither),
        GameplayAction::KillWither => Some(AchievementKind::KillWither),
        GameplayAction::FullBeacon => Some(AchievementKind::FullBeacon),
        GameplayAction::Overkill => Some(AchievementKind::Overkill),
        GameplayAction::Overpowered => Some(AchievementKind::Overpowered),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameplayAction {
    OpenInventoryFirstTime,
    BreakWoodFirstTime,
    CraftWorkbench,
    CraftWoodPickaxe,
    CraftFurnace,
    PickupIron,
    CraftHoe,
    MakeBread,
    BakeCake,
    CraftSword,
    KillHostileMob,
    KillCow,
    DealDamageFar,
    CreateMap,
    BuildPortal,
    DrinkPotion,
    SpawnWither,
    KillWither,
    FullBeacon,
    Overkill,
    Overpowered,
    OnARailFar,
    CookFish,
    BookcaseBuilder,
    ReturnToSender,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_inv_first_time_maps() {
        assert_eq!(
            achievement_for_action(GameplayAction::OpenInventoryFirstTime),
            Some(AchievementKind::OpenInventory)
        );
    }
}
