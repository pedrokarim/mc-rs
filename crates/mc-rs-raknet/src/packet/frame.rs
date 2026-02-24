use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::codec::{read_u24_le, write_u24_le};
use crate::error::RakNetError;

/// Reliability types for RakNet frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Reliability {
    Unreliable = 0,
    UnreliableSequenced = 1,
    Reliable = 2,
    ReliableOrdered = 3,
    ReliableSequenced = 4,
    UnreliableWithAckReceipt = 5,
    ReliableWithAckReceipt = 6,
    ReliableOrderedWithAckReceipt = 7,
}

impl Reliability {
    pub fn from_u8(v: u8) -> Result<Self, RakNetError> {
        match v {
            0 => Ok(Self::Unreliable),
            1 => Ok(Self::UnreliableSequenced),
            2 => Ok(Self::Reliable),
            3 => Ok(Self::ReliableOrdered),
            4 => Ok(Self::ReliableSequenced),
            5 => Ok(Self::UnreliableWithAckReceipt),
            6 => Ok(Self::ReliableWithAckReceipt),
            7 => Ok(Self::ReliableOrderedWithAckReceipt),
            _ => Err(RakNetError::InvalidReliability(v)),
        }
    }

    pub fn is_reliable(self) -> bool {
        matches!(
            self,
            Self::Reliable
                | Self::ReliableOrdered
                | Self::ReliableSequenced
                | Self::ReliableWithAckReceipt
                | Self::ReliableOrderedWithAckReceipt
        )
    }

    pub fn is_ordered(self) -> bool {
        matches!(
            self,
            Self::ReliableOrdered | Self::ReliableOrderedWithAckReceipt
        )
    }

    pub fn is_sequenced(self) -> bool {
        matches!(self, Self::UnreliableSequenced | Self::ReliableSequenced)
    }
}

/// Split/fragment metadata for a frame.
#[derive(Debug, Clone, Copy)]
pub struct SplitInfo {
    pub count: u32,
    pub id: u16,
    pub index: u32,
}

/// A single frame within a FrameSet.
#[derive(Debug, Clone)]
pub struct Frame {
    pub reliability: Reliability,
    pub reliable_index: Option<u32>,
    pub sequenced_index: Option<u32>,
    pub ordered_index: Option<u32>,
    pub order_channel: Option<u8>,
    pub split: Option<SplitInfo>,
    pub body: Bytes,
}

impl Frame {
    /// Decode a single frame from a buffer.
    pub fn decode(buf: &mut impl Buf) -> Result<Self, RakNetError> {
        if buf.remaining() < 3 {
            return Err(RakNetError::PacketTooShort {
                expected: 3,
                actual: buf.remaining(),
            });
        }

        let flags = buf.get_u8();
        let reliability = Reliability::from_u8((flags >> 5) & 0x07)?;
        let is_split = (flags >> 4) & 0x01 == 1;

        let body_length_bits = buf.get_u16() as usize;
        let body_length = body_length_bits.div_ceil(8);

        let reliable_index = if reliability.is_reliable() {
            Some(read_u24_le(buf))
        } else {
            None
        };

        let sequenced_index = if reliability.is_sequenced() {
            Some(read_u24_le(buf))
        } else {
            None
        };

        let (ordered_index, order_channel) =
            if reliability.is_ordered() || reliability.is_sequenced() {
                (Some(read_u24_le(buf)), Some(buf.get_u8()))
            } else {
                (None, None)
            };

        let split = if is_split {
            Some(SplitInfo {
                count: buf.get_u32(),
                id: buf.get_u16(),
                index: buf.get_u32(),
            })
        } else {
            None
        };

        if buf.remaining() < body_length {
            return Err(RakNetError::PacketTooShort {
                expected: body_length,
                actual: buf.remaining(),
            });
        }
        let body = buf.copy_to_bytes(body_length);

        Ok(Self {
            reliability,
            reliable_index,
            sequenced_index,
            ordered_index,
            order_channel,
            split,
            body,
        })
    }

