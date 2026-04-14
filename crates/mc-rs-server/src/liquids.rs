//! Liquids — port PMMP `src/block/Water.php` + `Lava.php` + `Liquid.php`.
//! Gère flow / spread, stills vs flowing, source blocks.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiquidKind {
    Water,
    Lava,
}

impl LiquidKind {
    /// Vitesse de flow (ticks par move). Water=5, Lava=30 (overworld) ou 10 (nether).
    pub fn flow_tick_rate(&self, in_nether: bool) -> u32 {
        match self {
            Self::Water => 5,
            Self::Lava => {
                if in_nether {
                    10
                } else {
                    30
                }
            }
        }
    }

    /// Maximum distance for flow spread (blocks).
    pub fn flow_distance(&self) -> u32 {
        match self {
            Self::Water => 8,
            Self::Lava => 4,
        }
    }

    /// Damage per tick quand entité est dedans.
    pub fn contact_damage(&self) -> f32 {
        match self {
            Self::Water => 0.0,
            Self::Lava => 4.0,
        }
    }

    /// Fire ticks appliqués quand entité sort. Lava seulement.
    pub fn fire_ticks_on_exit(&self) -> u32 {
        match self {
            Self::Lava => 15 * 20, // 15s
            _ => 0,
        }
    }

    pub fn can_be_source(&self, level: u8) -> bool {
        level == 0
    }
}

/// État d'un bloc liquid : 0 = source, 1-7 = flowing water, 1-3 = flowing lava (pas 4-7).
#[derive(Debug, Clone, Copy)]
pub struct LiquidLevel(pub u8);

impl LiquidLevel {
    pub fn is_source(&self) -> bool {
        self.0 == 0
    }

    pub fn is_flowing(&self) -> bool {
        !self.is_source()
    }

    /// Prochain niveau de flow après `self`. Retourne None si exhausted.
    pub fn next_for(&self, kind: LiquidKind) -> Option<Self> {
        let max = kind.flow_distance();
        let next = self.0 + 1;
        if next >= max as u8 {
            None
        } else {
            Some(Self(next))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn water_flows_faster_than_lava() {
        assert!(LiquidKind::Water.flow_tick_rate(false) < LiquidKind::Lava.flow_tick_rate(false));
    }

    #[test]
    fn lava_damages() {
        assert_eq!(LiquidKind::Lava.contact_damage(), 4.0);
        assert_eq!(LiquidKind::Water.contact_damage(), 0.0);
    }

    #[test]
    fn water_max_flow_8() {
        assert_eq!(LiquidKind::Water.flow_distance(), 8);
    }

    #[test]
    fn flow_level_advances() {
        let l = LiquidLevel(3);
        let next = l.next_for(LiquidKind::Water).unwrap();
        assert_eq!(next.0, 4);
    }
}
