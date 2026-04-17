//! Smoke from campfire / fire — decorative particles.

/// Max height of smoke column (8 blocks default, 24 with hay bale).
pub fn smoke_height(signal: bool) -> u32 {
    if signal {
        24
    } else {
        8
    }
}

/// Smoke duration per spawn tick (10 ticks).
pub const SMOKE_DURATION: u32 = 10;

/// Colored smoke (Bedrock) — tintings based on material beneath.
pub fn colored_smoke_for_block(block_below: u16) -> Option<u32> {
    match block_below {
        170 => Some(0xffff00), // yellow (hay bale)
        35 => Some(0xffffff),  // various wool variants
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_fire_tall() {
        assert!(smoke_height(true) > smoke_height(false));
    }
}
