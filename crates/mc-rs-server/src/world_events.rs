//! World events / level events — port PMMP `LevelEventPacket::EVENT_*`.
//! Événements visuels/sonores non-entity (portail ouvert, expérience orb,
//! bubble column, ender eye, etc.).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum LevelEvent {
    // Sound (0-999)
    SoundClick = 1000,
    SoundClickFail = 1001,
    SoundShoot = 1002,
    SoundDoorOpen = 1003,
    SoundFizz = 1004,
    SoundIgnite = 1005,
    SoundGhast = 1007,
    SoundGhastShoot = 1008,
    SoundBlazeShoot = 1009,
    SoundDoorBump = 1010,
    SoundDoorClose = 1011,
    SoundExtinguish = 1012,
    SoundOrb = 1030,
    SoundPortal = 1032,
    SoundItemThrown = 1050,
    SoundPortalTravel = 1051,
    // Visual (2000-2999)
    ParticlesSmoke = 2000,
    ParticlesBlockBreak = 2001,
    ParticlesBlockForceField = 2002,
    ParticlesPotionSplash = 2003,
    ParticlesPlayerHurt = 2004,
    ParticlesBonemeal = 2005,
    ParticlesExplosion = 2006,
    ParticlesEvaporateWater = 2008,
    ParticlesBlockGlow = 2009,
    ParticlesTurtleEgg = 2010,
    ParticlesSculkCharge = 2011,
    ParticlesDragonBlood = 2020,
    ParticlesItemLeaderboard = 2021,
    // Blocks (3000-3999)
    StartRaining = 3001,
    StopRaining = 3002,
    StartThundering = 3003,
    StopThundering = 3004,
    GlobalPause = 3005,
    SimTimeStep = 3006,
    SimTimeScale = 3007,
    // Actor (4000-4999)
    ActorCauldronExplode = 4000,
    ActorCauldronDye = 4001,
    ActorCauldronCleanItem = 4002,
    ActorCauldronFillPotion = 4003,
    ActorCauldronTakePotion = 4004,
    ActorCauldronFillWater = 4006,
    ActorCauldronTakeWater = 4007,
    ActorCauldronAddDye = 4008,
    ActorCauldronCleanBanner = 4009,
    // Misc
    ParticleLegacyEvent = 0x4000,
    AnimationEvent = 0x8000,
}

impl LevelEvent {
    pub fn event_id(&self) -> i32 {
        *self as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_is_1000() {
        assert_eq!(LevelEvent::SoundClick.event_id(), 1000);
    }

    #[test]
    fn rain_event_3001() {
        assert_eq!(LevelEvent::StartRaining.event_id(), 3001);
    }
}
