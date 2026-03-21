use std::collections::{BTreeMap, VecDeque};

use crate::consts::*;
use crate::protocol::datagram::{Datagram, EncapsulatedPacket, SplitInfo};

/// Send reliability layer.
/// Handles: sequencing, message indexing, packet splitting, retransmission.
pub struct SendLayer {
    // ── Sequence counters ──
    send_seq_number: u32,
    send_message_index: u32,
    send_order_index: [u32; MAX_ORDER_CHANNELS],
    send_sequenced_index: [u32; MAX_ORDER_CHANNELS],
    split_id: u16,

    // ── MTU ──
    #[allow(dead_code)]
    mtu: u16,
    max_encapsulated_size: usize,

    // ── Current datagram being built ──
    current_datagram: Vec<EncapsulatedPacket>,
    current_datagram_size: usize,

    // ── Queues ──
    send_queue: VecDeque<Datagram>,
    resend_queue: VecDeque<EncapsulatedPacket>,

    // ── Reliable cache (for retransmission on NACK) ──
    /// seq_number → list of reliable packets in that datagram
    reliable_cache: BTreeMap<u32, Vec<EncapsulatedPacket>>,

    // ── Reliable window (used for flow control, TODO)
    #[allow(dead_code)]
    reliable_window_start: u32,
    #[allow(dead_code)]
    reliable_window_end: u32,
    #[allow(dead_code)]
    reliable_acked: BTreeMap<u32, bool>,

    // ── Retransmission timing ──
    /// seq_number → timestamp when sent
    datagram_timestamps: BTreeMap<u32, f64>,
}

impl SendLayer {
    pub fn new(mtu: u16) -> Self {
        let max_payload = mtu as usize - DATAGRAM_MTU_OVERHEAD;
        Self {
            send_seq_number: 0,
            send_message_index: 0,
            send_order_index: [0; MAX_ORDER_CHANNELS],
            send_sequenced_index: [0; MAX_ORDER_CHANNELS],
            split_id: 0,
            mtu,
            max_encapsulated_size: max_payload,
            current_datagram: Vec::new(),
            current_datagram_size: 0,
            send_queue: VecDeque::new(),
            resend_queue: VecDeque::new(),
            reliable_cache: BTreeMap::new(),
            reliable_window_start: 0,
            reliable_window_end: SEND_RELIABLE_WINDOW_SIZE,
            reliable_acked: BTreeMap::new(),
            datagram_timestamps: BTreeMap::new(),
        }
    }

    /// Queue a packet for sending.
    pub fn add_packet(&mut self, mut pkt: EncapsulatedPacket, immediate: bool) {
        // Assign ordering indices
        let ch = pkt.order_channel.unwrap_or(0) as usize;
        if pkt.reliability.is_ordered() && ch < MAX_ORDER_CHANNELS {
            pkt.order_index = Some(self.send_order_index[ch]);
            self.send_order_index[ch] += 1;
        } else if pkt.reliability.is_sequenced() && ch < MAX_ORDER_CHANNELS {
            pkt.order_index = Some(self.send_order_index[ch]);
            pkt.sequence_index = Some(self.send_sequenced_index[ch]);
            self.send_sequenced_index[ch] += 1;
        }

        // Check if packet needs splitting
        let header_size = pkt.header_size();
        if header_size + pkt.body.len() > self.max_encapsulated_size {
            self.split_packet(pkt);
        } else {
            self.queue_encapsulated(pkt);
        }

        if immediate {
            self.flush_current_datagram();
        }
    }

