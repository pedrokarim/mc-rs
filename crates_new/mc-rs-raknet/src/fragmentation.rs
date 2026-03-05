use std::collections::HashMap;
use std::time::{Duration, Instant};

use bytes::{Bytes, BytesMut};

use crate::constants::MAX_SPLIT_COUNT;
use crate::packet::frame::SplitInfo;

/// Reassembles fragmented (split) packets.
pub struct FragmentAssembler {
    pending: HashMap<u16, FragmentBuffer>,
}

struct FragmentBuffer {
    count: u32,
    fragments: HashMap<u32, Bytes>,
    created_at: Instant,
}

impl FragmentAssembler {
    pub fn new() -> Self {
        Self {
            pending: HashMap::new(),
        }
    }

    /// Insert a fragment. Returns the fully reassembled payload if all fragments
    /// have arrived, or `None` if more fragments are needed.
    pub fn insert(&mut self, info: &SplitInfo, body: Bytes) -> Result<Option<Bytes>, String> {
        if info.count > MAX_SPLIT_COUNT {
            return Err(format!(
                "split_count {} exceeds maximum {}",
                info.count, MAX_SPLIT_COUNT
            ));
        }
        if info.index >= info.count {
            return Err(format!(
                "split_index {} >= split_count {}",
                info.index, info.count
            ));
        }

        let buffer = self
            .pending
            .entry(info.id)
            .or_insert_with(|| FragmentBuffer {
                count: info.count,
                fragments: HashMap::new(),
                created_at: Instant::now(),
            });

        buffer.fragments.insert(info.index, body);

        if buffer.fragments.len() as u32 == buffer.count {
            // All fragments received — reassemble
            let buffer = self.pending.remove(&info.id).unwrap();
            let mut result = BytesMut::new();
            for i in 0..buffer.count {
                if let Some(frag) = buffer.fragments.get(&i) {
                    result.extend_from_slice(frag);
                } else {
                    return Err(format!("missing fragment index {i}"));
                }
            }
            Ok(Some(result.freeze()))
        } else {
            Ok(None)
        }
    }

    /// Remove incomplete fragment assemblies older than the given timeout.
    pub fn cleanup(&mut self, timeout: Duration) {
        let now = Instant::now();
        self.pending
            .retain(|_, buf| now.duration_since(buf.created_at) < timeout);
    }
}

impl Default for FragmentAssembler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reassemble_in_order() {
        let mut fa = FragmentAssembler::new();
        let info = |idx| SplitInfo {
            count: 3,
            id: 1,
            index: idx,
        };

        assert!(fa
            .insert(&info(0), Bytes::from_static(b"aaa"))
            .unwrap()
            .is_none());
        assert!(fa
            .insert(&info(1), Bytes::from_static(b"bbb"))
            .unwrap()
            .is_none());
        let result = fa
            .insert(&info(2), Bytes::from_static(b"ccc"))
            .unwrap()
            .unwrap();
        assert_eq!(result, Bytes::from_static(b"aaabbbccc"));
    }

    #[test]
    fn reassemble_out_of_order() {
        let mut fa = FragmentAssembler::new();
        let info = |idx| SplitInfo {
            count: 3,
            id: 2,
            index: idx,
        };

        assert!(fa
            .insert(&info(2), Bytes::from_static(b"ccc"))
            .unwrap()
            .is_none());
        assert!(fa
            .insert(&info(0), Bytes::from_static(b"aaa"))
            .unwrap()
            .is_none());
        let result = fa
            .insert(&info(1), Bytes::from_static(b"bbb"))
            .unwrap()
            .unwrap();
        assert_eq!(result, Bytes::from_static(b"aaabbbccc"));
    }

    #[test]
    fn reject_excessive_split_count() {
        let mut fa = FragmentAssembler::new();
        let info = SplitInfo {
            count: 1000,
            id: 1,
            index: 0,
        };
        assert!(fa.insert(&info, Bytes::from_static(b"x")).is_err());
    }

    #[test]
    fn reject_invalid_index() {
        let mut fa = FragmentAssembler::new();
        let info = SplitInfo {
            count: 3,
            id: 1,
            index: 5,
        };
        assert!(fa.insert(&info, Bytes::from_static(b"x")).is_err());
    }

    #[test]
    fn cleanup_stale() {
        let mut fa = FragmentAssembler::new();
        let info = SplitInfo {
            count: 3,
            id: 1,
            index: 0,
        };
        let _ = fa.insert(&info, Bytes::from_static(b"a"));
        assert_eq!(fa.pending.len(), 1);

        // With zero timeout, everything is stale
        fa.cleanup(Duration::ZERO);
        assert_eq!(fa.pending.len(), 0);
    }
}
