//! Base data types used throughout the Bedrock protocol.

use std::fmt;
use std::ops::{Add, Mul, Neg, Range, Sub};

use bytes::{Buf, BufMut};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::codec::{ProtoDecode, ProtoEncode};
use crate::error::ProtoError;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum VarIntError {
    #[error("buffer too short")]
    BufferTooShort,
    #[error("VarInt is too long (more than {max_bytes} bytes)")]
    TooManyBytes { max_bytes: usize },
}

// ---------------------------------------------------------------------------
// VarInt (i32 — ZigZag + LEB128)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VarInt(pub i32);

impl VarInt {
    /// Maximum bytes a VarInt can occupy.
    pub const MAX_BYTES: usize = 5;

    /// Encode into the provided buffer and return the number of bytes written.
    pub fn encode(&self, buf: &mut Vec<u8>) -> usize {
        let mut value = zigzag_encode_32(self.0);
        let mut written = 0;
        loop {
            if value & !0x7F == 0 {
                buf.push(value as u8);
                written += 1;
                return written;
            }
            buf.push((value & 0x7F | 0x80) as u8);
            value >>= 7;
            written += 1;
        }
    }

    /// Decode from a byte slice. Returns the value and the number of bytes consumed.
    pub fn decode(buf: &[u8]) -> Result<(Self, usize), VarIntError> {
        let mut result: u32 = 0;
        let mut shift: u32 = 0;
        for (i, &byte) in buf.iter().enumerate() {
            if i >= Self::MAX_BYTES {
                return Err(VarIntError::TooManyBytes {
                    max_bytes: Self::MAX_BYTES,
                });
            }
            result |= ((byte & 0x7F) as u32) << shift;
            if byte & 0x80 == 0 {
                return Ok((VarInt(zigzag_decode_32(result)), i + 1));
            }
            shift += 7;
        }
        Err(VarIntError::BufferTooShort)
    }
}

impl ProtoEncode for VarInt {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        let mut value = zigzag_encode_32(self.0);
        loop {
            if value & !0x7F == 0 {
                buf.put_u8(value as u8);
                return;
            }
            buf.put_u8((value & 0x7F | 0x80) as u8);
            value >>= 7;
        }
    }
}

impl ProtoDecode for VarInt {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let mut result: u32 = 0;
        let mut shift: u32 = 0;
        for i in 0..Self::MAX_BYTES {
            if !buf.has_remaining() {
                return Err(VarIntError::BufferTooShort.into());
            }
            let byte = buf.get_u8();
            result |= ((byte & 0x7F) as u32) << shift;
            if byte & 0x80 == 0 {
                return Ok(VarInt(zigzag_decode_32(result)));
            }
            shift += 7;
            if i == Self::MAX_BYTES - 1 {
                return Err(VarIntError::TooManyBytes {
                    max_bytes: Self::MAX_BYTES,
                }
                .into());
            }
        }
        Err(VarIntError::BufferTooShort.into())
    }
}

impl From<i32> for VarInt {
    fn from(v: i32) -> Self {
        VarInt(v)
    }
}

impl From<VarInt> for i32 {
    fn from(v: VarInt) -> Self {
        v.0
    }
}

impl fmt::Debug for VarInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarInt({})", self.0)
    }
}

impl fmt::Display for VarInt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// VarLong (i64 — ZigZag + LEB128)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VarLong(pub i64);

impl VarLong {
    /// Maximum bytes a VarLong can occupy.
    pub const MAX_BYTES: usize = 10;

    /// Encode into the provided buffer and return the number of bytes written.
    pub fn encode(&self, buf: &mut Vec<u8>) -> usize {
        let mut value = zigzag_encode_64(self.0);
        let mut written = 0;
        loop {
            if value & !0x7F == 0 {
                buf.push(value as u8);
                written += 1;
                return written;
            }
            buf.push((value & 0x7F | 0x80) as u8);
            value >>= 7;
            written += 1;
        }
    }

