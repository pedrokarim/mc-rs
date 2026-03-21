use std::collections::{BTreeMap, HashMap, HashSet};

use crate::consts::*;
use crate::protocol::datagram::{EncapsulatedPacket, Reliability};

/// Receive reliability layer.
/// Handles: ACK/NACK generation, ordering, sequencing, split reassembly.
pub struct ReceiveLayer {
    // ── Sequence window ──
    /// Expected next datagram seq number
    window_start: u32,
    window_end: u32,
    /// Received seq numbers (for gap detection)
    received_seqs: HashSet<u32>,

    // ── ACK/NACK queues ──
    pub ack_queue: Vec<u32>,
    pub nack_queue: Vec<u32>,

    // ── Reliable message tracking ──
    reliable_window_start: u32,
    reliable_window_end: u32,
    reliable_received: HashSet<u32>,

    // ── Ordering ──
    /// Next expected order index per channel
    recv_ordered_index: [u32; MAX_ORDER_CHANNELS],
    /// Queued out-of-order packets per channel
    recv_ordered_queue: [BTreeMap<u32, EncapsulatedPacket>; MAX_ORDER_CHANNELS],
    /// Highest received sequence index per channel (for sequenced mode)
    recv_sequenced_highest: [u32; MAX_ORDER_CHANNELS],

    // ── Split reassembly ──
    split_packets: HashMap<u16, SplitAssembly>,
}

struct SplitAssembly {
    count: u32,
    parts: HashMap<u32, Vec<u8>>,
    reliability: Reliability,
    message_index: Option<u32>,
    order_index: Option<u32>,
    order_channel: Option<u8>,
}

impl Default for ReceiveLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl ReceiveLayer {
    pub fn new() -> Self {
        Self {
            window_start: 0,
            window_end: RECV_WINDOW_SIZE,
            received_seqs: HashSet::new(),
            ack_queue: Vec::new(),
            nack_queue: Vec::new(),
            reliable_window_start: 0,
            reliable_window_end: RECV_WINDOW_SIZE,
            reliable_received: HashSet::new(),
            recv_ordered_index: [0; MAX_ORDER_CHANNELS],
            recv_ordered_queue: std::array::from_fn(|_| BTreeMap::new()),
            recv_sequenced_highest: [0; MAX_ORDER_CHANNELS],
            split_packets: HashMap::new(),
        }
    }

    /// Process a received datagram. Returns the list of user-level packets ready for delivery.
    pub fn on_datagram(
        &mut self,
        seq_number: u32,
        packets: Vec<EncapsulatedPacket>,
    ) -> Vec<Vec<u8>> {
        // Sequence window check
        if seq_number < self.window_start || seq_number >= self.window_end {
            return Vec::new(); // out of window, drop
        }

        // Detect gaps for NACK
        if seq_number > self.window_start {
            for missing in self.window_start..seq_number {
                if !self.received_seqs.contains(&missing) {
                    self.nack_queue.push(missing);
                }
            }
        }

        self.received_seqs.insert(seq_number);
        self.ack_queue.push(seq_number);

        // Slide window
        while self.received_seqs.contains(&self.window_start) {
            self.received_seqs.remove(&self.window_start);
            self.window_start += 1;
            self.window_end += 1;
        }

        // Process encapsulated packets
        let mut output = Vec::new();
        for pkt in packets {
            self.handle_encapsulated(pkt, &mut output);
        }
        output
    }

    fn handle_encapsulated(&mut self, pkt: EncapsulatedPacket, output: &mut Vec<Vec<u8>>) {
        // Handle split packets
        let pkt = if pkt.split.is_some() {
            match self.handle_split(pkt) {
                Some(reassembled) => reassembled,
                None => return, // not yet complete
            }
        } else {
            pkt
        };

        // Handle reliability
        if pkt.reliability.is_reliable() {
            if let Some(msg_idx) = pkt.message_index {
                if msg_idx < self.reliable_window_start
                    || msg_idx >= self.reliable_window_end
                    || self.reliable_received.contains(&msg_idx)
                {
                    return; // duplicate or out of window
                }
                self.reliable_received.insert(msg_idx);
                // Slide reliable window
                while self.reliable_received.contains(&self.reliable_window_start) {
                    self.reliable_received.remove(&self.reliable_window_start);
                    self.reliable_window_start += 1;
                    self.reliable_window_end += 1;
                }
            }
        }

        // Handle ordering / sequencing
        if pkt.reliability.is_sequenced() {
            let ch = pkt.order_channel.unwrap_or(0) as usize;
            if ch >= MAX_ORDER_CHANNELS {
                return;
            }
            let seq_idx = pkt.sequence_index.unwrap_or(0);
            let ord_idx = pkt.order_index.unwrap_or(0);
            if seq_idx < self.recv_sequenced_highest[ch]
                || ord_idx < self.recv_ordered_index[ch]
            {
                return; // old packet, discard
            }
            self.recv_sequenced_highest[ch] = seq_idx + 1;
            output.push(pkt.body);
        } else if pkt.reliability.is_ordered() {
            let ch = pkt.order_channel.unwrap_or(0) as usize;
            if ch >= MAX_ORDER_CHANNELS {
                return;
            }
            let ord_idx = pkt.order_index.unwrap_or(0);
            if ord_idx == self.recv_ordered_index[ch] {
                // Expected order — deliver immediately
                self.recv_ordered_index[ch] += 1;
                self.recv_sequenced_highest[ch] = 0; // reset sequenced
                output.push(pkt.body);

                // Deliver any queued packets that are now in order
                while let Some(entry) = self.recv_ordered_queue[ch]
                    .remove(&self.recv_ordered_index[ch])
                {
                    self.recv_ordered_index[ch] += 1;
                    self.recv_sequenced_highest[ch] = 0;
                    output.push(entry.body);
                }
            } else if ord_idx > self.recv_ordered_index[ch] {
                // Future packet — queue it
                self.recv_ordered_queue[ch].insert(ord_idx, pkt);
            }
            // else: old packet, discard
        } else {
            // Unreliable, no ordering — deliver immediately
            output.push(pkt.body);
        }
    }

