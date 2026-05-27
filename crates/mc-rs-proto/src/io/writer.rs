/// Binary writer for Minecraft Bedrock protocol.
/// All multi-byte integers are little-endian unless specified.
pub struct ProtoWriter {
    buf: Vec<u8>,
}

impl ProtoWriter {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            buf: Vec::with_capacity(cap),
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }

    // ── Fixed-size types ──

    pub fn write_u8(&mut self, v: u8) {
        self.buf.push(v);
    }

    pub fn write_i8(&mut self, v: i8) {
        self.buf.push(v as u8);
    }

    pub fn write_bool(&mut self, v: bool) {
        self.buf.push(v as u8);
    }

    pub fn write_u16_le(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_i16_le(&mut self, v: i16) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_u16_be(&mut self, v: u16) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub fn write_u24_le(&mut self, v: u32) {
        let bytes = v.to_le_bytes();
        self.buf.extend_from_slice(&bytes[..3]);
    }

    pub fn write_u32_le(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_i32_le(&mut self, v: i32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_u32_be(&mut self, v: u32) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub fn write_i32_be(&mut self, v: i32) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub fn write_u64_le(&mut self, v: u64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_i64_le(&mut self, v: i64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_i64_be(&mut self, v: i64) {
        self.buf.extend_from_slice(&v.to_be_bytes());
    }

    pub fn write_f32_le(&mut self, v: f32) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    pub fn write_f64_le(&mut self, v: f64) {
        self.buf.extend_from_slice(&v.to_le_bytes());
    }

    // ── Variable-length integers ──

    /// Unsigned VarInt (LEB128), max 5 bytes
    pub fn write_var_u32(&mut self, mut v: u32) {
        loop {
            let mut byte = (v & 0x7F) as u8;
            v >>= 7;
            if v != 0 {
                byte |= 0x80;
            }
            self.buf.push(byte);
            if v == 0 {
                break;
            }
        }
    }

    /// Signed VarInt (zigzag + LEB128), max 5 bytes
    pub fn write_var_i32(&mut self, v: i32) {
        self.write_var_u32(zigzag_encode_32(v));
    }

    /// Unsigned VarLong (LEB128), max 10 bytes
    pub fn write_var_u64(&mut self, mut v: u64) {
        loop {
            let mut byte = (v & 0x7F) as u8;
            v >>= 7;
            if v != 0 {
                byte |= 0x80;
            }
            self.buf.push(byte);
            if v == 0 {
                break;
            }
        }
    }

    /// Signed VarLong (zigzag + LEB128), max 10 bytes
    pub fn write_var_i64(&mut self, v: i64) {
        self.write_var_u64(zigzag_encode_64(v));
    }

    // ── Composite types ──

    /// String: VarUInt32 length + UTF-8 bytes
    pub fn write_string(&mut self, v: &str) {
        self.write_var_u32(v.len() as u32);
        self.buf.extend_from_slice(v.as_bytes());
    }

    /// Byte array: VarUInt32 length + raw bytes
    pub fn write_byte_array(&mut self, v: &[u8]) {
        self.write_var_u32(v.len() as u32);
        self.buf.extend_from_slice(v);
    }

    /// Write raw bytes (no length prefix)
    pub fn write_raw(&mut self, v: &[u8]) {
        self.buf.extend_from_slice(v);
    }
}

impl Default for ProtoWriter {
    fn default() -> Self {
        Self::new()
    }
}

// ── Zigzag encoding helpers ──

#[inline]
fn zigzag_encode_32(v: i32) -> u32 {
    ((v << 1) ^ (v >> 31)) as u32
}

#[inline]
fn zigzag_encode_64(v: i64) -> u64 {
    ((v << 1) ^ (v >> 63)) as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::reader::ProtoReader;

    #[test]
    fn test_u8_roundtrip() {
        let mut w = ProtoWriter::new();
        w.write_u8(0x42);
        let mut r = ProtoReader::new(w.as_bytes());
        assert_eq!(r.read_u8().unwrap(), 0x42);
    }

    #[test]
    fn test_u16_le_roundtrip() {
        let mut w = ProtoWriter::new();
        w.write_u16_le(0x1234);
        let mut r = ProtoReader::new(w.as_bytes());
        assert_eq!(r.read_u16_le().unwrap(), 0x1234);
    }

    #[test]
    fn test_u24_le_roundtrip() {
        let mut w = ProtoWriter::new();
        w.write_u24_le(0x030201);
        let mut r = ProtoReader::new(w.as_bytes());
        assert_eq!(r.read_u24_le().unwrap(), 0x030201);
    }

    #[test]
    fn test_i32_be_roundtrip() {
        let mut w = ProtoWriter::new();
        w.write_i32_be(924);
        let mut r = ProtoReader::new(w.as_bytes());
        assert_eq!(r.read_i32_be().unwrap(), 924);
    }

    #[test]
    fn test_var_u32_roundtrip() {
        for val in [0u32, 1, 127, 128, 255, 300, 16383, 65535, u32::MAX] {
            let mut w = ProtoWriter::new();
            w.write_var_u32(val);
            let mut r = ProtoReader::new(w.as_bytes());
            assert_eq!(r.read_var_u32().unwrap(), val, "failed for {val}");
        }
    }

    #[test]
    fn test_var_i32_roundtrip() {
        for val in [0i32, 1, -1, 2, -2, 127, -128, i32::MAX, i32::MIN] {
            let mut w = ProtoWriter::new();
            w.write_var_i32(val);
            let mut r = ProtoReader::new(w.as_bytes());
            assert_eq!(r.read_var_i32().unwrap(), val, "failed for {val}");
        }
    }

    #[test]
    fn test_var_u64_roundtrip() {
        for val in [0u64, 1, 255, 65535, u32::MAX as u64, u64::MAX] {
            let mut w = ProtoWriter::new();
            w.write_var_u64(val);
            let mut r = ProtoReader::new(w.as_bytes());
            assert_eq!(r.read_var_u64().unwrap(), val, "failed for {val}");
        }
    }

    #[test]
    fn test_var_i64_roundtrip() {
        for val in [0i64, 1, -1, i64::MAX, i64::MIN] {
            let mut w = ProtoWriter::new();
            w.write_var_i64(val);
            let mut r = ProtoReader::new(w.as_bytes());
            assert_eq!(r.read_var_i64().unwrap(), val, "failed for {val}");
        }
    }

    #[test]
    fn test_string_roundtrip() {
        let mut w = ProtoWriter::new();
        w.write_string("hello world");
        let mut r = ProtoReader::new(w.as_bytes());
        assert_eq!(r.read_string().unwrap(), "hello world");
    }

    #[test]
    fn test_byte_array_roundtrip() {
        let data = vec![1, 2, 3, 4, 5];
        let mut w = ProtoWriter::new();
        w.write_byte_array(&data);
        let mut r = ProtoReader::new(w.as_bytes());
        assert_eq!(r.read_byte_array().unwrap(), data);
    }

    #[test]
    fn test_f32_le_roundtrip() {
        let mut w = ProtoWriter::new();
        let probe: f32 = 1.5;
        w.write_f32_le(probe);
        let mut r = ProtoReader::new(w.as_bytes());
        let v = r.read_f32_le().unwrap();
        assert!((v - probe).abs() < 0.001);
    }

    #[test]
    fn test_zigzag_encode() {
        assert_eq!(zigzag_encode_32(0), 0);
        assert_eq!(zigzag_encode_32(-1), 1);
        assert_eq!(zigzag_encode_32(1), 2);
        assert_eq!(zigzag_encode_32(-2), 3);
        assert_eq!(zigzag_encode_32(2), 4);
    }

    #[test]
    fn test_mixed_writes() {
        let mut w = ProtoWriter::new();
        w.write_u8(0xFF);
        w.write_i32_be(924);
        w.write_var_u32(300);
        w.write_string("test");
        w.write_bool(true);

        let mut r = ProtoReader::new(w.as_bytes());
        assert_eq!(r.read_u8().unwrap(), 0xFF);
        assert_eq!(r.read_i32_be().unwrap(), 924);
        assert_eq!(r.read_var_u32().unwrap(), 300);
        assert_eq!(r.read_string().unwrap(), "test");
        assert!(r.read_bool().unwrap());
        assert_eq!(r.remaining(), 0);
    }
}
