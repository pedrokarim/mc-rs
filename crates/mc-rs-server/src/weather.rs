//! Weather — port de `.reference/PocketMine-MP/src/world/*` (weather partial).
//! Gère rain, thunder, transitions smooth, lightning strikes.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherKind {
    Clear,
    Rain,
    Thunder,
}

#[derive(Debug, Clone)]
pub struct WeatherState {
    pub current: WeatherKind,
    /// Intensité pluie (0.0 - 1.0) — interpolée progressivement.
    pub rain_intensity: f32,
    /// Intensité lightning (0.0 - 1.0).
    pub thunder_intensity: f32,
    /// Ticks restants avant changement de météo.
    pub time_until_change: u32,
    /// Ticks cumulés dans le state courant.
    pub elapsed_ticks: u32,
}

impl Default for WeatherState {
    fn default() -> Self {
        Self::new_clear()
    }
}

impl WeatherState {
    pub fn new_clear() -> Self {
        Self {
            current: WeatherKind::Clear,
            rain_intensity: 0.0,
            thunder_intensity: 0.0,
            time_until_change: 20 * 60 * 10, // 10 min default
            elapsed_ticks: 0,
        }
    }

    /// Tick (20 TPS). Retourne `true` si le state a changé.
    pub fn tick(&mut self) -> bool {
        self.elapsed_ticks = self.elapsed_ticks.wrapping_add(1);

        // Interpolation smooth.
        let target_rain = match self.current {
            WeatherKind::Clear => 0.0,
            WeatherKind::Rain | WeatherKind::Thunder => 1.0,
        };
        let target_thunder = match self.current {
            WeatherKind::Thunder => 1.0,
            _ => 0.0,
        };
        self.rain_intensity += (target_rain - self.rain_intensity) * 0.01;
        self.thunder_intensity += (target_thunder - self.thunder_intensity) * 0.01;

        if self.time_until_change == 0 {
            self.next();
            return true;
        }
        self.time_until_change -= 1;
        false
    }

    fn next(&mut self) {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        self.current = match self.current {
            WeatherKind::Clear => {
                if rng.gen_bool(0.3) {
                    WeatherKind::Thunder
                } else {
                    WeatherKind::Rain
                }
            }
            _ => WeatherKind::Clear,
        };
        self.elapsed_ticks = 0;
        self.time_until_change = match self.current {
            WeatherKind::Clear => rng.gen_range(12_000..=180_000), // 10-150 min
            WeatherKind::Rain => rng.gen_range(12_000..=24_000),
            WeatherKind::Thunder => rng.gen_range(3_600..=15_600),
        };
    }

    pub fn can_lightning_strike(&self) -> bool {
        matches!(self.current, WeatherKind::Thunder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initially_clear() {
        let w = WeatherState::new_clear();
        assert_eq!(w.current, WeatherKind::Clear);
        assert_eq!(w.rain_intensity, 0.0);
    }

    #[test]
    fn rain_intensity_increases_toward_target() {
        let mut w = WeatherState::new_clear();
        w.current = WeatherKind::Rain;
        for _ in 0..100 {
            w.tick();
        }
        assert!(w.rain_intensity > 0.5);
    }

    #[test]
    fn only_thunder_allows_lightning() {
        let mut w = WeatherState::new_clear();
        assert!(!w.can_lightning_strike());
        w.current = WeatherKind::Thunder;
        assert!(w.can_lightning_strike());
    }
}