    /// Decode from a byte slice. Returns the value and the number of bytes consumed.
    pub fn decode(buf: &[u8]) -> Result<(Self, usize), VarIntError> {
        let mut result: u64 = 0;
        let mut shift: u32 = 0;
        for (i, &byte) in buf.iter().enumerate() {
            if i >= Self::MAX_BYTES {
                return Err(VarIntError::TooManyBytes {
                    max_bytes: Self::MAX_BYTES,
                });
            }
            result |= ((byte & 0x7F) as u64) << shift;
            if byte & 0x80 == 0 {
                return Ok((VarLong(zigzag_decode_64(result)), i + 1));
            }
            shift += 7;
        }
        Err(VarIntError::BufferTooShort)
    }
}

impl ProtoEncode for VarLong {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        let mut value = zigzag_encode_64(self.0);
        loop {
            if value & !0x7F == 0 {
                buf.put_u8(value as u8);
                return;
            }
            buf.put_u8((value & 0x7F | 0x80) as u8);
            value >>= 7;
        }
    }
}

impl ProtoDecode for VarLong {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let mut result: u64 = 0;
        let mut shift: u32 = 0;
        for i in 0..Self::MAX_BYTES {
            if !buf.has_remaining() {
                return Err(VarIntError::BufferTooShort.into());
            }
            let byte = buf.get_u8();
            result |= ((byte & 0x7F) as u64) << shift;
            if byte & 0x80 == 0 {
                return Ok(VarLong(zigzag_decode_64(result)));
            }
            shift += 7;
            if i == Self::MAX_BYTES - 1 {
                return Err(VarIntError::TooManyBytes {
                    max_bytes: Self::MAX_BYTES,
                }
                .into());
            }
        }
        Err(VarIntError::BufferTooShort.into())
    }
}

impl From<i64> for VarLong {
    fn from(v: i64) -> Self {
        VarLong(v)
    }
}

impl From<VarLong> for i64 {
    fn from(v: VarLong) -> Self {
        v.0
    }
}

impl fmt::Debug for VarLong {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarLong({})", self.0)
    }
}

impl fmt::Display for VarLong {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// VarUInt32 (unsigned LEB128, NO ZigZag)
// ---------------------------------------------------------------------------

/// Unsigned variable-length integer (plain LEB128, NO ZigZag).
/// Used for packet lengths, string lengths, and packet IDs in Bedrock.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarUInt32(pub u32);

impl VarUInt32 {
    pub const MAX_BYTES: usize = 5;
}

impl ProtoEncode for VarUInt32 {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        let mut value = self.0;
        loop {
            if value & !0x7F == 0 {
                buf.put_u8(value as u8);
                return;
            }
            buf.put_u8((value & 0x7F | 0x80) as u8);
            value >>= 7;
        }
    }
}

impl ProtoDecode for VarUInt32 {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let mut result: u32 = 0;
        let mut shift: u32 = 0;
        for i in 0..Self::MAX_BYTES {
            if !buf.has_remaining() {
                return Err(VarIntError::BufferTooShort.into());
            }
            let byte = buf.get_u8();
            result |= ((byte & 0x7F) as u32) << shift;
            if byte & 0x80 == 0 {
                return Ok(VarUInt32(result));
            }
            shift += 7;
            if i == Self::MAX_BYTES - 1 {
                return Err(VarIntError::TooManyBytes {
                    max_bytes: Self::MAX_BYTES,
                }
                .into());
            }
        }
        Err(VarIntError::BufferTooShort.into())
    }
}

impl From<u32> for VarUInt32 {
    fn from(v: u32) -> Self {
        VarUInt32(v)
    }
}

impl From<VarUInt32> for u32 {
    fn from(v: VarUInt32) -> Self {
        v.0
    }
}

impl fmt::Debug for VarUInt32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarUInt32({})", self.0)
    }
}

impl fmt::Display for VarUInt32 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// VarUInt64 (unsigned LEB128, NO ZigZag)
// ---------------------------------------------------------------------------

/// Unsigned variable-length 64-bit integer (plain LEB128, NO ZigZag).
/// Used for EntityRuntimeID and other unsigned 64-bit fields in Bedrock.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarUInt64(pub u64);

impl VarUInt64 {
    pub const MAX_BYTES: usize = 10;
}

