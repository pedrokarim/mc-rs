//! Sound events registry — port de `LevelSoundEvent.php` PMMP.
//!
//! Bedrock utilise des IDs numériques pour les sound events. On liste les
//! principaux avec leurs IDs pour les broadcast via LevelSoundEventPacket.

/// PMMP `LevelSoundEventPacket::SOUND_*` constantes. Liste sélectionnée
/// (full list > 400). IDs stables depuis 1.19+.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum SoundEvent {
    ItemUseOn = 0,
    Hit = 1,
    Step = 2,
    Fly = 3,
    Jump = 4,
    Break = 5,
    Place = 6,
    HeavyStep = 7,
    Gallop = 8,
    Fall = 9,
    Ambient = 10,
    AmbientBaby = 11,
    AmbientAgressive = 12,
    AmbientWorried = 13,
    AmbientTame = 14,
    Death = 15,
    Hurt = 16,
    Pop = 17,
    Bow = 18,
    Splash = 19,
    Attack = 20,
    AttackNoDamage = 21,
    AttackStrong = 22,
    LargeBlast = 23,
    LargeBlastFarAway = 24,
    PortalCreated = 25,
    GhastWarning = 26,
    GhastFireball = 27,
    BlazeFireball = 28,
    FireSwap = 29,
    FireChargeUse = 30,
    BlastFireChargeUse = 31,
    BucketFillWater = 32,
    BucketFillLava = 33,
    BucketEmptyWater = 34,
    BucketEmptyLava = 35,
    EquipChain = 36,
    EquipDiamond = 37,
    EquipGeneric = 38,
    EquipGold = 39,
    EquipIron = 40,
    EquipLeather = 41,
    EquipElytra = 42,
    Record13 = 43,
    RecordCat = 44,
    RecordBlocks = 45,
    RecordChirp = 46,
    RecordFar = 47,
    RecordMall = 48,
    RecordMellohi = 49,
    RecordStal = 50,
    RecordStrad = 51,
    RecordWard = 52,
    Record11 = 53,
    RecordWait = 54,
    // Bows & crossbows
    CrossbowLoadingStart = 55,
    CrossbowLoadingMiddle = 56,
    CrossbowLoadingEnd = 57,
    CrossbowShoot = 58,
    // Notes
    NotePiano = 59,
    NoteBassAttack = 60,
    NoteBassDrum = 61,
    NoteSnare = 62,
    NoteHat = 63,
    NoteGuitar = 64,
    NoteFlute = 65,
    NoteBell = 66,
    NoteChime = 67,
    NoteXylophone = 68,
    NoteIronXylophone = 69,
    NoteCowBell = 70,
    NoteDidgeridoo = 71,
    NoteBit = 72,
    NoteBanjo = 73,
    NotePling = 74,
    // More common events
    GuardianAmbientLand = 75,
    GuardianAttackLoop = 76,
    GuardianAttackLoopOcean = 77,
    PlayerHurt = 78,
    PlayerHurtDrown = 79,
    PlayerHurtOnFire = 80,
    TotemUsed = 81,
    FireworkLaunch = 82,
    FireworkTwinkle = 83,
    FireworkBlast = 84,
    FireworkLargeBlast = 85,
    FireworkShoot = 86,
    ExperienceOrbPickup = 87,
    XpLevelUp = 88,
    ArmorEquipNetherite = 89,
    ShieldBlock = 90,
    Eat = 91,
    Drink = 92,
    TotemOfUndying = 93,
    CommandSay = 94,
    AnvilUse = 95,
    AnvilBreak = 96,
    AnvilLand = 97,
    Thunder = 98,
    Explode = 99,
}

impl SoundEvent {
    pub fn id(&self) -> u32 {
        *self as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_ids_are_stable() {
        assert_eq!(SoundEvent::Hit.id(), 1);
        assert_eq!(SoundEvent::XpLevelUp.id(), 88);
    }
}
