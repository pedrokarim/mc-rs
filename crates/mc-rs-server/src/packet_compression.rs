//! Packet compression (zlib/snappy).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionAlgorithm {
    None = 0,
    Zlib = 1,
    Snappy = 2,
}

/// Threshold below which packets are not compressed.
pub const DEFAULT_THRESHOLD: u32 = 256;

/// Compress only if message is >= threshold.
pub fn should_compress(size: usize, threshold: u32) -> bool {
    size >= threshold as usize
}

/// Zlib compression level.
pub const ZLIB_LEVEL: u8 = 7;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn below_threshold_no_compress() {
        assert!(!should_compress(100, DEFAULT_THRESHOLD));
    }
}