impl ProtoEncode for VarUInt64 {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        let mut value = self.0;
        loop {
            if value & !0x7F == 0 {
                buf.put_u8(value as u8);
                return;
            }
            buf.put_u8((value & 0x7F | 0x80) as u8);
            value >>= 7;
        }
    }
}

impl ProtoDecode for VarUInt64 {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let mut result: u64 = 0;
        let mut shift: u32 = 0;
        for i in 0..Self::MAX_BYTES {
            if !buf.has_remaining() {
                return Err(VarIntError::BufferTooShort.into());
            }
            let byte = buf.get_u8();
            result |= ((byte & 0x7F) as u64) << shift;
            if byte & 0x80 == 0 {
                return Ok(VarUInt64(result));
            }
            shift += 7;
            if i == Self::MAX_BYTES - 1 {
                return Err(VarIntError::TooManyBytes {
                    max_bytes: Self::MAX_BYTES,
                }
                .into());
            }
        }
        Err(VarIntError::BufferTooShort.into())
    }
}

impl From<u64> for VarUInt64 {
    fn from(v: u64) -> Self {
        VarUInt64(v)
    }
}

impl From<VarUInt64> for u64 {
    fn from(v: VarUInt64) -> Self {
        v.0
    }
}

impl fmt::Debug for VarUInt64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VarUInt64({})", self.0)
    }
}

impl fmt::Display for VarUInt64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ---------------------------------------------------------------------------
// ZigZag encoding helpers
// ---------------------------------------------------------------------------

#[inline]
fn zigzag_encode_32(v: i32) -> u32 {
    ((v << 1) ^ (v >> 31)) as u32
}

#[inline]
fn zigzag_decode_32(v: u32) -> i32 {
    (v >> 1) as i32 ^ -((v & 1) as i32)
}

#[inline]
fn zigzag_encode_64(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

#[inline]
fn zigzag_decode_64(v: u64) -> i64 {
    (v >> 1) as i64 ^ -((v & 1) as i64)
}

// ---------------------------------------------------------------------------
// Vec3 (f32 x, y, z)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn length(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn distance(&self, other: &Vec3) -> f32 {
        (*self - *other).length()
    }
}

impl ProtoEncode for Vec3 {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_f32_le(self.x);
        buf.put_f32_le(self.y);
        buf.put_f32_le(self.z);
    }
}

impl ProtoDecode for Vec3 {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        if buf.remaining() < 12 {
            return Err(ProtoError::BufferTooShort {
                needed: 12,
                remaining: buf.remaining(),
            });
        }
        Ok(Self {
            x: buf.get_f32_le(),
            y: buf.get_f32_le(),
            z: buf.get_f32_le(),
        })
    }
}

impl Add for Vec3 {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Mul<f32> for Vec3 {
    type Output = Self;
    fn mul(self, rhs: f32) -> Self {
        Self::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl Neg for Vec3 {
    type Output = Self;
    fn neg(self) -> Self {
        Self::new(-self.x, -self.y, -self.z)
    }
}

impl fmt::Display for Vec3 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

// ---------------------------------------------------------------------------
// Vec2 (f32 x, z)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f32,
    pub z: f32,
}

impl Vec2 {
    pub const ZERO: Self = Self { x: 0.0, z: 0.0 };

    pub fn new(x: f32, z: f32) -> Self {
        Self { x, z }
    }
}

impl ProtoEncode for Vec2 {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_f32_le(self.x);
        buf.put_f32_le(self.z);
    }
}

impl ProtoDecode for Vec2 {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        if buf.remaining() < 8 {
            return Err(ProtoError::BufferTooShort {
                needed: 8,
                remaining: buf.remaining(),
            });
        }
        Ok(Self {
            x: buf.get_f32_le(),
            z: buf.get_f32_le(),
        })
    }
}

impl fmt::Display for Vec2 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.z)
    }
}

// ---------------------------------------------------------------------------
// Uuid (Bedrock: two i64 LE)
// ---------------------------------------------------------------------------

/// 128-bit UUID as stored by Bedrock: two little-endian i64 values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Uuid {
    pub most_significant: u64,
    pub least_significant: u64,
}

