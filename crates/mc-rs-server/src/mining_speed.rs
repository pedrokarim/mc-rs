//! Mining speed — port PMMP `src/item/Tool.php::getMiningEfficiency`.

use crate::durability::{ToolTier, ToolType};

/// Mining speed per second pour un bloc avec un outil donné.
/// Formule : base = tier_multiplier if tool type correct, else 1.0.
pub fn mining_speed(
    tool_tier: Option<ToolTier>,
    tool_type: Option<ToolType>,
    required_type: Option<ToolType>,
    efficiency_level: u8,
) -> f32 {
    let mut speed = match (tool_tier, tool_type, required_type) {
        (Some(tier), Some(ttype), Some(req)) if ttype == req => tier.mining_speed(),
        _ => 1.0,
    };
    if efficiency_level > 0 {
        // PMMP : speed += efficiency^2 + 1
        speed += (efficiency_level as f32).powi(2) + 1.0;
    }
    speed
}

/// Break ticks requis : hardness * (effective_multiplier) * 20.
pub fn break_time_ticks(hardness: f32, mining_speed_mult: f32, correct_tool: bool) -> u32 {
    if hardness <= 0.0 {
        return 0;
    }
    let divisor = if correct_tool { 30.0 } else { 100.0 };
    ((hardness * divisor) / mining_speed_mult).ceil() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wood_pickaxe_faster_than_fist_on_stone() {
        let pickaxe = mining_speed(
            Some(ToolTier::Wood),
            Some(ToolType::Pickaxe),
            Some(ToolType::Pickaxe),
            0,
        );
        let fist = mining_speed(None, None, Some(ToolType::Pickaxe), 0);
        assert!(pickaxe > fist);
    }

    #[test]
    fn efficiency_boosts_speed() {
        let e0 = mining_speed(
            Some(ToolTier::Iron),
            Some(ToolType::Pickaxe),
            Some(ToolType::Pickaxe),
            0,
        );
        let e5 = mining_speed(
            Some(ToolTier::Iron),
            Some(ToolType::Pickaxe),
            Some(ToolType::Pickaxe),
            5,
        );
        assert!(e5 > e0);
    }
}
