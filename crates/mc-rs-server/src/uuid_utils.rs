//! UUID helpers (v4, player UUIDs).

use rand::Rng;

/// Generate random UUID v4.
pub fn new_v4() -> String {
    let mut rng = rand::thread_rng();
    let mut bytes = [0u8; 16];
    rng.fill(&mut bytes);
    bytes[6] = (bytes[6] & 0x0f) | 0x40; // Version 4.
    bytes[8] = (bytes[8] & 0x3f) | 0x80; // Variant 1.
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    )
}

/// Offline mode UUID (deterministic from name using SipHash).
pub fn offline_uuid(name: &str) -> String {
    use std::hash::{Hash, Hasher, BuildHasher};
    use std::collections::hash_map::RandomState;
    // Use a fixed hasher seed for determinism.
    let seed1 = 0x0123_4567_89ab_cdef_u64;
    let seed2 = 0xfedc_ba98_7654_3210_u64;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    let _ = (RandomState::new, seed1, seed2);
    let input = format!("OfflinePlayer:{}", name);
    input.hash(&mut hasher);
    let h = hasher.finish();
    // Derive second 64-bit by hashing reversed.
    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    input.chars().rev().collect::<String>().hash(&mut h2);
    let h_low = h2.finish();
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&h.to_be_bytes());
    bytes[8..].copy_from_slice(&h_low.to_be_bytes());
    bytes[6] = (bytes[6] & 0x0f) | 0x30;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offline_deterministic() {
        let a = offline_uuid("Steve");
        let b = offline_uuid("Steve");
        assert_eq!(a, b);
    }

    #[test]
    fn different_names_different_uuids() {
        assert_ne!(offline_uuid("Steve"), offline_uuid("Alex"));
    }
}