impl Uuid {
    pub const ZERO: Self = Self {
        most_significant: 0,
        least_significant: 0,
    };

    pub fn new(most: u64, least: u64) -> Self {
        Self {
            most_significant: most,
            least_significant: least,
        }
    }
}

impl ProtoEncode for Uuid {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u64_le(self.most_significant);
        buf.put_u64_le(self.least_significant);
    }
}

impl ProtoDecode for Uuid {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        if buf.remaining() < 16 {
            return Err(ProtoError::BufferTooShort {
                needed: 16,
                remaining: buf.remaining(),
            });
        }
        Ok(Self {
            most_significant: buf.get_u64_le(),
            least_significant: buf.get_u64_le(),
        })
    }
}

impl fmt::Display for Uuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let bytes = {
            let mut b = [0u8; 16];
            b[..8].copy_from_slice(&self.most_significant.to_be_bytes());
            b[8..].copy_from_slice(&self.least_significant.to_be_bytes());
            b
        };
        write!(
            f,
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5],
            bytes[6], bytes[7],
            bytes[8], bytes[9],
            bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
        )
    }
}

// ---------------------------------------------------------------------------
// BlockPos (i32 x, y, z)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl BlockPos {
    pub fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Convert to the chunk position that contains this block.
    pub fn chunk_pos(&self) -> ChunkPos {
        ChunkPos::new(self.x >> 4, self.z >> 4)
    }

    /// Convert a floating-point position to a block position (floor).
    pub fn from_vec3(v: &Vec3) -> Self {
        Self {
            x: v.x.floor() as i32,
            y: v.y.floor() as i32,
            z: v.z.floor() as i32,
        }
    }
}

/// Wire format: VarInt32(x, zigzag) + VarUInt32(y) + VarInt32(z, zigzag).
impl ProtoEncode for BlockPos {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarInt(self.x).proto_encode(buf);
        VarUInt32(self.y as u32).proto_encode(buf);
        VarInt(self.z).proto_encode(buf);
    }
}

impl ProtoDecode for BlockPos {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let x = VarInt::proto_decode(buf)?.0;
        let y = VarUInt32::proto_decode(buf)?.0 as i32;
        let z = VarInt::proto_decode(buf)?.0;
        Ok(Self { x, y, z })
    }
}

impl fmt::Display for BlockPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

// ---------------------------------------------------------------------------
// ChunkPos (i32 x, z)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkPos {
    pub x: i32,
    pub z: i32,
}

impl ChunkPos {
    pub fn new(x: i32, z: i32) -> Self {
        Self { x, z }
    }

    /// Range of block X coordinates within this chunk.
    pub fn block_x_range(&self) -> Range<i32> {
        let start = self.x << 4;
        start..start + 16
    }

    /// Range of block Z coordinates within this chunk.
    pub fn block_z_range(&self) -> Range<i32> {
        let start = self.z << 4;
        start..start + 16
    }
}

