//! Entity ID generator.

use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Reset for testing.
pub fn reset_id_counter() {
    NEXT_ID.store(1, Ordering::SeqCst);
}

pub fn next_entity_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// IDs used for vanilla (runtime ID).
/// 0 = server, >0 = entities.
pub const SERVER_ENTITY_ID: u64 = 0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_unique() {
        let a = next_entity_id();
        let b = next_entity_id();
        assert_ne!(a, b);
    }
}