    fn split_packet(&mut self, pkt: EncapsulatedPacket) {
        // Calculate max body per split part
        let split_overhead = 10; // count(4) + id(2) + index(4)
        let base_header = pkt.header_size() + split_overhead;
        let max_body = self.max_encapsulated_size - base_header;
        if max_body == 0 {
            return;
        }

        let split_id = self.split_id;
        self.split_id = self.split_id.wrapping_add(1);

        let body = &pkt.body;
        let count = body.len().div_ceil(max_body) as u32;

        for i in 0..count {
            let start = (i as usize) * max_body;
            let end = ((i as usize + 1) * max_body).min(body.len());
            let part_body = body[start..end].to_vec();

            let part = EncapsulatedPacket {
                reliability: pkt.reliability,
                message_index: None, // assigned in queue_encapsulated
                sequence_index: pkt.sequence_index,
                order_index: pkt.order_index,
                order_channel: pkt.order_channel,
                split: Some(SplitInfo {
                    count,
                    id: split_id,
                    index: i,
                }),
                body: part_body,
            };

            self.queue_encapsulated(part);
        }
    }

    fn queue_encapsulated(&mut self, mut pkt: EncapsulatedPacket) {
        // Assign message index for reliable packets
        if pkt.reliability.is_reliable() {
            pkt.message_index = Some(self.send_message_index);
            self.send_message_index += 1;
        }

        let pkt_size = pkt.total_size();

        // Check if adding to current datagram would exceed MTU
        if !self.current_datagram.is_empty()
            && self.current_datagram_size + pkt_size > self.max_encapsulated_size
        {
            self.flush_current_datagram();
        }

        self.current_datagram_size += pkt_size;
        self.current_datagram.push(pkt);
    }

    /// Flush the current datagram to the send queue.
    pub fn flush_current_datagram(&mut self) {
        if self.current_datagram.is_empty() {
            return;
        }

        let seq = self.send_seq_number;
        self.send_seq_number += 1;

        let packets = std::mem::take(&mut self.current_datagram);
        self.current_datagram_size = 0;

        // Cache reliable packets for retransmission
        let reliable_packets: Vec<EncapsulatedPacket> = packets
            .iter()
            .filter(|p| p.reliability.is_reliable())
            .cloned()
            .collect();
        if !reliable_packets.is_empty() {
            self.reliable_cache.insert(seq, reliable_packets);
        }

        self.send_queue.push_back(Datagram {
            seq_number: seq,
            packets,
        });
    }

    /// Handle ACK: mark sequence numbers as acknowledged.
    pub fn on_ack(&mut self, seq_numbers: &[u32]) {
        for &seq in seq_numbers {
            self.reliable_cache.remove(&seq);
            self.datagram_timestamps.remove(&seq);
        }
    }

    /// Handle NACK: re-queue reliable packets from those datagrams.
    pub fn on_nack(&mut self, seq_numbers: &[u32]) {
        for &seq in seq_numbers {
            if let Some(packets) = self.reliable_cache.remove(&seq) {
                for pkt in packets {
                    self.resend_queue.push_back(pkt);
                }
            }
            self.datagram_timestamps.remove(&seq);
        }
    }

    /// Check for timed-out reliable datagrams (no ACK received within timeout).
    pub fn check_timeouts(&mut self, current_time: f64) {
        let timeout = RETRANSMIT_TIMEOUT_SECS;
        let timed_out: Vec<u32> = self
            .datagram_timestamps
            .iter()
            .filter(|(_, &ts)| current_time - ts >= timeout)
            .map(|(&seq, _)| seq)
            .collect();

        for seq in timed_out {
            if let Some(packets) = self.reliable_cache.remove(&seq) {
                for pkt in packets {
                    self.resend_queue.push_back(pkt);
                }
            }
            self.datagram_timestamps.remove(&seq);
        }
    }

    /// Get the next datagram to send (from resend queue first, then send queue).
    pub fn next_datagram(&mut self, current_time: f64) -> Option<Datagram> {
        // Process resend queue first
        if !self.resend_queue.is_empty() {
            let packets: Vec<EncapsulatedPacket> = self.resend_queue.drain(..).collect();
            let seq = self.send_seq_number;
            self.send_seq_number += 1;

            let reliable_packets: Vec<EncapsulatedPacket> = packets
                .iter()
                .filter(|p| p.reliability.is_reliable())
                .cloned()
                .collect();
            if !reliable_packets.is_empty() {
                self.reliable_cache.insert(seq, reliable_packets);
            }
            self.datagram_timestamps.insert(seq, current_time);

            return Some(Datagram {
                seq_number: seq,
                packets,
            });
        }

        // Then send queue
        if let Some(dg) = self.send_queue.pop_front() {
            self.datagram_timestamps.insert(dg.seq_number, current_time);
            Some(dg)
        } else {
            None
        }
    }

