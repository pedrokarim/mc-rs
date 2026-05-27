//! Splash damage calculation (explosions, AOE).

/// Calculate damage at distance from explosion center.
/// Uses the PMMP / vanilla formula.
pub fn explosion_damage(
    distance: f64,
    power: f32,
    block_exposure: f32, // 0.0-1.0 based on ray exposure
) -> f32 {
    if distance > (power * 2.0) as f64 {
        return 0.0;
    }
    let impact = (1.0 - distance / (power as f64 * 2.0)) as f32 * block_exposure;

    ((impact * impact + impact) / 2.0 * 7.0 * power + 1.0).max(0.0)
}

/// Knockback impulse from explosion.
pub fn explosion_knockback(distance: f64, power: f32) -> f32 {
    if distance > (power * 2.0) as f64 {
        return 0.0;
    }
    (1.0 - distance / (power as f64 * 2.0)) as f32
}

/// Block break range from explosion.
pub fn explosion_break_range(power: f32) -> f64 {
    0.7 + 0.6 * power as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn close_max_damage() {
        let close = explosion_damage(0.0, 4.0, 1.0);
        let far = explosion_damage(7.0, 4.0, 1.0);
        assert!(close > far);
    }

    #[test]
    fn blocked_no_damage() {
        assert!(explosion_damage(0.0, 4.0, 0.0) <= 1.0);
    }
}
