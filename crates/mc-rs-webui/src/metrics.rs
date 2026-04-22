//! Mesure TPS (ticks per second) via fenêtre glissante 1 seconde.

use std::time::{Duration, Instant};

pub struct TpsTracker {
    window_start: Instant,
    tick_count_in_window: u64,
    last_tps: f32,
    total_ticks: u64,
}

impl Default for TpsTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TpsTracker {
    pub fn new() -> Self {
        Self {
            window_start: Instant::now(),
            tick_count_in_window: 0,
            last_tps: 0.0,
            total_ticks: 0,
        }
    }

    /// À appeler à chaque server tick. Renvoie le TPS courant (mis à jour
    /// chaque fois que la fenêtre de 1s est remplie).
    pub fn on_tick(&mut self) -> f32 {
        self.tick_count_in_window += 1;
        self.total_ticks += 1;
        let elapsed = self.window_start.elapsed();
        if elapsed >= Duration::from_secs(1) {
            self.last_tps = self.tick_count_in_window as f32 / elapsed.as_secs_f32();
            self.tick_count_in_window = 0;
            self.window_start = Instant::now();
        }
        self.last_tps
    }

    pub fn current_tps(&self) -> f32 {
        self.last_tps
    }

    pub fn total_ticks(&self) -> u64 {
        self.total_ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn tps_measured_after_window() {
        let mut t = TpsTracker::new();
        for _ in 0..50 {
            t.on_tick();
        }
        // Pas encore une seconde : TPS toujours à 0.
        assert_eq!(t.current_tps(), 0.0);
        sleep(Duration::from_millis(1050));
        t.on_tick();
        // Après 1s + un tick de plus : TPS doit être autour de 51.
        assert!(t.current_tps() > 40.0);
    }
}
