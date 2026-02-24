use std::collections::{HashMap, HashSet, VecDeque};
use std::net::SocketAddr;
use std::time::Instant;

use bytes::{BufMut, Bytes, BytesMut};

use crate::constants::*;
use crate::fragmentation::FragmentAssembler;
use crate::ordering::OrderingChannels;
use crate::packet::frame::{AckNack, Frame, FrameSet, Reliability, SplitInfo};
use crate::reliability::expand_ack_records;

/// State of a RakNet session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Received OpenConnectionRequest1, sent Reply1.
    Connecting,
    /// Received OpenConnectionRequest2, sent Reply2.
    HandshakeCompleted,
    /// Received ConnectionRequest, sent ConnectionRequestAccepted.
    ConnectionPending,
    /// Received NewIncomingConnection — fully connected.
    Connected,
    /// Session is being removed.
    Disconnected,
}

/// Metadata for a sent FrameSet, used for retransmission.
struct SentFrameSet {
    frames: Vec<Frame>,
    sent_time: Instant,
}

/// Per-connection RakNet session state.
pub struct RakNetSession {
    pub addr: SocketAddr,
    pub state: SessionState,
    pub mtu: u16,
    pub client_guid: i64,
    pub last_activity: Instant,
    pub last_ping_sent: Instant,

    // --- Sending ---
    send_sequence_number: u32,
    send_reliable_index: u32,
    send_ordered_index: [u32; NUM_ORDER_CHANNELS],
    split_id_counter: u16,
    send_queue: VecDeque<Frame>,
    sent_framesets: HashMap<u32, SentFrameSet>,

    // --- Receiving ---
    recv_highest_sequence: Option<u32>,
    received_sequences: HashSet<u32>,
    received_reliable_set: HashSet<u32>,

    // --- ACK/NACK ---
    pub ack_queue: Vec<u32>,
    nack_queue: Vec<u32>,

    // --- Ordering & Fragmentation ---
    ordering: OrderingChannels,
    fragment_assembler: FragmentAssembler,
}

impl RakNetSession {
    pub fn new(addr: SocketAddr, mtu: u16, client_guid: i64) -> Self {
        let now = Instant::now();
        Self {
            addr,
            state: SessionState::Connecting,
            mtu,
            client_guid,
            last_activity: now,
            last_ping_sent: now,
            send_sequence_number: 0,
            send_reliable_index: 0,
            send_ordered_index: [0; NUM_ORDER_CHANNELS],
            split_id_counter: 0,
            send_queue: VecDeque::new(),
            sent_framesets: HashMap::new(),
            recv_highest_sequence: None,
            received_sequences: HashSet::new(),
            received_reliable_set: HashSet::new(),
            ack_queue: Vec::new(),
            nack_queue: Vec::new(),
            ordering: OrderingChannels::new(),
            fragment_assembler: FragmentAssembler::new(),
        }
    }

    /// Queue a payload for sending with the given reliability and order channel.
    pub fn queue_frame(&mut self, body: Bytes, reliability: Reliability, channel: u8) {
        let max_body = self.mtu as usize - MAX_FRAME_OVERHEAD - 4; // 4 for frameset header

        if body.len() > max_body {
            // Fragment the payload
            let split_id = self.split_id_counter;
            self.split_id_counter = self.split_id_counter.wrapping_add(1);
            let split_body_max = max_body - 10; // 10 bytes for split header
            let split_count = body.len().div_ceil(split_body_max) as u32;

            for i in 0..split_count {
                let start = i as usize * split_body_max;
                let end = ((i as usize + 1) * split_body_max).min(body.len());
                let fragment = body.slice(start..end);

                let frame = self.build_frame(
                    fragment,
                    reliability,
                    channel,
                    Some(SplitInfo {
                        count: split_count,
                        id: split_id,
                        index: i,
                    }),
                );
                self.send_queue.push_back(frame);
            }
        } else {
            let frame = self.build_frame(body, reliability, channel, None);
            self.send_queue.push_back(frame);
        }
    }

    fn build_frame(
        &mut self,
        body: Bytes,
        reliability: Reliability,
        channel: u8,
        split: Option<SplitInfo>,
    ) -> Frame {
        let reliable_index = if reliability.is_reliable() {
            let idx = self.send_reliable_index;
            self.send_reliable_index += 1;
            Some(idx)
        } else {
            None
        };

        let (ordered_index, order_channel) =
            if reliability.is_ordered() || reliability.is_sequenced() {
                let ch = channel as usize;
                let idx = self.send_ordered_index[ch];
                self.send_ordered_index[ch] += 1;
                (Some(idx), Some(channel))
            } else {
                (None, None)
            };

        Frame {
            reliability,
            reliable_index,
            sequenced_index: None, // Simplified: not using sequenced mode actively
            ordered_index,
            order_channel,
            split,
            body,
        }
    }