    /// Encode a single frame into a buffer.
    pub fn encode(&self, buf: &mut BytesMut) {
        let mut flags = (self.reliability as u8) << 5;
        if self.split.is_some() {
            flags |= 1 << 4;
        }
        buf.put_u8(flags);
        buf.put_u16((self.body.len() * 8) as u16);

        if let Some(idx) = self.reliable_index {
            write_u24_le(buf, idx);
        }
        if let Some(idx) = self.sequenced_index {
            write_u24_le(buf, idx);
        }
        if let Some(idx) = self.ordered_index {
            write_u24_le(buf, idx);
            buf.put_u8(self.order_channel.unwrap_or(0));
        }
        if let Some(split) = &self.split {
            buf.put_u32(split.count);
            buf.put_u16(split.id);
            buf.put_u32(split.index);
        }
        buf.put_slice(&self.body);
    }

    /// Calculate the encoded size of this frame in bytes.
    pub fn encoded_size(&self) -> usize {
        let mut size = 1 + 2 + self.body.len(); // flags + body_length_bits + body
        if self.reliability.is_reliable() {
            size += 3;
        }
        if self.reliability.is_sequenced() {
            size += 3;
        }
        if self.reliability.is_ordered() || self.reliability.is_sequenced() {
            size += 4; // ordered_index(3) + order_channel(1)
        }
        if self.split.is_some() {
            size += 10; // count(4) + id(2) + index(4)
        }
        size
    }
}

/// A set of frames with a sequence number, sent as a single UDP datagram.
#[derive(Debug)]
pub struct FrameSet {
    pub sequence_number: u32,
    pub frames: Vec<Frame>,
}

impl FrameSet {
    pub fn decode(data: &[u8]) -> Result<Self, RakNetError> {
        if data.len() < 4 {
            return Err(RakNetError::PacketTooShort {
                expected: 4,
                actual: data.len(),
            });
        }
        let mut buf = std::io::Cursor::new(data);
        let _packet_id = buf.get_u8(); // 0x80-0x8D
        let sequence_number = read_u24_le(&mut buf);

        let mut frames = Vec::new();
        while buf.remaining() > 0 {
            frames.push(Frame::decode(&mut buf)?);
        }

        Ok(Self {
            sequence_number,
            frames,
        })
    }

    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u8(0x84); // FrameSet packet ID
        write_u24_le(buf, self.sequence_number);
        for frame in &self.frames {
            frame.encode(buf);
        }
    }
}

/// An ACK or NACK packet.
#[derive(Debug)]
pub struct AckNack {
    pub is_ack: bool,
    pub records: Vec<AckRecord>,
}

/// A single ACK/NACK record — either a single sequence number or a range.
#[derive(Debug, Clone)]
pub enum AckRecord {
    Single(u32),
    Range { min: u32, max: u32 },
}

impl AckNack {
    pub const ACK_ID: u8 = 0xC0;
    pub const NACK_ID: u8 = 0xA0;

    pub fn decode(data: &[u8]) -> Result<Self, RakNetError> {
        if data.len() < 3 {
            return Err(RakNetError::PacketTooShort {
                expected: 3,
                actual: data.len(),
            });
        }
        let mut buf = std::io::Cursor::new(data);
        let packet_id = buf.get_u8();
        let is_ack = packet_id == Self::ACK_ID;
        let record_count = buf.get_u16() as usize;

        let mut records = Vec::with_capacity(record_count);
        for _ in 0..record_count {
            let is_range = buf.get_u8() == 0; // 0 = range, 1 = single (inverted!)
            if is_range {
                let min = read_u24_le(&mut buf);
                let max = read_u24_le(&mut buf);
                records.push(AckRecord::Range { min, max });
            } else {
                let seq = read_u24_le(&mut buf);
                records.push(AckRecord::Single(seq));
            }
        }

        Ok(Self { is_ack, records })
    }

