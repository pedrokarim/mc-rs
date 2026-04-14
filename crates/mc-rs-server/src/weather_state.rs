//! Weather state machine.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherKind {
    Clear,
    Rain,
    Thunder,
}

#[derive(Debug, Clone)]
pub struct WeatherState {
    pub current: WeatherKind,
    pub duration_ticks: u32,
}

/// Clear weather duration (12000-168000 ticks = 10 min to 7 days).
pub const CLEAR_MIN: u32 = 12000;
pub const CLEAR_MAX: u32 = 168000;
/// Rain duration (12000-24000 ticks = 10-20 min).
pub const RAIN_MIN: u32 = 12000;
pub const RAIN_MAX: u32 = 24000;
/// Thunder duration (3600-15600 ticks = 3-13 min).
pub const THUNDER_MIN: u32 = 3600;
pub const THUNDER_MAX: u32 = 15600;

impl WeatherState {
    pub fn new() -> Self {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        Self {
            current: WeatherKind::Clear,
            duration_ticks: rng.gen_range(CLEAR_MIN..=CLEAR_MAX),
        }
    }

    pub fn tick(&mut self) {
        if self.duration_ticks > 0 {
            self.duration_ticks -= 1;
        } else {
            self.transition();
        }
    }

    fn transition(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        self.current = match self.current {
            WeatherKind::Clear => WeatherKind::Rain,
            WeatherKind::Rain => {
                if rng.gen::<f32>() < 0.3 {
                    WeatherKind::Thunder
                } else {
                    WeatherKind::Clear
                }
            }
            WeatherKind::Thunder => WeatherKind::Clear,
        };
        self.duration_ticks = match self.current {
            WeatherKind::Clear => rng.gen_range(CLEAR_MIN..=CLEAR_MAX),
            WeatherKind::Rain => rng.gen_range(RAIN_MIN..=RAIN_MAX),
            WeatherKind::Thunder => rng.gen_range(THUNDER_MIN..=THUNDER_MAX),
        };
    }
}

impl Default for WeatherState {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tick_reduces_duration() {
        let mut w = WeatherState::new();
        let before = w.duration_ticks;
        w.tick();
        assert!(w.duration_ticks < before || w.current != WeatherKind::Clear);
    }
}
