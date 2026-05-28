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
    /// Received seq numbers (for gap detection + duplicate rejection within window)
    received_seqs: HashSet<u32>,
    /// Highest datagram seq number ever received (-1 = none). Mirrors RakLib
    /// `highestSeqNumber`; drives the forced window advance in `update()`.
    highest_seq_number: i64,

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

    // ── Freeze diagnostics (per-session) ──
    /// Total datagrams accepted (passed the window check) since session start.
    diag_datagrams_in: u64,
    /// Total user-level packets actually emitted to the game layer.
    diag_packets_out: u64,
    /// Datagrams rejected by the window check (out of window / duplicate).
    diag_dropped_window: u64,

    /// Set when an unrecoverable split-packet error occurs. RakLib throws
    /// `PacketHandlingException` here and its doc mandates the owning session
    /// MUST disconnect the peer (a dropped reliable-ordered split otherwise
    /// wedges the ordered channel forever = freeze). The session checks this
    /// and disconnects, so the client reconnects with a fresh layer.
    fatal: Option<&'static str>,
}

/// Snapshot of the receive layer's internal state, for freeze diagnostics.
/// Every field that could wedge the session is here so a single log line at
/// the moment of freeze tells us EXACTLY which mechanism stalled.
#[derive(Debug, Clone)]
pub struct RecvDiag {
    pub datagrams_in: u64,
    pub packets_out: u64,
    pub dropped_window: u64,
    pub window_start: u32,
    pub window_end: u32,
    pub highest_seq: i64,
    pub reliable_window_start: u32,
    pub reliable_window_end: u32,
    pub reliable_held: usize,
    /// Per ordered-channel: (next expected index, queued-out-of-order count).
    /// Only channels with a non-zero index or non-empty queue are listed.
    pub ordered_channels: Vec<(usize, u32, usize)>,
    pub split_in_progress: usize,
    pub nack_pending: usize,
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
            highest_seq_number: -1,
            ack_queue: Vec::new(),
            nack_queue: Vec::new(),
            reliable_window_start: 0,
            reliable_window_end: RECV_WINDOW_SIZE,
            reliable_received: HashSet::new(),
            recv_ordered_index: [0; MAX_ORDER_CHANNELS],
            recv_ordered_queue: std::array::from_fn(|_| BTreeMap::new()),
            recv_sequenced_highest: [0; MAX_ORDER_CHANNELS],
            split_packets: HashMap::new(),
            diag_datagrams_in: 0,
            diag_packets_out: 0,
            diag_dropped_window: 0,
            fatal: None,
        }
    }

    /// Unrecoverable split error, if any. The session must disconnect the peer
    /// when this is `Some` (faithful to RakLib's throw → Session disconnect).
    pub fn fatal_error(&self) -> Option<&'static str> {
        self.fatal
    }

    /// Snapshot the internal state for freeze diagnostics.
    pub fn diag(&self) -> RecvDiag {
        let ordered_channels = self
            .recv_ordered_index
            .iter()
            .enumerate()
            .filter(|(ch, &idx)| idx != 0 || !self.recv_ordered_queue[*ch].is_empty())
            .map(|(ch, &idx)| (ch, idx, self.recv_ordered_queue[ch].len()))
            .collect();
        RecvDiag {
            datagrams_in: self.diag_datagrams_in,
            packets_out: self.diag_packets_out,
            dropped_window: self.diag_dropped_window,
            window_start: self.window_start,
            window_end: self.window_end,
            highest_seq: self.highest_seq_number,
            reliable_window_start: self.reliable_window_start,
            reliable_window_end: self.reliable_window_end,
            reliable_held: self.reliable_received.len(),
            ordered_channels,
            split_in_progress: self.split_packets.len(),
            nack_pending: self.nack_queue.len(),
        }
    }

    /// Process a received datagram. Returns the list of user-level packets ready for delivery.
    pub fn on_datagram(
        &mut self,
        seq_number: u32,
        packets: Vec<EncapsulatedPacket>,
    ) -> Vec<Vec<u8>> {
        // Sequence window check (RakLib ReceiveReliabilityLayer::onDatagram).
        // Upper bound is INCLUSIVE (`> window_end`) as in RakLib. A seq already
        // in `received_seqs` is a duplicate (RakLib `isset(ACKQueue[seq])`):
        // RakNet never reuses a datagram seq number, so this only fires on
        // network-duplicated UDP delivery.
        if seq_number < self.window_start
            || seq_number > self.window_end
            || self.received_seqs.contains(&seq_number)
        {
            self.diag_dropped_window += 1;
            return Vec::new(); // out of window or duplicate, drop
        }
        self.diag_datagrams_in += 1;

        // This datagram filled a gap we may have NACKed earlier — cancel that
        // pending NACK (RakLib `unset($this->NACKQueue[$packet->seqNumber])`).
        if let Some(pos) = self.nack_queue.iter().position(|&s| s == seq_number) {
            self.nack_queue.swap_remove(pos);
        }

        self.received_seqs.insert(seq_number);
        self.ack_queue.push(seq_number);
        if (seq_number as i64) > self.highest_seq_number {
            self.highest_seq_number = seq_number as i64;
        }

        if seq_number == self.window_start {
            // Contiguous — shift the window forward as far as the received set
            // allows (fast path; the hard guarantee is `update()` below).
            while self.received_seqs.contains(&self.window_start) {
                self.window_start += 1;
                self.window_end += 1;
            }
        } else {
            // Gap: a later datagram arrived before earlier ones. NACK the
            // missing seqs (deduplicated — RakLib keys NACKQueue by seq).
            for missing in self.window_start..seq_number {
                if !self.received_seqs.contains(&missing) && !self.nack_queue.contains(&missing) {
                    self.nack_queue.push(missing);
                }
            }
        }

        // Process encapsulated packets
        let mut output = Vec::new();
        for pkt in packets {
            self.handle_encapsulated(pkt, &mut output);
        }
        self.diag_packets_out += output.len() as u64;
        output
    }

    fn handle_encapsulated(&mut self, pkt: EncapsulatedPacket, output: &mut Vec<Vec<u8>>) {
        // Reliable window dedup + slide — done for EVERY incoming encapsulated
        // packet that carries a message_index, INCLUDING each split part,
        // BEFORE split reassembly. This is the exact order of RakLib
        // `ReceiveReliabilityLayer::handleEncapsulatedPacket` (messageIndex
        // block, then `handleSplit`).
        //
        // ⚠️ Doing split reassembly first (as before) silently dropped the
        // message_index of every split part except the one that triggered
        // completion → `reliable_window_start` wedged at the first skipped
        // index (e.g. 4 = the split Login packet) → after 2048 message
        // indices every reliable packet was rejected (`msg_idx >=
        // reliable_window_end`) → ordered channel head-of-line blocked →
        // per-session freeze. Reconnect = fresh window = temporary fix.
        if let Some(msg_idx) = pkt.message_index {
            // Upper bound INCLUSIVE (`>`) as in RakLib.
            if msg_idx < self.reliable_window_start
                || msg_idx > self.reliable_window_end
                || self.reliable_received.contains(&msg_idx)
            {
                return; // duplicate or out of window
            }
            self.reliable_received.insert(msg_idx);
            // Slide reliable window over the contiguous run.
            while self.reliable_received.contains(&self.reliable_window_start) {
                self.reliable_received.remove(&self.reliable_window_start);
                self.reliable_window_start += 1;
                self.reliable_window_end += 1;
            }
        }

        // Split reassembly — AFTER the reliable window (RakLib `handleSplit`).
        let pkt = if pkt.split.is_some() {
            match self.handle_split(pkt) {
                Some(reassembled) => reassembled,
                None => return, // not yet complete
            }
        } else {
            pkt
        };

        // Handle ordering / sequencing
        if pkt.reliability.is_sequenced() {
            let ch = pkt.order_channel.unwrap_or(0) as usize;
            if ch >= MAX_ORDER_CHANNELS {
                return;
            }
            let seq_idx = pkt.sequence_index.unwrap_or(0);
            let ord_idx = pkt.order_index.unwrap_or(0);
            if seq_idx < self.recv_sequenced_highest[ch] || ord_idx < self.recv_ordered_index[ch] {
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
                while let Some(entry) =
                    self.recv_ordered_queue[ch].remove(&self.recv_ordered_index[ch])
                {
                    self.recv_ordered_index[ch] += 1;
                    self.recv_sequenced_highest[ch] = 0;
                    output.push(entry.body);
                }
            } else if ord_idx > self.recv_ordered_index[ch] {
                // Future packet — queue it, but bound the queue (RakLib
                // handleEncapsulatedPacket: `if(count(...) >= WINDOW_SIZE)
                // return;`). Without this, a never-filled ordered gap grows
                // the queue unbounded.
                if self.recv_ordered_queue[ch].len() >= RECV_WINDOW_SIZE as usize {
                    return; // ordered queue overflow for this channel
                }
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
        // RakLib handleSplit throws PacketHandlingException on each of these;
        // its doc mandates the session MUST disconnect the peer (an
        // unrecoverable reliable-ordered split otherwise wedges the ordered
        // channel forever = freeze). We flag `fatal`; the session disconnects.
        if split.count > MAX_SPLIT_PART_COUNT || split.count == 0 {
            tracing::warn!(
                "split FATAL: invalid part count {} (max {}) — disconnecting peer",
                split.count,
                MAX_SPLIT_PART_COUNT
            );
            self.fatal = Some("invalid split packet part count");
            return None;
        }
        if split.index >= split.count {
            tracing::warn!(
                "split FATAL: invalid part index {} (count {}) — disconnecting peer",
                split.index,
                split.count
            );
            self.fatal = Some("invalid split packet part index");
            return None;
        }
        if self.split_packets.len() >= MAX_CONCURRENT_SPLITS
            && !self.split_packets.contains_key(&split.id)
        {
            tracing::warn!(
                "split FATAL: {} concurrent splits >= MAX_CONCURRENT_SPLITS={} — disconnecting peer",
                self.split_packets.len(), MAX_CONCURRENT_SPLITS
            );
            self.fatal = Some("exceeded concurrent split packet limit");
            return None;
        }
        // Inconsistent header: a later part claims a different total count
        // (RakLib SPLIT_PACKET_INCONSISTENT_HEADER).
        if let Some(existing) = self.split_packets.get(&split.id) {
            if existing.count != split.count {
                tracing::warn!(
                    "split FATAL: inconsistent count for split id {} ({} vs {}) — disconnecting peer",
                    split.id, split.count, existing.count
                );
                self.fatal = Some("inconsistent split packet header");
                return None;
            }
        }

        let assembly = self
            .split_packets
            .entry(split.id)
            .or_insert_with(|| SplitAssembly {
                count: split.count,
                parts: HashMap::new(),
                reliability: pkt.reliability,
                message_index: pkt.message_index,
                order_index: pkt.order_index,
                order_channel: pkt.order_channel,
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

    /// Per-tick maintenance. **THIS IS THE FIX for the per-session freeze.**
    ///
    /// Faithful port of RakLib `ReceiveReliabilityLayer::update()`: the receive
    /// window is FORCE-advanced past the highest seq number ever received,
    /// regardless of gaps. Datagram seq numbers we NACKed but never recovered
    /// are abandoned at the *datagram-window* level — RakNet never re-sends a
    /// datagram with the same seq number, it resends the lost *message* in a
    /// NEW datagram (higher seq, deduplicated by the reliable `messageIndex`
    /// window). Without this, a single permanently-lost datagram wedges
    /// `window_start` forever; once the client's seq numbers climb past
    /// `window_start + RECV_WINDOW_SIZE`, every incoming datagram is dropped
    /// (`seq > window_end`) → the session stops processing all client packets
    /// while the server keeps ticking. Reconnect resets the layer = temp fix.
    ///
    /// Must be called every session tick (see `session.rs::tick`).
    pub fn update(&mut self) {
        let diff = self.highest_seq_number - self.window_start as i64 + 1;
        if diff > 0 {
            // Count seqs being abandoned (NACKed but never recovered) so a
            // future regression is immediately visible per-session.
            let abandoned = (self.window_start..self.window_start + diff as u32)
                .filter(|s| !self.received_seqs.contains(s))
                .count();
            self.window_start += diff as u32;
            self.window_end += diff as u32;
            if abandoned > 0 {
                tracing::debug!(
                    "recv window force-advanced by {} ({} unrecovered datagram(s) abandoned, window now {}..{})",
                    diff, abandoned, self.window_start, self.window_end
                );
            }
            // Bound memory: drop received-seq entries now behind the window.
            self.received_seqs.retain(|&s| s >= self.window_start);
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

    /// Regression test for the per-session freeze bug.
    ///
    /// A single datagram (seq 5) is lost forever. The client keeps sending
    /// (RakNet never reuses a seq number, so seq 5 never reappears). Without
    /// `update()` force-advancing the window, `window_start` would wedge at 5
    /// and every datagram with seq >= 5 + RECV_WINDOW_SIZE would be dropped,
    /// freezing the session. With the fix, the window advances each tick and
    /// later datagrams keep being delivered.
    #[test]
    fn test_lost_datagram_does_not_wedge_window() {
        let mut layer = ReceiveLayer::new();

        // Deliver seq 0..5 normally (msg/order 0..5).
        for i in 0..5u32 {
            let pkt = make_reliable_ordered(vec![i as u8], i, i);
            assert_eq!(layer.on_datagram(i, vec![pkt]).len(), 1);
        }
        // Window has slid to 5; seq 5 is now permanently lost (never arrives).
        assert_eq!(layer.window_start, 5);

        // Client keeps sending. RakNet assigns ever-increasing seq numbers and
        // resends the lost reliable message in a NEW datagram (seq 6, but the
        // same order_index 5 so ordering is preserved).
        let resend = make_reliable_ordered(vec![5], 5, 5);
        assert_eq!(layer.on_datagram(6, vec![resend]).len(), 1);

        // Tick: window must force-advance past the lost seq 5.
        layer.update();
        assert!(
            layer.window_start > 5,
            "window wedged at {} — freeze bug present",
            layer.window_start
        );

        // Simulate a long mining burst: far more than RECV_WINDOW_SIZE
        // datagrams. Every one must still be delivered (no silent drop).
        for i in 0..(RECV_WINDOW_SIZE * 3) {
            let next_seq = 7 + i;
            let next_ord = 6 + i;
            let pkt = make_reliable_ordered(vec![0xAB], next_ord, next_ord);
            let out = layer.on_datagram(next_seq, vec![pkt]);
            assert_eq!(
                out.len(),
                1,
                "datagram seq {} dropped — receive window wedged",
                next_seq
            );
            layer.update();
        }
    }

    /// Regression test for the per-session freeze: a split reliable packet
    /// must register the message_index of EVERY part in the reliable window,
    /// not just the part that completes reassembly. Otherwise the reliable
    /// window wedges on the skipped indices (the real freeze observed in
    /// production: `reliable_win` stuck at 4 = the split Login packet).
    #[test]
    fn test_split_parts_register_all_reliable_indices() {
        use crate::protocol::datagram::SplitInfo;

        let mut layer = ReceiveLayer::new();

        // Deliver 4 plain reliable-ordered messages (msg/order 0..3).
        for i in 0..4u32 {
            let pkt = make_reliable_ordered(vec![i as u8], i, i);
            assert_eq!(layer.on_datagram(i, vec![pkt]).len(), 1);
        }
        assert_eq!(layer.reliable_window_start, 4);

        // A 2-part split reliable-ordered packet. Each part has its own
        // message_index (4 and 5); both share order_index 4.
        let part0 = EncapsulatedPacket {
            reliability: Reliability::ReliableOrdered,
            message_index: Some(4),
            sequence_index: None,
            order_index: Some(4),
            order_channel: Some(0),
            split: Some(SplitInfo {
                count: 2,
                id: 1,
                index: 0,
            }),
            body: vec![0xAA],
        };
        let part1 = EncapsulatedPacket {
            reliability: Reliability::ReliableOrdered,
            message_index: Some(5),
            sequence_index: None,
            order_index: Some(4),
            order_channel: Some(0),
            split: Some(SplitInfo {
                count: 2,
                id: 1,
                index: 1,
            }),
            body: vec![0xBB],
        };
        assert_eq!(layer.on_datagram(4, vec![part0]).len(), 0); // incomplete
        assert_eq!(layer.on_datagram(5, vec![part1]).len(), 1); // reassembled

        // BOTH split parts' message indices (4 and 5) must have been
        // registered → window slid past them.
        assert_eq!(
            layer.reliable_window_start, 6,
            "split parts' message indices not all registered — reliable window wedged"
        );

        // A subsequent reliable-ordered message must still be delivered.
        let next = make_reliable_ordered(vec![0xCC], 6, 5);
        assert_eq!(layer.on_datagram(6, vec![next]).len(), 1);
    }

    /// An unrecoverable split error must flag `fatal` so the session
    /// disconnects the peer (RakLib throws PacketHandlingException here).
    /// Silently dropping would wedge the ordered channel forever = freeze.
    #[test]
    fn test_bad_split_flags_fatal() {
        use crate::protocol::datagram::SplitInfo;

        let mut layer = ReceiveLayer::new();
        assert!(layer.fatal_error().is_none());

        let bad = EncapsulatedPacket {
            reliability: Reliability::ReliableOrdered,
            message_index: Some(0),
            sequence_index: None,
            order_index: Some(0),
            order_channel: Some(0),
            split: Some(SplitInfo {
                count: MAX_SPLIT_PART_COUNT + 1, // invalid
                id: 1,
                index: 0,
            }),
            body: vec![1],
        };
        layer.on_datagram(0, vec![bad]);
        assert!(
            layer.fatal_error().is_some(),
            "bad split must flag fatal so the session disconnects the peer"
        );
    }

    /// The ordered queue must not grow unbounded on a never-filled gap
    /// (RakLib caps at WINDOW_SIZE).
    #[test]
    fn test_ordered_queue_is_bounded() {
        let mut layer = ReceiveLayer::new();
        // order_index 0 never arrives; flood future ordered packets.
        for i in 1..(RECV_WINDOW_SIZE + 500) {
            let pkt = make_reliable_ordered(vec![0], i, i);
            layer.on_datagram(i, vec![pkt]);
        }
        assert!(
            layer.recv_ordered_queue[0].len() <= RECV_WINDOW_SIZE as usize,
            "ordered queue grew past RECV_WINDOW_SIZE ({} entries)",
            layer.recv_ordered_queue[0].len()
        );
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