    fn handle_split(&mut self, pkt: EncapsulatedPacket) -> Option<EncapsulatedPacket> {
        let split = pkt.split.as_ref()?;
        if split.count > MAX_SPLIT_PART_COUNT || split.index >= split.count {
            return None;
        }
        if self.split_packets.len() >= MAX_CONCURRENT_SPLITS
            && !self.split_packets.contains_key(&split.id)
        {
            return None; // too many concurrent splits
        }

        let assembly = self.split_packets.entry(split.id).or_insert_with(|| {
            SplitAssembly {
                count: split.count,
                parts: HashMap::new(),
                reliability: pkt.reliability,
                message_index: pkt.message_index,
                order_index: pkt.order_index,
                order_channel: pkt.order_channel,
            }
        });

        assembly.parts.insert(split.index, pkt.body);

        if assembly.parts.len() as u32 == assembly.count {
            // Reassemble
            let mut body = Vec::new();
            for i in 0..assembly.count {
                if let Some(part) = assembly.parts.get(&i) {
                    body.extend_from_slice(part);
                } else {
                    return None; // missing part (shouldn't happen)
                }
            }

            let reassembled = EncapsulatedPacket {
                reliability: assembly.reliability,
                message_index: assembly.message_index,
                sequence_index: None,
                order_index: assembly.order_index,
                order_channel: assembly.order_channel,
                split: None,
                body,
            };

            self.split_packets.remove(&split.id);
            Some(reassembled)
        } else {
            None
        }
    }

    /// Check if there's anything to flush (ACKs/NACKs).
    pub fn has_pending(&self) -> bool {
        !self.ack_queue.is_empty() || !self.nack_queue.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_reliable_ordered(body: Vec<u8>, msg_idx: u32, ord_idx: u32) -> EncapsulatedPacket {
        EncapsulatedPacket {
            reliability: Reliability::ReliableOrdered,
            message_index: Some(msg_idx),
            sequence_index: None,
            order_index: Some(ord_idx),
            order_channel: Some(0),
            split: None,
            body,
        }
    }

    #[test]
    fn test_simple_delivery() {
        let mut layer = ReceiveLayer::new();
        let pkt = make_reliable_ordered(vec![1, 2, 3], 0, 0);
        let result = layer.on_datagram(0, vec![pkt]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], vec![1, 2, 3]);
    }

    #[test]
    fn test_ordered_delivery() {
        let mut layer = ReceiveLayer::new();

        // Send packet with order_index=1 first (out of order)
        let pkt1 = make_reliable_ordered(vec![2], 1, 1);
        let result = layer.on_datagram(1, vec![pkt1]);
        assert_eq!(result.len(), 0); // queued, not delivered

        // Now send packet with order_index=0
        let pkt0 = make_reliable_ordered(vec![1], 0, 0);
        let result = layer.on_datagram(0, vec![pkt0]);
        assert_eq!(result.len(), 2); // both delivered in order
        assert_eq!(result[0], vec![1]);
        assert_eq!(result[1], vec![2]);
    }

    #[test]
    fn test_ack_generation() {
        let mut layer = ReceiveLayer::new();
        let pkt = make_reliable_ordered(vec![1], 0, 0);
        layer.on_datagram(0, vec![pkt]);
        assert_eq!(layer.ack_queue, vec![0]);
    }

    #[test]
    fn test_nack_on_gap() {
        let mut layer = ReceiveLayer::new();
        // Skip seq 0, send seq 2
        let pkt = make_reliable_ordered(vec![1], 0, 0);
        layer.on_datagram(2, vec![pkt]);
        // Should NACK seq 0 and 1
        assert!(layer.nack_queue.contains(&0));
        assert!(layer.nack_queue.contains(&1));
    }

    #[test]
    fn test_duplicate_reliable_rejected() {
        let mut layer = ReceiveLayer::new();
        let pkt = make_reliable_ordered(vec![1], 0, 0);
        let result1 = layer.on_datagram(0, vec![pkt.clone()]);
        assert_eq!(result1.len(), 1);

        // Same message_index again — should be dropped
        let result2 = layer.on_datagram(1, vec![pkt]);
        assert_eq!(result2.len(), 0);
    }

    #[test]
    fn test_split_reassembly() {
        use crate::protocol::datagram::SplitInfo;

        let mut layer = ReceiveLayer::new();

        // Send 3 parts of a split packet
        for i in 0..3 {
            let pkt = EncapsulatedPacket {
                reliability: Reliability::Reliable,
                message_index: Some(i),
                sequence_index: None,
                order_index: None,
                order_channel: None,
                split: Some(SplitInfo {
                    count: 3,
                    id: 1,
                    index: i,
                }),
                body: vec![i as u8 + 1],
            };
            let result = layer.on_datagram(i, vec![pkt]);
            if i < 2 {
                assert_eq!(result.len(), 0);
            } else {
                assert_eq!(result.len(), 1);
                assert_eq!(result[0], vec![1, 2, 3]); // reassembled
            }
        }
    }
}
