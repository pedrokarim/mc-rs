//! Block states — port PMMP `src/block/utils/*`.
//! Properties pour doors, trapdoors, buttons, pressure plates, fences, etc.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorHalf {
    Bottom,
    Top,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorHinge {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoorState {
    pub open: bool,
    pub half: DoorHalf,
    pub hinge: DoorHinge,
    pub facing: u8, // 0=N 1=E 2=S 3=W
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrapdoorState {
    pub open: bool,
    pub top_half: bool,
    pub facing: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlabType {
    Bottom,
    Top,
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StairShape {
    Straight,
    InnerLeft,
    InnerRight,
    OuterLeft,
    OuterRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StairState {
    pub shape: StairShape,
    pub facing: u8,
    pub top_half: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonState {
    pub pressed: bool,
    pub facing: u8,
    pub ticks_until_release: u32,
}

impl ButtonState {
    /// Cooldown : wood=30 ticks, stone=20 ticks.
    pub fn press(&mut self, is_wooden: bool) {
        self.pressed = true;
        self.ticks_until_release = if is_wooden { 30 } else { 20 };
    }

    pub fn tick(&mut self) {
        if self.ticks_until_release > 0 {
            self.ticks_until_release -= 1;
            if self.ticks_until_release == 0 {
                self.pressed = false;
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PressurePlateState {
    pub powered: bool,
    /// Entities actuellement dessus (count).
    pub entities_above: u32,
}

impl PressurePlateState {
    pub fn update(&mut self, new_entities_above: u32) {
        self.entities_above = new_entities_above;
        self.powered = new_entities_above > 0;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FenceGateState {
    pub open: bool,
    pub in_wall: bool,
    pub facing: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampfireState {
    pub lit: bool,
    pub signal_fire: bool, // signal fire when hay_bale below
    pub cooking_slots: [Option<i32>; 4], // 4 slots item IDs
    pub cook_times: [u16; 4],
}

impl CampfireState {
    pub fn new() -> Self {
        Self {
            lit: true,
            signal_fire: false,
            cooking_slots: [None; 4],
            cook_times: [0; 4],
        }
    }

    pub fn add_to_cook(&mut self, slot: usize, item_id: i32) -> bool {
        if slot < 4 && self.cooking_slots[slot].is_none() {
            self.cooking_slots[slot] = Some(item_id);
            self.cook_times[slot] = 0;
            true
        } else {
            false
        }
    }

    pub fn tick(&mut self, finished_cook_ticks: u16) -> Vec<(usize, i32)> {
        let mut done = Vec::new();
        if !self.lit {
            return done;
        }
        for i in 0..4 {
            if let Some(id) = self.cooking_slots[i] {
                self.cook_times[i] = self.cook_times[i].saturating_add(1);
                if self.cook_times[i] >= finished_cook_ticks {
                    done.push((i, id));
                    self.cooking_slots[i] = None;
                    self.cook_times[i] = 0;
                }
            }
        }
        done
    }
}

impl Default for CampfireState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stone_button_cooldown_20() {
        let mut b = ButtonState {
            pressed: false,
            facing: 0,
            ticks_until_release: 0,
        };
        b.press(false);
        assert_eq!(b.ticks_until_release, 20);
        for _ in 0..20 {
            b.tick();
        }
        assert!(!b.pressed);
    }

    #[test]
    fn wooden_button_cooldown_30() {
        let mut b = ButtonState {
            pressed: false,
            facing: 0,
            ticks_until_release: 0,
        };
        b.press(true);
        assert_eq!(b.ticks_until_release, 30);
    }

    #[test]
    fn campfire_cooks_and_finishes() {
        let mut c = CampfireState::new();
        c.add_to_cook(0, 100);
        let mut finished: Vec<(usize, i32)> = Vec::new();
        for _ in 0..600 {
            finished.extend(c.tick(600));
        }
        assert!(!finished.is_empty());
    }

    #[test]
    fn pressure_plate_powers_with_entity() {
        let mut p = PressurePlateState {
            powered: false,
            entities_above: 0,
        };
        p.update(1);
        assert!(p.powered);
        p.update(0);
        assert!(!p.powered);
    }
}
