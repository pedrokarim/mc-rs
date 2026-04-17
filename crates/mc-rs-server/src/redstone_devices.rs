//! Redstone devices — buttons, levers, pressure plates, tripwire, rails, lamp, bell, etc.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonMaterial {
    Wood,     // 30 ticks
    Stone,    // 20 ticks
    Polished, // 20 ticks (polished blackstone)
}

#[derive(Debug, Clone)]
pub struct Button {
    pub material: ButtonMaterial,
    pub pressed_ticks: u32,
}

impl Button {
    pub fn new(material: ButtonMaterial) -> Self {
        Self {
            material,
            pressed_ticks: 0,
        }
    }

    pub fn press(&mut self) {
        self.pressed_ticks = match self.material {
            ButtonMaterial::Wood => 30,
            _ => 20,
        };
    }

    pub fn is_pressed(&self) -> bool {
        self.pressed_ticks > 0
    }

    pub fn tick(&mut self) {
        if self.pressed_ticks > 0 {
            self.pressed_ticks -= 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct Lever {
    pub powered: bool,
    pub facing: u8,
}

impl Lever {
    pub fn new() -> Self {
        Self {
            powered: false,
            facing: 0,
        }
    }

    pub fn toggle(&mut self) -> bool {
        self.powered = !self.powered;
        self.powered
    }
}

impl Default for Lever {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PressurePlateKind {
    Wood,              // Player + mob + items
    Stone,             // Player + mob only
    GoldHeavyWeighted, // Emits proportional to items
    IronLightWeighted, // Lighter threshold
}

#[derive(Debug, Clone)]
pub struct PressurePlate {
    pub kind: PressurePlateKind,
    pub entities_on: u32,
    pub output_power: u8,
}

impl PressurePlate {
    pub fn new(kind: PressurePlateKind) -> Self {
        Self {
            kind,
            entities_on: 0,
            output_power: 0,
        }
    }

    pub fn activate_for(&mut self, entity_count: u32) {
        self.entities_on = entity_count;
        self.output_power = match self.kind {
            PressurePlateKind::Wood | PressurePlateKind::Stone => {
                if entity_count > 0 {
                    15
                } else {
                    0
                }
            }
            PressurePlateKind::GoldHeavyWeighted => ((entity_count + 3) / 4).min(15) as u8,
            PressurePlateKind::IronLightWeighted => (entity_count).min(15) as u8,
        };
    }
}

#[derive(Debug, Clone)]
pub struct Bell {
    pub ringing_ticks: u32,
    pub facing: u8,
}

impl Bell {
    pub fn new() -> Self {
        Self {
            ringing_ticks: 0,
            facing: 0,
        }
    }

    pub fn ring(&mut self) {
        self.ringing_ticks = 60; // 3 seconds
    }

    pub fn is_ringing(&self) -> bool {
        self.ringing_ticks > 0
    }

    pub fn tick(&mut self) {
        if self.ringing_ticks > 0 {
            self.ringing_ticks -= 1;
        }
    }

    /// Reveal raid invaders nearby.
    pub fn raid_reveal_range() -> f64 {
        32.0
    }
}

impl Default for Bell {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stone_button_20_ticks() {
        let mut b = Button::new(ButtonMaterial::Stone);
        b.press();
        assert_eq!(b.pressed_ticks, 20);
    }

    #[test]
    fn heavy_plate_divides_by_4() {
        let mut p = PressurePlate::new(PressurePlateKind::GoldHeavyWeighted);
        p.activate_for(4);
        assert_eq!(p.output_power, 1);
    }

    #[test]
    fn lever_toggles() {
        let mut l = Lever::new();
        assert!(l.toggle());
        assert!(!l.toggle());
    }

    #[test]
    fn bell_rings_briefly() {
        let mut b = Bell::new();
        b.ring();
        assert!(b.is_ringing());
    }
}