impl fmt::Display for ChunkPos {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.z)
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    // -- VarInt (legacy API) --

    fn roundtrip_varint(value: i32) {
        let vi = VarInt(value);
        let mut buf = Vec::new();
        let written = vi.encode(&mut buf);
        let (decoded, consumed) = VarInt::decode(&buf).unwrap();
        assert_eq!(decoded.0, value, "VarInt roundtrip failed for {value}");
        assert_eq!(written, consumed);
    }

    #[test]
    fn varint_zero() {
        roundtrip_varint(0);
    }

    #[test]
    fn varint_positive() {
        roundtrip_varint(1);
        roundtrip_varint(127);
        roundtrip_varint(128);
        roundtrip_varint(255);
        roundtrip_varint(1000);
        roundtrip_varint(100_000);
    }

    #[test]
    fn varint_negative() {
        roundtrip_varint(-1);
        roundtrip_varint(-127);
        roundtrip_varint(-128);
        roundtrip_varint(-1000);
        roundtrip_varint(-100_000);
    }

    #[test]
    fn varint_extremes() {
        roundtrip_varint(i32::MAX);
        roundtrip_varint(i32::MIN);
    }

    #[test]
    fn varint_buffer_too_short() {
        assert!(VarInt::decode(&[]).is_err());
        assert!(VarInt::decode(&[0x80]).is_err());
    }

    #[test]
    fn varint_from_into() {
        let vi: VarInt = 42.into();
        let raw: i32 = vi.into();
        assert_eq!(raw, 42);
    }

    // -- VarInt (proto API) --

    fn roundtrip_varint_proto(value: i32) {
        let vi = VarInt(value);
        let mut buf = BytesMut::new();
        vi.proto_encode(&mut buf);
        let decoded = VarInt::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded.0, value);
    }

    #[test]
    fn varint_proto_roundtrip() {
        roundtrip_varint_proto(0);
        roundtrip_varint_proto(1);
        roundtrip_varint_proto(-1);
        roundtrip_varint_proto(i32::MAX);
        roundtrip_varint_proto(i32::MIN);
    }

    // -- VarLong --

    fn roundtrip_varlong(value: i64) {
        let vl = VarLong(value);
        let mut buf = Vec::new();
        let written = vl.encode(&mut buf);
        let (decoded, consumed) = VarLong::decode(&buf).unwrap();
        assert_eq!(decoded.0, value, "VarLong roundtrip failed for {value}");
        assert_eq!(written, consumed);
    }

    #[test]
    fn varlong_zero() {
        roundtrip_varlong(0);
    }

    #[test]
    fn varlong_positive() {
        roundtrip_varlong(1);
        roundtrip_varlong(1_000_000_000);
    }

    #[test]
    fn varlong_negative() {
        roundtrip_varlong(-1);
        roundtrip_varlong(-1_000_000_000);
    }

    #[test]
    fn varlong_extremes() {
        roundtrip_varlong(i64::MAX);
        roundtrip_varlong(i64::MIN);
    }

    // -- VarUInt32 --

    fn roundtrip_varuint32(value: u32) {
        let v = VarUInt32(value);
        let mut buf = BytesMut::new();
        v.proto_encode(&mut buf);
        let decoded = VarUInt32::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded.0, value);
    }

    #[test]
    fn varuint32_zero() {
        roundtrip_varuint32(0);
    }

    #[test]
    fn varuint32_small() {
        roundtrip_varuint32(1);
        roundtrip_varuint32(127);
        roundtrip_varuint32(128);
        roundtrip_varuint32(255);
        roundtrip_varuint32(300);
    }

    #[test]
    fn varuint32_large() {
        roundtrip_varuint32(100_000);
        roundtrip_varuint32(u32::MAX);
    }

    #[test]
    fn varuint32_not_zigzag() {
        // VarUInt32(1) should encode as [0x01], not ZigZag [0x02]
        let mut buf = BytesMut::new();
        VarUInt32(1).proto_encode(&mut buf);
        assert_eq!(&buf[..], &[0x01]);

        // VarInt(1) with ZigZag encodes 1 as (1<<1)^(1>>31) = 2
        let mut buf2 = BytesMut::new();
        VarInt(1).proto_encode(&mut buf2);
        assert_eq!(&buf2[..], &[0x02]);
    }

    // -- VarUInt64 --

    fn roundtrip_varuint64(value: u64) {
        let v = VarUInt64(value);
        let mut buf = BytesMut::new();
        v.proto_encode(&mut buf);
        let decoded = VarUInt64::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded.0, value);
    }

    #[test]
    fn varuint64_zero() {
        roundtrip_varuint64(0);
    }

    #[test]
    fn varuint64_small() {
        roundtrip_varuint64(1);
        roundtrip_varuint64(127);
        roundtrip_varuint64(128);
        roundtrip_varuint64(255);
    }

    #[test]
    fn varuint64_large() {
        roundtrip_varuint64(u32::MAX as u64);
        roundtrip_varuint64(u64::MAX);
    }

    #[test]
    fn varuint64_not_zigzag() {
        // VarUInt64(1) should encode as [0x01], not ZigZag [0x02]
        let mut buf = BytesMut::new();
        VarUInt64(1).proto_encode(&mut buf);
        assert_eq!(&buf[..], &[0x01]);
    }

    // -- Vec3 --

    #[test]
    fn vec3_zero() {
        assert_eq!(Vec3::ZERO, Vec3::new(0.0, 0.0, 0.0));
    }

    #[test]
    fn vec3_add() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_eq!(a + b, Vec3::new(5.0, 7.0, 9.0));
    }

    #[test]
    fn vec3_sub() {
        let a = Vec3::new(5.0, 7.0, 9.0);
        let b = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(a - b, Vec3::new(4.0, 5.0, 6.0));
    }

    #[test]
    fn vec3_mul_scalar() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        assert_eq!(a * 2.0, Vec3::new(2.0, 4.0, 6.0));
    }

    #[test]
    fn vec3_neg() {
        let a = Vec3::new(1.0, -2.0, 3.0);
        assert_eq!(-a, Vec3::new(-1.0, 2.0, -3.0));
    }

    #[test]
    fn vec3_length() {
        let a = Vec3::new(3.0, 4.0, 0.0);
        assert!((a.length() - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn vec3_distance() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(3.0, 4.0, 0.0);
        assert!((a.distance(&b) - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn vec3_proto_roundtrip() {
        let v = Vec3::new(1.5, -2.0, 3.25);
        let mut buf = BytesMut::new();
        v.proto_encode(&mut buf);
        assert_eq!(buf.len(), 12);
        let decoded = Vec3::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded, v);
    }

    // -- Vec2 --

    #[test]
    fn vec2_proto_roundtrip() {
        let v = Vec2::new(1.5, -3.25);
        let mut buf = BytesMut::new();
        v.proto_encode(&mut buf);
        assert_eq!(buf.len(), 8);
        let decoded = Vec2::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded, v);
    }

    // -- Uuid --

    #[test]
    fn uuid_proto_roundtrip() {
        let u = Uuid::new(0x0123456789ABCDEF, 0xFEDCBA9876543210);
        let mut buf = BytesMut::new();
        u.proto_encode(&mut buf);
        assert_eq!(buf.len(), 16);
        let decoded = Uuid::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded, u);
    }

    #[test]
    fn uuid_zero() {
        assert_eq!(
            Uuid::ZERO,
            Uuid {
                most_significant: 0,
                least_significant: 0
            }
        );
    }

    // -- BlockPos --

    #[test]
    fn blockpos_chunk_pos() {
        assert_eq!(BlockPos::new(0, 64, 0).chunk_pos(), ChunkPos::new(0, 0));
        assert_eq!(BlockPos::new(15, 64, 15).chunk_pos(), ChunkPos::new(0, 0));
        assert_eq!(BlockPos::new(16, 64, 16).chunk_pos(), ChunkPos::new(1, 1));
        assert_eq!(BlockPos::new(-1, 64, -1).chunk_pos(), ChunkPos::new(-1, -1));
        assert_eq!(
            BlockPos::new(-16, 64, -16).chunk_pos(),
            ChunkPos::new(-1, -1)
        );
        assert_eq!(
            BlockPos::new(-17, 64, -17).chunk_pos(),
            ChunkPos::new(-2, -2)
        );
    }

    #[test]
    fn blockpos_from_vec3() {
        let pos = BlockPos::from_vec3(&Vec3::new(1.9, 64.5, -0.1));
        assert_eq!(pos, BlockPos::new(1, 64, -1));
    }

    #[test]
    fn blockpos_proto_roundtrip() {
        let bp = BlockPos::new(100, 64, -200);
        let mut buf = BytesMut::new();
        bp.proto_encode(&mut buf);
        let decoded = BlockPos::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded, bp);
    }

    // -- ChunkPos --

    #[test]
    fn chunkpos_block_ranges() {
        let cp = ChunkPos::new(0, 0);
        assert_eq!(cp.block_x_range(), 0..16);
        assert_eq!(cp.block_z_range(), 0..16);

        let cp = ChunkPos::new(1, -1);
        assert_eq!(cp.block_x_range(), 16..32);
        assert_eq!(cp.block_z_range(), -16..0);
    }
}
