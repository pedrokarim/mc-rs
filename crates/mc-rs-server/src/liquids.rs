//! Liquids — port PMMP `src/block/Water.php` + `Lava.php` + `Liquid.php`.
//! Gère flow / spread, stills vs flowing, source blocks.

/// Hauteur de l'eau dans son bloc, en fraction (PMMP `Liquid::getFluidHeightPercent` :
/// (déclin + 1) / 9, déclin nul pour une eau qui tombe) ; `None` si le bloc n'est pas de l'eau.
///
/// `liquid_depth` (0 à 15 : déclin sur 3 bits, bit 3 = chute) est l'unique propriété de `water` et de
/// `flowing_water`, et la palette range ses états dans l'ordre des valeurs : l'état se lit par
/// décalage depuis le premier runtime ID du bloc (vérifié par le test `water_states_follow_liquid_depth`).
pub fn water_height_percent(runtime_id: u32) -> Option<f32> {
    use crate::world::block_registry::BLOCKS;
    let depth = ["minecraft:water", "minecraft:flowing_water"]
        .into_iter()
        .map(|name| BLOCKS.get(name))
        .find(|&first| (first..first + 16).contains(&runtime_id))
        .map(|first| runtime_id - first)?;
    let falling = depth & 0b1000 != 0;
    let decay = if falling { 0 } else { depth & 0b0111 };
    Some((decay as f32 + 1.0) / 9.0)
}

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

    #[test]
    fn water_states_follow_liquid_depth() {
        use mc_rs_nbt::NbtTag;
        let palette = include_bytes!("../data/canonical_block_states.nbt");
        let mut buf = &palette[..];
        let mut runtime_id = 0u32;
        let mut checked = 0;
        while !buf.is_empty() {
            let root = mc_rs_nbt::read_nbt_network(&mut buf).unwrap();
            if let Some(NbtTag::String(name)) = root.compound.get("name") {
                if name == "minecraft:water" || name == "minecraft:flowing_water" {
                    let Some(NbtTag::Compound(states)) = root.compound.get("states") else {
                        panic!("états absents");
                    };
                    assert_eq!(states.len(), 1);
                    let Some(NbtTag::Int(depth)) = states.get("liquid_depth") else {
                        panic!("liquid_depth absent");
                    };
                    let first = crate::world::block_registry::BLOCKS.get(name);
                    assert_eq!(runtime_id - first, *depth as u32);
                    checked += 1;
                }
            }
            runtime_id += 1;
        }
        assert_eq!(checked, 32);
        // Source : déclin 0 → 1/9 ; déclin 7 → 8/9 ; eau qui tombe → 1/9.
        let water = crate::world::block_registry::BLOCKS.get("minecraft:water");
        assert_eq!(water_height_percent(water), Some(1.0 / 9.0));
        assert_eq!(water_height_percent(water + 7), Some(8.0 / 9.0));
        assert_eq!(water_height_percent(water + 15), Some(1.0 / 9.0));
        assert_eq!(
            water_height_percent(crate::world::block_registry::BLOCKS.stone),
            None
        );
    }
}
