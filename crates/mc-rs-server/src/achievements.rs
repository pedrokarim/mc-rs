//! Achievements — vanilla list.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Achievement {
    OpenInventory,
    GetWood,
    Benchmarking,
    TimeToFarm,
    BakeBread,
    TheLie,        // Cake
    GetStone,
    TimeToMine,
    HotTopic,
    AcquireHardware,
    TimeToStrike,
    MonsterHunter,
    CowTipper,
    WhenPigsFly,
    SniperDuel,
    DiamondsToYou,
    OnTheWayDown,
    IntoTheNether,
    ReturnToSender,
    OverkillMonsterHunter,
    OverpoweredMonsterHunter,
    TheBeginning,
    TheEnd,
    TheBeginning2,
    LocalBrewery,
    TheEnd2,
    AdventuringTime,
    RepopulatingTheEndermen,
    OnARail,
    DiamondsForBlocks,
    LeaderOfThePack,
    PorkChop,
    Beaconator,
    Overpowered,
    Freight,
    Librarian,
    OvercomeBoss,
}

impl Achievement {
    pub fn parent(&self) -> Option<Achievement> {
        use Achievement::*;
        Some(match self {
            GetWood => OpenInventory,
            Benchmarking => GetWood,
            TimeToFarm => Benchmarking,
            BakeBread => TimeToFarm,
            TheLie => BakeBread,
            GetStone => Benchmarking,
            TimeToMine => GetStone,
            HotTopic => TimeToMine,
            AcquireHardware => HotTopic,
            TimeToStrike => Benchmarking,
            MonsterHunter => TimeToStrike,
            CowTipper => TimeToStrike,
            DiamondsToYou => DiamondsForBlocks,
            _ => return None,
        })
    }

    pub fn title(&self) -> &'static str {
        use Achievement::*;
        match self {
            OpenInventory => "Taking Inventory",
            GetWood => "Getting Wood",
            Benchmarking => "Benchmarking",
            TimeToFarm => "Time to Farm!",
            BakeBread => "Bake Bread",
            TheLie => "The Lie",
            GetStone => "Getting an Upgrade",
            TimeToMine => "Time to Mine!",
            HotTopic => "Hot Topic",
            AcquireHardware => "Acquire Hardware",
            TimeToStrike => "Time to Strike!",
            MonsterHunter => "Monster Hunter",
            CowTipper => "Cow Tipper",
            WhenPigsFly => "When Pigs Fly",
            SniperDuel => "Sniper Duel",
            DiamondsToYou => "DIAMONDS!",
            OnTheWayDown => "On The Way Down",
            IntoTheNether => "Into the Nether",
            ReturnToSender => "Return to Sender",
            OverkillMonsterHunter => "Overkill",
            OverpoweredMonsterHunter => "Overpowered",
            TheBeginning => "The Beginning?",
            TheEnd => "The End?",
            TheBeginning2 => "The Beginning.",
            LocalBrewery => "Local Brewery",
            TheEnd2 => "The End.",
            AdventuringTime => "Adventuring Time",
            RepopulatingTheEndermen => "Repopulating the Endermen",
            OnARail => "On A Rail",
            DiamondsForBlocks => "Diamonds to you!",
            LeaderOfThePack => "Leader of the Pack",
            PorkChop => "Pork Chop",
            Beaconator => "Beaconator",
            Overpowered => "Overpowered",
            Freight => "Freight Train",
            Librarian => "Librarian",
            OvercomeBoss => "Overkill",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_inv_root() {
        assert_eq!(Achievement::OpenInventory.parent(), None);
    }

    #[test]
    fn chains_through_tree() {
        assert_eq!(Achievement::MonsterHunter.parent(), Some(Achievement::TimeToStrike));
    }
}
