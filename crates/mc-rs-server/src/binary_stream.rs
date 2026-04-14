//! Binary stream utility similar to PMMP BinaryStream.

#[derive(Debug, Clone)]
pub struct BinaryStream {
    pub buffer: Vec<u8>,
    pub offset: usize,
}

impl BinaryStream {
    pub fn new() -> Self {
        Self { buffer: Vec::new(), offset: 0 }
    }

    pub fn from_bytes(bytes: Vec<u8>) -> Self {
        Self { buffer: bytes, offset: 0 }
    }

    pub fn eof(&self) -> bool {
        self.offset >= self.buffer.len()
    }

    pub fn read_byte(&mut self) -> Option<u8> {
        if self.offset >= self.buffer.len() {
            return None;
        }
        let b = self.buffer[self.offset];
        self.offset += 1;
        Some(b)
    }

    pub fn write_byte(&mut self, b: u8) {
        self.buffer.push(b);
    }

    pub fn read_short_be(&mut self) -> Option<i16> {
        let h = self.read_byte()? as i16;
        let l = self.read_byte()? as i16;
        Some(h << 8 | l)
    }

    pub fn write_short_be(&mut self, v: i16) {
        self.write_byte(((v >> 8) & 0xff) as u8);
        self.write_byte((v & 0xff) as u8);
    }

    pub fn read_int_be(&mut self) -> Option<i32> {
        let mut v: i32 = 0;
        for _ in 0..4 {
            v = (v << 8) | (self.read_byte()? as i32);
        }
        Some(v)
    }

    pub fn write_int_be(&mut self, v: i32) {
        self.write_byte(((v >> 24) & 0xff) as u8);
        self.write_byte(((v >> 16) & 0xff) as u8);
        self.write_byte(((v >> 8) & 0xff) as u8);
        self.write_byte((v & 0xff) as u8);
    }

    pub fn read_var_int(&mut self) -> Option<i32> {
        let mut value: i32 = 0;
        let mut shift = 0;
        loop {
            if shift > 28 {
                return None;
            }
            let b = self.read_byte()?;
            value |= ((b & 0x7f) as i32) << shift;
            if b & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        Some(value)
    }

    pub fn write_var_int(&mut self, mut v: i32) {
        let u = v as u32;
        v = u as i32;
        let mut v = v as u32;
        loop {
            if v >= 0x80 {
                self.write_byte((v as u8 | 0x80) as u8);
                v >>= 7;
            } else {
                self.write_byte(v as u8);
                break;
            }
        }
    }
}

impl Default for BinaryStream {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_roundtrip() {
        let mut s = BinaryStream::new();
        s.write_byte(42);
        s.offset = 0;
        assert_eq!(s.read_byte(), Some(42));
    }

    #[test]
    fn short_be_roundtrip() {
        let mut s = BinaryStream::new();
        s.write_short_be(-1234);
        s.offset = 0;
        assert_eq!(s.read_short_be(), Some(-1234));
    }
}