    /// Check if there are pending datagrams to send.
    pub fn has_pending(&self) -> bool {
        !self.send_queue.is_empty()
            || !self.resend_queue.is_empty()
            || !self.current_datagram.is_empty()
    }

    /// Flush everything and return all pending.
    pub fn flush_all(&mut self, current_time: f64) -> Vec<Datagram> {
        self.flush_current_datagram();
        let mut result = Vec::new();
        while let Some(dg) = self.next_datagram(current_time) {
            result.push(dg);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::datagram::Reliability;

    fn make_body(size: usize) -> Vec<u8> {
        vec![0xAA; size]
    }

    #[test]
    fn test_simple_send() {
        let mut layer = SendLayer::new(1400);
        let pkt = EncapsulatedPacket {
            reliability: Reliability::ReliableOrdered,
            message_index: None,
            sequence_index: None,
            order_index: None,
            order_channel: Some(0),
            split: None,
            body: vec![1, 2, 3],
        };
        layer.add_packet(pkt, true);

        let datagrams = layer.flush_all(0.0);
        assert_eq!(datagrams.len(), 1);
        assert_eq!(datagrams[0].seq_number, 0);
        assert_eq!(datagrams[0].packets[0].body, vec![1, 2, 3]);
        assert_eq!(datagrams[0].packets[0].message_index, Some(0));
        assert_eq!(datagrams[0].packets[0].order_index, Some(0));
    }

    #[test]
    fn test_split_large_packet() {
        let mut layer = SendLayer::new(600);
        let pkt = EncapsulatedPacket {
            reliability: Reliability::Reliable,
            message_index: None,
            sequence_index: None,
            order_index: None,
            order_channel: None,
            split: None,
            body: make_body(2000), // larger than MTU
        };
        layer.add_packet(pkt, true);

        let datagrams = layer.flush_all(0.0);
        // Should have been split into multiple datagrams
        let total_body: usize = datagrams
            .iter()
            .flat_map(|d| &d.packets)
            .map(|p| p.body.len())
            .sum();
        assert_eq!(total_body, 2000);

        // All parts should have split info
        for dg in &datagrams {
            for pkt in &dg.packets {
                assert!(pkt.split.is_some());
            }
        }
    }

    #[test]
    fn test_nack_retransmit() {
        let mut layer = SendLayer::new(1400);
        let pkt = EncapsulatedPacket {
            reliability: Reliability::Reliable,
            message_index: None,
            sequence_index: None,
            order_index: None,
            order_channel: None,
            split: None,
            body: vec![42],
        };
        layer.add_packet(pkt, true);

        // Get the datagram
        let dg = layer.next_datagram(0.0).unwrap();
        let seq = dg.seq_number;

        // Simulate NACK
        layer.on_nack(&[seq]);

        // Should have a retransmit
        let retransmit = layer.next_datagram(1.0);
        assert!(retransmit.is_some());
        assert_eq!(retransmit.unwrap().packets[0].body, vec![42]);
    }

    #[test]
    fn test_ack_removes_cache() {
        let mut layer = SendLayer::new(1400);
        let pkt = EncapsulatedPacket {
            reliability: Reliability::Reliable,
            message_index: None,
            sequence_index: None,
            order_index: None,
            order_channel: None,
            split: None,
            body: vec![42],
        };
        layer.add_packet(pkt, true);

        let dg = layer.next_datagram(0.0).unwrap();
        let seq = dg.seq_number;

        // ACK
        layer.on_ack(&[seq]);

        // NACK should have nothing to retransmit
        layer.on_nack(&[seq]);
        assert!(layer.next_datagram(1.0).is_none());
    }
}
