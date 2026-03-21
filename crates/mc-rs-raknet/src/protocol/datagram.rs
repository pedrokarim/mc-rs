use crate::protocol::types::*;

/// Packet reliability mode (3-bit field in EncapsulatedPacket flags).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Reliability {
    Unreliable = 0,
    UnreliableSequenced = 1,
    Reliable = 2,
    ReliableOrdered = 3,
    ReliableSequenced = 4,
}

impl Reliability {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Unreliable),
            1 => Some(Self::UnreliableSequenced),
            2 => Some(Self::Reliable),
            3 => Some(Self::ReliableOrdered),
            4 => Some(Self::ReliableSequenced),
            _ => None,
        }
    }

    pub fn is_reliable(self) -> bool {
        matches!(
            self,
            Self::Reliable | Self::ReliableOrdered | Self::ReliableSequenced
        )
    }

    pub fn is_sequenced(self) -> bool {
        matches!(self, Self::UnreliableSequenced | Self::ReliableSequenced)
    }

    pub fn is_ordered(self) -> bool {
        matches!(self, Self::ReliableOrdered)
    }

    pub fn is_sequenced_or_ordered(self) -> bool {
        matches!(
            self,
            Self::UnreliableSequenced | Self::ReliableOrdered | Self::ReliableSequenced
        )
    }
}

/// Split packet info.
#[derive(Debug, Clone)]
pub struct SplitInfo {
    pub count: u32,
    pub id: u16,
    pub index: u32,
}

/// An encapsulated packet within a datagram.
#[derive(Debug, Clone)]
pub struct EncapsulatedPacket {
    pub reliability: Reliability,
    pub message_index: Option<u32>,    // 24-bit, if reliable
    pub sequence_index: Option<u32>,   // 24-bit, if sequenced
    pub order_index: Option<u32>,      // 24-bit, if ordered/sequenced
    pub order_channel: Option<u8>,     // if ordered/sequenced
    pub split: Option<SplitInfo>,
    pub body: Vec<u8>,
}

impl EncapsulatedPacket {
    /// Calculate the header size in bytes for this packet.
    pub fn header_size(&self) -> usize {
        let mut size = 3; // flags(1) + length(2)
        if self.reliability.is_reliable() {
            size += 3; // messageIndex
        }
        if self.reliability.is_sequenced() {
            size += 3; // sequenceIndex
        }
        if self.reliability.is_sequenced_or_ordered() {
            size += 4; // orderIndex(3) + orderChannel(1)
        }
        if self.split.is_some() {
            size += 10; // count(4) + id(2) + index(4)
        }
        size
    }

    /// Total size on wire.
    pub fn total_size(&self) -> usize {
        self.header_size() + self.body.len()
    }

    /// Decode an encapsulated packet from a buffer. Returns (packet, bytes_consumed).
    pub fn decode(buf: &[u8]) -> Option<(Self, usize)> {
        if buf.len() < 3 {
            return None;
        }
        let mut pos = 0;

        // Flags byte
        let flags = buf[pos];
        pos += 1;
        let reliability = Reliability::from_u8((flags >> 5) & 0x07)?;
        let has_split = (flags & 0x10) != 0;

        // Length in BITS (u16 BE)
        let length_bits = read_u16_be(&buf[pos..]);
        pos += 2;
        let body_len = (length_bits as usize).div_ceil(8);

        // Message index (if reliable)
        let message_index = if reliability.is_reliable() {
            if buf.len() < pos + 3 {
                return None;
            }
            let v = read_u24_le(&buf[pos..]);
            pos += 3;
            Some(v)
        } else {
            None
        };

        // Sequence index (if sequenced)
        let sequence_index = if reliability.is_sequenced() {
            if buf.len() < pos + 3 {
                return None;
            }
            let v = read_u24_le(&buf[pos..]);
            pos += 3;
            Some(v)
        } else {
            None
        };

        // Order index + channel (if ordered or sequenced)
        let (order_index, order_channel) = if reliability.is_sequenced_or_ordered() {
            if buf.len() < pos + 4 {
                return None;
            }
            let idx = read_u24_le(&buf[pos..]);
            pos += 3;
            let ch = buf[pos];
            pos += 1;
            (Some(idx), Some(ch))
        } else {
            (None, None)
        };

        // Split info
        let split = if has_split {
            if buf.len() < pos + 10 {
                return None;
            }
            let count = read_i32_be(&buf[pos..]) as u32;
            pos += 4;
            let id = read_u16_be(&buf[pos..]);
            pos += 2;
            let index = read_i32_be(&buf[pos..]) as u32;
            pos += 4;
            Some(SplitInfo { count, id, index })
        } else {
            None
        };

        // Body
        if buf.len() < pos + body_len {
            return None;
        }
        let body = buf[pos..pos + body_len].to_vec();
        pos += body_len;

        Some((
            Self {
                reliability,
                message_index,
                sequence_index,
                order_index,
                order_channel,
                split,
                body,
            },
            pos,
        ))
    }

