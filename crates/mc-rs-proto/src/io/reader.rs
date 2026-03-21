use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtoReadError {
    #[error("unexpected end of buffer: need {needed} bytes, have {available}")]
    BufferUnderflow { needed: usize, available: usize },
    #[error("VarInt too large (exceeded 5 bytes)")]
    VarIntTooLarge,
    #[error("VarLong too large (exceeded 10 bytes)")]
    VarLongTooLarge,
    #[error("invalid UTF-8 string: {0}")]
    InvalidUtf8(#[from] std::string::FromUtf8Error),
}

/// Binary reader for Minecraft Bedrock protocol.
/// All multi-byte integers are little-endian unless specified.
pub struct ProtoReader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> ProtoReader<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len() - self.pos
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    pub fn remaining_bytes(&self) -> &'a [u8] {
        &self.buf[self.pos..]
    }

    fn ensure(&self, n: usize) -> Result<(), ProtoReadError> {
        if self.remaining() < n {
            Err(ProtoReadError::BufferUnderflow {
                needed: n,
                available: self.remaining(),
            })
        } else {
            Ok(())
        }
    }

    fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], ProtoReadError> {
        self.ensure(n)?;
        let slice = &self.buf[self.pos..self.pos + n];
        self.pos += n;
        Ok(slice)
    }

    // ── Fixed-size types ──

    pub fn read_u8(&mut self) -> Result<u8, ProtoReadError> {
        let b = self.read_bytes(1)?;
        Ok(b[0])
    }

    pub fn read_i8(&mut self) -> Result<i8, ProtoReadError> {
        Ok(self.read_u8()? as i8)
    }

    pub fn read_bool(&mut self) -> Result<bool, ProtoReadError> {
        Ok(self.read_u8()? != 0)
    }

    pub fn read_u16_le(&mut self) -> Result<u16, ProtoReadError> {
        let b = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn read_i16_le(&mut self) -> Result<i16, ProtoReadError> {
        let b = self.read_bytes(2)?;
        Ok(i16::from_le_bytes([b[0], b[1]]))
    }

    pub fn read_u16_be(&mut self) -> Result<u16, ProtoReadError> {
        let b = self.read_bytes(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    pub fn read_u24_le(&mut self) -> Result<u32, ProtoReadError> {
        let b = self.read_bytes(3)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], 0]))
    }

    pub fn read_u32_le(&mut self) -> Result<u32, ProtoReadError> {
        let b = self.read_bytes(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_i32_le(&mut self) -> Result<i32, ProtoReadError> {
        let b = self.read_bytes(4)?;
        Ok(i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_u32_be(&mut self) -> Result<u32, ProtoReadError> {
        let b = self.read_bytes(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_i32_be(&mut self) -> Result<i32, ProtoReadError> {
        let b = self.read_bytes(4)?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_u64_le(&mut self) -> Result<u64, ProtoReadError> {
        let b = self.read_bytes(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    pub fn read_i64_le(&mut self) -> Result<i64, ProtoReadError> {
        let b = self.read_bytes(8)?;
        Ok(i64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    pub fn read_i64_be(&mut self) -> Result<i64, ProtoReadError> {
        let b = self.read_bytes(8)?;
        Ok(i64::from_be_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    pub fn read_f32_le(&mut self) -> Result<f32, ProtoReadError> {
        let b = self.read_bytes(4)?;
        Ok(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    pub fn read_f64_le(&mut self) -> Result<f64, ProtoReadError> {
        let b = self.read_bytes(8)?;
        Ok(f64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    // ── Variable-length integers ──

    /// Unsigned VarInt (LEB128), max 5 bytes → u32
    pub fn read_var_u32(&mut self) -> Result<u32, ProtoReadError> {
        let mut value: u32 = 0;
        for i in 0..5u32 {
            let byte = self.read_u8()?;
            value |= ((byte & 0x7F) as u32) << (i * 7);
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(ProtoReadError::VarIntTooLarge)
    }

    /// Signed VarInt (zigzag + LEB128), max 5 bytes → i32
    pub fn read_var_i32(&mut self) -> Result<i32, ProtoReadError> {
        let raw = self.read_var_u32()?;
        Ok(zigzag_decode_32(raw))
    }

    /// Unsigned VarLong (LEB128), max 10 bytes → u64
    pub fn read_var_u64(&mut self) -> Result<u64, ProtoReadError> {
        let mut value: u64 = 0;
        for i in 0..10u32 {
            let byte = self.read_u8()?;
            value |= ((byte & 0x7F) as u64) << (i * 7);
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(ProtoReadError::VarLongTooLarge)
    }

    /// Signed VarLong (zigzag + LEB128), max 10 bytes → i64
    pub fn read_var_i64(&mut self) -> Result<i64, ProtoReadError> {
        let raw = self.read_var_u64()?;
        Ok(zigzag_decode_64(raw))
    }

    // ── Composite types ──

    /// String: VarUInt32 length + UTF-8 bytes
    pub fn read_string(&mut self) -> Result<String, ProtoReadError> {
        let len = self.read_var_u32()? as usize;
        let bytes = self.read_bytes(len)?;
        Ok(String::from_utf8(bytes.to_vec())?)
    }

    /// Byte array: VarUInt32 length + raw bytes
    pub fn read_byte_array(&mut self) -> Result<Vec<u8>, ProtoReadError> {
        let len = self.read_var_u32()? as usize;
        let bytes = self.read_bytes(len)?;
        Ok(bytes.to_vec())
    }

    /// Read exactly `n` raw bytes
    pub fn read_raw(&mut self, n: usize) -> Result<Vec<u8>, ProtoReadError> {
        let bytes = self.read_bytes(n)?;
        Ok(bytes.to_vec())
    }

    /// Read all remaining bytes
    pub fn read_remaining(&mut self) -> Vec<u8> {
        let rest = self.buf[self.pos..].to_vec();
        self.pos = self.buf.len();
        rest
    }
}

// ── Zigzag encoding helpers ──

#[inline]
fn zigzag_decode_32(v: u32) -> i32 {
    ((v >> 1) as i32) ^ (-((v & 1) as i32))
}

#[inline]
fn zigzag_decode_64(v: u64) -> i64 {
    ((v >> 1) as i64) ^ (-((v & 1) as i64))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u8() {
        let buf = [0x42];
        let mut r = ProtoReader::new(&buf);
        assert_eq!(r.read_u8().unwrap(), 0x42);
        assert_eq!(r.remaining(), 0);
    }

    #[test]
    fn test_u16_le() {
        let buf = [0x01, 0x02];
        let mut r = ProtoReader::new(&buf);
        assert_eq!(r.read_u16_le().unwrap(), 0x0201);
    }

    #[test]
    fn test_u24_le() {
        let buf = [0x01, 0x02, 0x03];
        let mut r = ProtoReader::new(&buf);
        assert_eq!(r.read_u24_le().unwrap(), 0x030201);
    }

    #[test]
    fn test_i32_be() {
        let buf = 924i32.to_be_bytes();
        let mut r = ProtoReader::new(&buf);
        assert_eq!(r.read_i32_be().unwrap(), 924);
    }

    #[test]
    fn test_var_u32() {
        // 300 = 0b100101100 → [0xAC, 0x02]
        let buf = [0xAC, 0x02];
        let mut r = ProtoReader::new(&buf);
        assert_eq!(r.read_var_u32().unwrap(), 300);
    }

    #[test]
    fn test_var_u32_single_byte() {
        let buf = [0x01];
        let mut r = ProtoReader::new(&buf);
        assert_eq!(r.read_var_u32().unwrap(), 1);
    }

    #[test]
    fn test_var_i32_positive() {
        // zigzag(1) = 2 → [0x02]
        let buf = [0x02];
        let mut r = ProtoReader::new(&buf);
        assert_eq!(r.read_var_i32().unwrap(), 1);
    }

    #[test]
    fn test_var_i32_negative() {
        // zigzag(-1) = 1 → [0x01]
        let buf = [0x01];
        let mut r = ProtoReader::new(&buf);
        assert_eq!(r.read_var_i32().unwrap(), -1);
    }

    #[test]
    fn test_var_i32_negative_two() {
        // zigzag(-2) = 3 → [0x03]
        let buf = [0x03];
        let mut r = ProtoReader::new(&buf);
        assert_eq!(r.read_var_i32().unwrap(), -2);
    }

    #[test]
    fn test_string() {
        let mut buf = vec![0x05]; // length = 5
        buf.extend_from_slice(b"hello");
        let mut r = ProtoReader::new(&buf);
        assert_eq!(r.read_string().unwrap(), "hello");
    }

    #[test]
    fn test_buffer_underflow() {
        let buf = [0x01];
        let mut r = ProtoReader::new(&buf);
        assert!(r.read_u16_le().is_err());
    }

    #[test]
    fn test_zigzag_decode() {
        assert_eq!(zigzag_decode_32(0), 0);
        assert_eq!(zigzag_decode_32(1), -1);
        assert_eq!(zigzag_decode_32(2), 1);
        assert_eq!(zigzag_decode_32(3), -2);
        assert_eq!(zigzag_decode_32(4), 2);
    }
}
