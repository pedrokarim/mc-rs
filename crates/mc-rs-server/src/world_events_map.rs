//! World event IDs (levelevent packet).

pub const SOUND_CLICK: i32 = 1000;
pub const SOUND_CLICK_FAIL: i32 = 1001;
pub const SOUND_SHOOT: i32 = 1002;
pub const SOUND_DOOR: i32 = 1003;
pub const SOUND_FIZZ: i32 = 1004;
pub const SOUND_IGNITE: i32 = 1005;
pub const SOUND_GHAST: i32 = 1006;
pub const SOUND_GHAST_SHOOT: i32 = 1007;
pub const SOUND_BLAZE_SHOOT: i32 = 1008;
pub const SOUND_ZOMBIE_BASH: i32 = 1009;
pub const SOUND_ZOMBIE_DOOR_BREAK: i32 = 1010;

pub const PARTICLE_SHOOT: i32 = 2000;
pub const PARTICLE_DESTROY: i32 = 2001;
pub const PARTICLE_SPLASH: i32 = 2002;
pub const PARTICLE_EYE_DESPAWN: i32 = 2003;
pub const PARTICLE_SPAWN: i32 = 2004;
pub const PARTICLE_BONEMEAL: i32 = 2005;
pub const PARTICLE_GUARDIAN: i32 = 2006;

pub const EVENT_START_RAIN: i32 = 3001;
pub const EVENT_START_THUNDER: i32 = 3002;
pub const EVENT_STOP_RAIN: i32 = 3003;
pub const EVENT_STOP_THUNDER: i32 = 3004;
pub const EVENT_GLOBAL_PAUSE: i32 = 3005;
pub const EVENT_SIM_TIME_STEP: i32 = 3006;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_unique() {
        assert_ne!(EVENT_START_RAIN, EVENT_STOP_RAIN);
    }
}
