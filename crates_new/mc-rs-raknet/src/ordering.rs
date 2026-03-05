use std::collections::BTreeMap;

use bytes::Bytes;

use crate::constants::{MAX_ORDER_CHANNEL_BUFFER, NUM_ORDER_CHANNELS};

/// Manages ordering for all 32 RakNet ordering channels.
pub struct OrderingChannels {
    channels: [OrderChannel; NUM_ORDER_CHANNELS],
}

#[derive(Default)]
struct OrderChannel {
    /// The next expected ordered frame index.
    expected_index: u32,
    /// Out-of-order frames waiting to be delivered.
    buffer: BTreeMap<u32, Bytes>,
    /// Highest sequenced frame index seen (for sequenced mode).
    highest_sequenced_index: u32,
}

impl OrderingChannels {
    pub fn new() -> Self {
        Self {
            channels: std::array::from_fn(|_| OrderChannel::default()),
        }
    }

    /// Insert an ordered frame. Returns zero or more payloads in correct order.
    ///
    /// If the frame is the next expected one, it (and any consecutive buffered
    /// frames) are returned. Otherwise the frame is buffered.
    pub fn insert_ordered(&mut self, channel: u8, ordered_index: u32, body: Bytes) -> Vec<Bytes> {
        let ch = &mut self.channels[channel as usize];

        // Already delivered or duplicate
        if ordered_index < ch.expected_index {
            return Vec::new();
        }

        if ordered_index == ch.expected_index {
            // This is the next expected frame — yield it and any consecutive buffered frames
            let mut result = vec![body];
            ch.expected_index += 1;
            while let Some(next_body) = ch.buffer.remove(&ch.expected_index) {
                result.push(next_body);
                ch.expected_index += 1;
            }
            result
        } else {
            // Out of order — buffer it (with size limit)
            if ch.buffer.len() < MAX_ORDER_CHANNEL_BUFFER {
                ch.buffer.insert(ordered_index, body);
            }
            Vec::new()
        }
    }

    /// Insert a sequenced frame. Returns the body only if it's newer than any
    /// previously seen sequenced frame on this channel.
    pub fn insert_sequenced(
        &mut self,
        channel: u8,
        sequenced_index: u32,
        body: Bytes,
    ) -> Option<Bytes> {
        let ch = &mut self.channels[channel as usize];
        if sequenced_index > ch.highest_sequenced_index || ch.highest_sequenced_index == 0 {
            ch.highest_sequenced_index = sequenced_index;
            Some(body)
        } else {
            None // Old sequenced frame, drop it
        }
    }
}

impl Default for OrderingChannels {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_order_delivery() {
        let mut oc = OrderingChannels::new();
        let r = oc.insert_ordered(0, 0, Bytes::from_static(b"a"));
        assert_eq!(r.len(), 1);
        let r = oc.insert_ordered(0, 1, Bytes::from_static(b"b"));
        assert_eq!(r.len(), 1);
        let r = oc.insert_ordered(0, 2, Bytes::from_static(b"c"));
        assert_eq!(r.len(), 1);
    }

    #[test]
    fn out_of_order_delivery() {
        let mut oc = OrderingChannels::new();

        // Frame 2 arrives first — buffered
        let r = oc.insert_ordered(0, 2, Bytes::from_static(b"c"));
        assert!(r.is_empty());

        // Frame 0 arrives — delivered
        let r = oc.insert_ordered(0, 0, Bytes::from_static(b"a"));
        assert_eq!(r.len(), 1);
        assert_eq!(r[0], Bytes::from_static(b"a"));

        // Frame 1 arrives — delivers 1 and buffered 2
        let r = oc.insert_ordered(0, 1, Bytes::from_static(b"b"));
        assert_eq!(r.len(), 2);
        assert_eq!(r[0], Bytes::from_static(b"b"));
        assert_eq!(r[1], Bytes::from_static(b"c"));
    }

    #[test]
    fn duplicate_ignored() {
        let mut oc = OrderingChannels::new();
        let _ = oc.insert_ordered(0, 0, Bytes::from_static(b"a"));
        let r = oc.insert_ordered(0, 0, Bytes::from_static(b"a"));
        assert!(r.is_empty());
    }

    #[test]
    fn sequenced_newer_wins() {
        let mut oc = OrderingChannels::new();
        assert!(oc
            .insert_sequenced(0, 1, Bytes::from_static(b"a"))
            .is_some());
        assert!(oc
            .insert_sequenced(0, 3, Bytes::from_static(b"c"))
            .is_some());
        // Old sequenced frame is dropped
        assert!(oc
            .insert_sequenced(0, 2, Bytes::from_static(b"b"))
            .is_none());
    }

    #[test]
    fn independent_channels() {
        let mut oc = OrderingChannels::new();
        let r0 = oc.insert_ordered(0, 0, Bytes::from_static(b"ch0"));
        let r1 = oc.insert_ordered(1, 0, Bytes::from_static(b"ch1"));
        assert_eq!(r0.len(), 1);
        assert_eq!(r1.len(), 1);
    }
}