    pub fn encode(&self, buf: &mut BytesMut) {
        buf.put_u8(if self.is_ack {
            Self::ACK_ID
        } else {
            Self::NACK_ID
        });
        buf.put_u16(self.records.len() as u16);
        for record in &self.records {
            match record {
                AckRecord::Single(seq) => {
                    buf.put_u8(1); // not a range
                    write_u24_le(buf, *seq);
                }
                AckRecord::Range { min, max } => {
                    buf.put_u8(0); // is a range
                    write_u24_le(buf, *min);
                    write_u24_le(buf, *max);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reliability_from_u8() {
        assert_eq!(
            Reliability::from_u8(3).unwrap(),
            Reliability::ReliableOrdered
        );
        assert!(Reliability::from_u8(8).is_err());
    }

    #[test]
    fn reliability_traits() {
        assert!(Reliability::ReliableOrdered.is_reliable());
        assert!(Reliability::ReliableOrdered.is_ordered());
        assert!(!Reliability::ReliableOrdered.is_sequenced());
        assert!(!Reliability::Unreliable.is_reliable());
        assert!(Reliability::UnreliableSequenced.is_sequenced());
    }

    #[test]
    fn frame_roundtrip_unreliable() {
        let frame = Frame {
            reliability: Reliability::Unreliable,
            reliable_index: None,
            sequenced_index: None,
            ordered_index: None,
            order_channel: None,
            split: None,
            body: Bytes::from_static(b"hello"),
        };
        let mut buf = BytesMut::new();
        frame.encode(&mut buf);
        let mut cursor = std::io::Cursor::new(&buf[..]);
        let decoded = Frame::decode(&mut cursor).unwrap();
        assert_eq!(decoded.reliability, Reliability::Unreliable);
        assert_eq!(decoded.body, Bytes::from_static(b"hello"));
        assert!(decoded.split.is_none());
    }

    #[test]
    fn frame_roundtrip_reliable_ordered() {
        let frame = Frame {
            reliability: Reliability::ReliableOrdered,
            reliable_index: Some(42),
            sequenced_index: None,
            ordered_index: Some(7),
            order_channel: Some(0),
            split: None,
            body: Bytes::from_static(b"world"),
        };
        let mut buf = BytesMut::new();
        frame.encode(&mut buf);
        let mut cursor = std::io::Cursor::new(&buf[..]);
        let decoded = Frame::decode(&mut cursor).unwrap();
        assert_eq!(decoded.reliability, Reliability::ReliableOrdered);
        assert_eq!(decoded.reliable_index, Some(42));
        assert_eq!(decoded.ordered_index, Some(7));
        assert_eq!(decoded.order_channel, Some(0));
        assert_eq!(decoded.body, Bytes::from_static(b"world"));
    }

    #[test]
    fn frame_roundtrip_split() {
        let frame = Frame {
            reliability: Reliability::Reliable,
            reliable_index: Some(100),
            sequenced_index: None,
            ordered_index: None,
            order_channel: None,
            split: Some(SplitInfo {
                count: 3,
                id: 1,
                index: 0,
            }),
            body: Bytes::from_static(b"fragment"),
        };
        let mut buf = BytesMut::new();
        frame.encode(&mut buf);
        let mut cursor = std::io::Cursor::new(&buf[..]);
        let decoded = Frame::decode(&mut cursor).unwrap();
        assert!(decoded.split.is_some());
        let split = decoded.split.unwrap();
        assert_eq!(split.count, 3);
        assert_eq!(split.id, 1);
        assert_eq!(split.index, 0);
    }

    #[test]
    fn frameset_roundtrip() {
        let fs = FrameSet {
            sequence_number: 123,
            frames: vec![Frame {
                reliability: Reliability::Unreliable,
                reliable_index: None,
                sequenced_index: None,
                ordered_index: None,
                order_channel: None,
                split: None,
                body: Bytes::from_static(b"test"),
            }],
        };
        let mut buf = BytesMut::new();
        fs.encode(&mut buf);
        let decoded = FrameSet::decode(&buf).unwrap();
        assert_eq!(decoded.sequence_number, 123);
        assert_eq!(decoded.frames.len(), 1);
        assert_eq!(decoded.frames[0].body, Bytes::from_static(b"test"));
    }

    #[test]
    fn ack_roundtrip() {
        let ack = AckNack {
            is_ack: true,
            records: vec![AckRecord::Single(5), AckRecord::Range { min: 10, max: 15 }],
        };
        let mut buf = BytesMut::new();
        ack.encode(&mut buf);
        let decoded = AckNack::decode(&buf).unwrap();
        assert!(decoded.is_ack);
        assert_eq!(decoded.records.len(), 2);
    }

    #[test]
    fn nack_roundtrip() {
        let nack = AckNack {
            is_ack: false,
            records: vec![AckRecord::Range { min: 0, max: 3 }],
        };
        let mut buf = BytesMut::new();
        nack.encode(&mut buf);
        assert_eq!(buf[0], AckNack::NACK_ID);
        let decoded = AckNack::decode(&buf).unwrap();
        assert!(!decoded.is_ack);
    }
}