    /// Encode this encapsulated packet to bytes.
    pub fn encode(&self, buf: &mut Vec<u8>) {
        // Flags
        let mut flags = (self.reliability as u8) << 5;
        if self.split.is_some() {
            flags |= 0x10;
        }
        buf.push(flags);

        // Length in BITS (u16 BE)
        write_u16_be(buf, (self.body.len() * 8) as u16);

        // Message index
        if self.reliability.is_reliable() {
            write_u24_le(buf, self.message_index.unwrap_or(0));
        }

        // Sequence index
        if self.reliability.is_sequenced() {
            write_u24_le(buf, self.sequence_index.unwrap_or(0));
        }

        // Order index + channel
        if self.reliability.is_sequenced_or_ordered() {
            write_u24_le(buf, self.order_index.unwrap_or(0));
            buf.push(self.order_channel.unwrap_or(0));
        }

        // Split info
        if let Some(ref split) = self.split {
            let count_bytes = (split.count as i32).to_be_bytes();
            buf.extend_from_slice(&count_bytes);
            write_u16_be(buf, split.id);
            let index_bytes = (split.index as i32).to_be_bytes();
            buf.extend_from_slice(&index_bytes);
        }

        // Body
        buf.extend_from_slice(&self.body);
    }
}

/// A RakNet datagram containing one or more encapsulated packets.
#[derive(Debug, Clone)]
pub struct Datagram {
    pub seq_number: u32, // 24-bit
    pub packets: Vec<EncapsulatedPacket>,
}

impl Datagram {
    /// Decode a datagram from raw bytes (excluding the first header byte).
    /// The header byte (0x80+) has already been checked by the caller.
    pub fn decode(buf: &[u8]) -> Option<Self> {
        if buf.len() < 4 {
            return None;
        }
        // Skip header byte (already consumed), seq number at offset 1
        let seq_number = read_u24_le(&buf[1..]);
        let mut pos = 4;
        let mut packets = Vec::new();

        while pos < buf.len() {
            let (packet, consumed) = EncapsulatedPacket::decode(&buf[pos..])?;
            pos += consumed;
            packets.push(packet);
        }

        Some(Self {
            seq_number,
            packets,
        })
    }

    /// Encode this datagram to bytes.
    pub fn encode(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(256);
        buf.push(0x84); // BITFLAG_VALID | some flags
        write_u24_le(&mut buf, self.seq_number);
        for packet in &self.packets {
            packet.encode(&mut buf);
        }
        buf
    }
}

// ── ACK/NACK ──

/// A range record in an ACK/NACK packet.
#[derive(Debug, Clone)]
pub enum AckRecord {
    Single(u32),
    Range(u32, u32),
}

/// Decode ACK/NACK records from a packet (excluding the ID byte).
pub fn decode_ack_records(buf: &[u8]) -> Vec<u32> {
    if buf.len() < 3 {
        return Vec::new();
    }
    let record_count = read_u16_be(&buf[1..]) as usize;
    let mut pos = 3;
    let mut result = Vec::new();

    for _ in 0..record_count {
        if pos >= buf.len() {
            break;
        }
        let record_type = buf[pos];
        pos += 1;

        if record_type == 0 {
            // Range
            if pos + 6 > buf.len() {
                break;
            }
            let min = read_u24_le(&buf[pos..]);
            pos += 3;
            let max = read_u24_le(&buf[pos..]);
            pos += 3;
            for seq in min..=max.min(min + 512) {
                result.push(seq);
            }
        } else {
            // Single
            if pos + 3 > buf.len() {
                break;
            }
            let seq = read_u24_le(&buf[pos..]);
            pos += 3;
            result.push(seq);
        }
    }

    result
}

