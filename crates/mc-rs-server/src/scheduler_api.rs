//! Scheduler API — PMMP-like task scheduler.

#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub id: u64,
    pub tick_due: u64,
    pub period: Option<u32>, // None = one-shot, Some(N) = repeat every N ticks
    pub name: String,
}

impl ScheduledTask {
    pub fn one_shot(id: u64, due_tick: u64, name: impl Into<String>) -> Self {
        Self {
            id,
            tick_due: due_tick,
            period: None,
            name: name.into(),
        }
    }

    pub fn repeating(id: u64, first_due: u64, period: u32, name: impl Into<String>) -> Self {
        Self {
            id,
            tick_due: first_due,
            period: Some(period),
            name: name.into(),
        }
    }

    pub fn reschedule(&mut self) -> bool {
        if let Some(period) = self.period {
            self.tick_due += period as u64;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_shot_no_reschedule() {
        let mut t = ScheduledTask::one_shot(1, 100, "test");
        assert!(!t.reschedule());
    }

    #[test]
    fn repeating_reschedules() {
        let mut t = ScheduledTask::repeating(1, 100, 20, "test");
        assert!(t.reschedule());
        assert_eq!(t.tick_due, 120);
    }
}
