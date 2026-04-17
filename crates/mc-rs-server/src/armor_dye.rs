//! Leather armor dyeing.

#[derive(Debug, Clone, Copy)]
pub struct LeatherDyeColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Default leather color (brown).
pub const DEFAULT_LEATHER: LeatherDyeColor = LeatherDyeColor {
    r: 160,
    g: 101,
    b: 64,
};

/// Combine current color with new dye colors (vanilla averaging).
pub fn mix_dyes(base: LeatherDyeColor, dyes: &[LeatherDyeColor]) -> LeatherDyeColor {
    let mut r = base.r as u32;
    let mut g = base.g as u32;
    let mut b = base.b as u32;
    let mut count = 1u32;
    for d in dyes {
        r += d.r as u32;
        g += d.g as u32;
        b += d.b as u32;
        count += 1;
    }
    LeatherDyeColor {
        r: (r / count) as u8,
        g: (g / count) as u8,
        b: (b / count) as u8,
    }
}

/// Cauldron washes leather armor.
pub fn cauldron_washes() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mix_red_yellow() {
        let red = LeatherDyeColor { r: 255, g: 0, b: 0 };
        let yellow = LeatherDyeColor {
            r: 255,
            g: 255,
            b: 0,
        };
        let mixed = mix_dyes(red, &[yellow]);
        assert!(mixed.r > 200);
    }
}