    /// Flush the send queue into FrameSet datagrams respecting MTU limits.
    /// Returns encoded datagrams ready to send on the socket.
    pub fn flush_send_queue(&mut self) -> Vec<Bytes> {
        let mut datagrams = Vec::new();

        while !self.send_queue.is_empty() {
            let mut frameset_buf = BytesMut::with_capacity(self.mtu as usize);
            let seq = self.send_sequence_number;
            self.send_sequence_number += 1;

            // Reserve space for frameset header (4 bytes: 1 id + 3 seq)
            frameset_buf.put_u8(0x84);
            crate::codec::write_u24_le(&mut frameset_buf, seq);

            let mut frames_in_set = Vec::new();

            while let Some(frame) = self.send_queue.front() {
                let frame_size = frame.encoded_size();
                if frameset_buf.len() + frame_size > self.mtu as usize {
                    break;
                }
                let frame = self.send_queue.pop_front().unwrap();
                frame.encode(&mut frameset_buf);
                frames_in_set.push(frame);
            }

            if !frames_in_set.is_empty() {
                self.sent_framesets.insert(
                    seq,
                    SentFrameSet {
                        frames: frames_in_set,
                        sent_time: Instant::now(),
                    },
                );
                datagrams.push(frameset_buf.freeze());
            }
        }

        datagrams
    }

    /// Process a received ACK — remove acknowledged framesets from retransmit tracking.
    pub fn handle_ack(&mut self, ack: &AckNack) {
        let sequences = expand_ack_records(&ack.records);
        for seq in sequences {
            self.sent_framesets.remove(&seq);
        }
    }

    /// Process a received NACK — re-queue the frames from the NACKed framesets.
    pub fn handle_nack(&mut self, nack: &AckNack) {
        let sequences = expand_ack_records(&nack.records);
        for seq in sequences {
            if let Some(sent) = self.sent_framesets.remove(&seq) {
                for frame in sent.frames {
                    self.send_queue.push_back(frame);
                }
            }
        }
    }

    /// Check for framesets that haven't been ACKed within the retransmit timeout.
    pub fn check_retransmit(&mut self, now: Instant) {
        let timed_out: Vec<u32> = self
            .sent_framesets
            .iter()
            .filter(|(_, fs)| now.duration_since(fs.sent_time) > RETRANSMIT_TIMEOUT)
            .map(|(&seq, _)| seq)
            .collect();

        for seq in timed_out {
            if let Some(sent) = self.sent_framesets.remove(&seq) {
                for frame in sent.frames {
                    self.send_queue.push_back(frame);
                }
            }
        }
    }

    /// Process an incoming FrameSet. Returns ordered, reassembled payloads.
    pub fn process_incoming_frameset(&mut self, frameset: FrameSet) -> Vec<Bytes> {
        let seq = frameset.sequence_number;

        // Track for ACK
        self.ack_queue.push(seq);

        // Detect gaps for NACK
        if let Some(highest) = self.recv_highest_sequence {
            if seq > highest + 1 {
                for missing in (highest + 1)..seq {
                    if !self.received_sequences.contains(&missing) {
                        self.nack_queue.push(missing);
                    }
                }
            }
        }

        // Update highest sequence
        match self.recv_highest_sequence {
            Some(h) if seq > h => self.recv_highest_sequence = Some(seq),
            None => self.recv_highest_sequence = Some(seq),
            _ => {}
        }
        self.received_sequences.insert(seq);

        let mut payloads = Vec::new();

        for frame in frameset.frames {
            // Dedup reliable frames
            if frame.reliability.is_reliable() {
                if let Some(idx) = frame.reliable_index {
                    if self.received_reliable_set.contains(&idx) {
                        continue; // Duplicate
                    }
                    self.received_reliable_set.insert(idx);
                }
            }

            // Handle fragmentation
            let body = if let Some(ref split) = frame.split {
                match self.fragment_assembler.insert(split, frame.body) {
                    Ok(Some(assembled)) => assembled,
                    Ok(None) => continue, // More fragments needed
                    Err(e) => {
                        tracing::warn!("fragment error from {}: {e}", self.addr);
                        continue;
                    }
                }
            } else {
                frame.body
            };

            // Handle ordering
            if frame.reliability.is_ordered() {
                if let (Some(ordered_idx), Some(channel)) =
                    (frame.ordered_index, frame.order_channel)
                {
                    let ordered = self.ordering.insert_ordered(channel, ordered_idx, body);
                    payloads.extend(ordered);
                } else {
                    payloads.push(body);
                }
            } else if frame.reliability.is_sequenced() {
                if let (Some(_seq_idx), Some(channel)) =
                    (frame.sequenced_index, frame.order_channel)
                {
                    if let Some(b) = self.ordering.insert_sequenced(channel, _seq_idx, body) {
                        payloads.push(b);
                    }
                } else {
                    payloads.push(body);
                }
            } else {
                payloads.push(body);
            }
        }

        payloads
    }

    /// Check if the session has timed out.
    pub fn is_timed_out(&self, now: Instant) -> bool {
        now.duration_since(self.last_activity) > SESSION_TIMEOUT
    }

    /// Check if it's time to send a ConnectedPing.
    pub fn should_ping(&self, now: Instant) -> bool {
        now.duration_since(self.last_ping_sent) > PING_INTERVAL
    }

    /// Clean up stale fragment assemblies.
    pub fn cleanup_fragments(&mut self) {
        self.fragment_assembler.cleanup(FRAGMENT_TIMEOUT);
    }
}
