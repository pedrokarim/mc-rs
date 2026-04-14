//! Big dripleaf + small dripleaf — tilts under weight.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DripleafKind {
    Small,
    Big,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DripleafTilt {
    None,
    Unstable,
    Partial,
    Full,
}

#[derive(Debug, Clone)]
pub struct Dripleaf {
    pub kind: DripleafKind,
    pub tilt: DripleafTilt,
    pub ticks_since_entity: u32,
}

/// Tilt progression: None → Unstable → Partial → Full (in 11 ticks).
pub const TILT_TICKS: u32 = 11;

impl Dripleaf {
    pub fn new(kind: DripleafKind) -> Self {
        Self { kind, tilt: DripleafTilt::None, ticks_since_entity: 0 }
    }

    pub fn step_on(&mut self) {
        self.tilt = DripleafTilt::Unstable;
        self.ticks_since_entity = 0;
    }

    pub fn tick(&mut self) {
        self.ticks_since_entity += 1;
        if matches!(self.tilt, DripleafTilt::Unstable | DripleafTilt::Partial) {
            if self.ticks_since_entity >= TILT_TICKS {
                self.tilt = match self.tilt {
                    DripleafTilt::Unstable => DripleafTilt::Partial,
                    DripleafTilt::Partial => DripleafTilt::Full,
                    _ => self.tilt,
                };
                self.ticks_since_entity = 0;
            }
        }
    }

    /// Full tilt — entity falls through.
    pub fn can_support_entity(&self) -> bool {
        self.tilt != DripleafTilt::Full
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilt_progresses() {
        let mut d = Dripleaf::new(DripleafKind::Big);
        d.step_on();
        for _ in 0..TILT_TICKS + 1 {
            d.tick();
        }
        assert_eq!(d.tilt, DripleafTilt::Partial);
    }
}
