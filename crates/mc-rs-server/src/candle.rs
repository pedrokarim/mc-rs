//! Candle — up to 4 per block, stackable, dyeable.

#[derive(Debug, Clone)]
pub struct Candle {
    pub color: Option<u8>, // None = plain
    pub count: u8,         // 1-4
    pub lit: bool,
    pub on_cake: bool,
}

/// Light emission per candle lit.
pub const LIGHT_PER_CANDLE: u8 = 3;

impl Candle {
    pub fn new(color: Option<u8>, count: u8) -> Self {
        Self { color, count: count.clamp(1, 4), lit: false, on_cake: false }
    }

    pub fn light_emission(&self) -> u8 {
        if !self.lit {
            return 0;
        }
        (LIGHT_PER_CANDLE * self.count).min(15)
    }

    pub fn add_candle(&mut self) -> bool {
        if self.count >= 4 {
            return false;
        }
        self.count += 1;
        true
    }

    /// Extinguish with water/snow.
    pub fn extinguish(&mut self) {
        self.lit = false;
    }

    /// Light with flint & steel or fire charge.
    pub fn ignite(&mut self) {
        self.lit = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lit_emits_light() {
        let mut c = Candle::new(None, 1);
        c.ignite();
        assert!(c.light_emission() > 0);
    }

    #[test]
    fn unlit_no_light() {
        let c = Candle::new(None, 4);
        assert_eq!(c.light_emission(), 0);
    }

    #[test]
    fn max_4_candles() {
        let mut c = Candle::new(None, 4);
        assert!(!c.add_candle());
    }
}