/// Encode ACK/NACK packet from a list of sequence numbers.
pub fn encode_ack_nack(packet_id: u8, seq_numbers: &mut Vec<u32>) -> Vec<u8> {
    if seq_numbers.is_empty() {
        return Vec::new();
    }

    seq_numbers.sort_unstable();
    seq_numbers.dedup();

    let mut records: Vec<AckRecord> = Vec::new();
    let mut start = seq_numbers[0];
    let mut end = start;

    for &seq in &seq_numbers[1..] {
        if seq == end + 1 {
            end = seq;
        } else {
            if start == end {
                records.push(AckRecord::Single(start));
            } else {
                records.push(AckRecord::Range(start, end));
            }
            start = seq;
            end = seq;
        }
    }
    // Push last record
    if start == end {
        records.push(AckRecord::Single(start));
    } else {
        records.push(AckRecord::Range(start, end));
    }

    // Encode
    let mut buf = Vec::with_capacity(4 + records.len() * 7);
    buf.push(packet_id);
    write_u16_be(&mut buf, records.len() as u16);

    for record in &records {
        match record {
            AckRecord::Single(seq) => {
                buf.push(1); // single
                write_u24_le(&mut buf, *seq);
            }
            AckRecord::Range(min, max) => {
                buf.push(0); // range
                write_u24_le(&mut buf, *min);
                write_u24_le(&mut buf, *max);
            }
        }
    }

    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encapsulated_roundtrip_unreliable() {
        let pkt = EncapsulatedPacket {
            reliability: Reliability::Unreliable,
            message_index: None,
            sequence_index: None,
            order_index: None,
            order_channel: None,
            split: None,
            body: vec![0xFE, 0x01, 0x02, 0x03],
        };
        let mut buf = Vec::new();
        pkt.encode(&mut buf);
        let (decoded, consumed) = EncapsulatedPacket::decode(&buf).unwrap();
        assert_eq!(consumed, buf.len());
        assert_eq!(decoded.reliability, Reliability::Unreliable);
        assert_eq!(decoded.body, vec![0xFE, 0x01, 0x02, 0x03]);
        assert!(decoded.message_index.is_none());
    }

    #[test]
    fn test_encapsulated_roundtrip_reliable_ordered() {
        let pkt = EncapsulatedPacket {
            reliability: Reliability::ReliableOrdered,
            message_index: Some(42),
            sequence_index: None,
            order_index: Some(7),
            order_channel: Some(0),
            split: None,
            body: vec![0xAA, 0xBB],
        };
        let mut buf = Vec::new();
        pkt.encode(&mut buf);
        let (decoded, _) = EncapsulatedPacket::decode(&buf).unwrap();
        assert_eq!(decoded.reliability, Reliability::ReliableOrdered);
        assert_eq!(decoded.message_index, Some(42));
        assert_eq!(decoded.order_index, Some(7));
        assert_eq!(decoded.order_channel, Some(0));
        assert_eq!(decoded.body, vec![0xAA, 0xBB]);
    }

    #[test]
    fn test_encapsulated_with_split() {
        let pkt = EncapsulatedPacket {
            reliability: Reliability::Reliable,
            message_index: Some(10),
            sequence_index: None,
            order_index: None,
            order_channel: None,
            split: Some(SplitInfo {
                count: 3,
                id: 1,
                index: 0,
            }),
            body: vec![0x01, 0x02],
        };
        let mut buf = Vec::new();
        pkt.encode(&mut buf);
        let (decoded, _) = EncapsulatedPacket::decode(&buf).unwrap();
        assert!(decoded.split.is_some());
        let split = decoded.split.unwrap();
        assert_eq!(split.count, 3);
        assert_eq!(split.id, 1);
        assert_eq!(split.index, 0);
    }

    #[test]
    fn test_datagram_roundtrip() {
        let dg = Datagram {
            seq_number: 123,
            packets: vec![EncapsulatedPacket {
                reliability: Reliability::ReliableOrdered,
                message_index: Some(0),
                sequence_index: None,
                order_index: Some(0),
                order_channel: Some(0),
                split: None,
                body: vec![0xFE, 0x00, 0x01],
            }],
        };
        let encoded = dg.encode();
        let decoded = Datagram::decode(&encoded).unwrap();
        assert_eq!(decoded.seq_number, 123);
        assert_eq!(decoded.packets.len(), 1);
        assert_eq!(decoded.packets[0].body, vec![0xFE, 0x00, 0x01]);
    }

    #[test]
    fn test_ack_encode_decode() {
        let mut seqs = vec![1, 2, 3, 5, 7, 8, 9, 10];
        let encoded = encode_ack_nack(0xC0, &mut seqs);
        assert_eq!(encoded[0], 0xC0);
        let decoded = decode_ack_records(&encoded);
        assert_eq!(decoded, vec![1, 2, 3, 5, 7, 8, 9, 10]);
    }

    #[test]
    fn test_ack_single() {
        let mut seqs = vec![42];
        let encoded = encode_ack_nack(0xC0, &mut seqs);
        let decoded = decode_ack_records(&encoded);
        assert_eq!(decoded, vec![42]);
    }

    #[test]
    fn test_length_in_bits() {
        let pkt = EncapsulatedPacket {
            reliability: Reliability::Unreliable,
            message_index: None,
            sequence_index: None,
            order_index: None,
            order_channel: None,
            split: None,
            body: vec![0x01, 0x02, 0x03], // 3 bytes = 24 bits
        };
        let mut buf = Vec::new();
        pkt.encode(&mut buf);
        // Length field at offset 1-2 should be 24 (0x0018) in BE
        assert_eq!(buf[1], 0x00);
        assert_eq!(buf[2], 0x18); // 24
    }
}
