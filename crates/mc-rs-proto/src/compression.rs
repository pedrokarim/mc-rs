//! Compression/decompression for Bedrock game packets.

use crate::error::ProtoError;

/// Compression algorithms supported by Bedrock Edition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum CompressionAlgorithm {
    Zlib = 0,
    Snappy = 1,
    None = 0xFFFF,
}

impl CompressionAlgorithm {
    pub fn from_u16(v: u16) -> Result<Self, ProtoError> {
        match v {
            0 => Ok(Self::Zlib),
            1 => Ok(Self::Snappy),
            0xFFFF => Ok(Self::None),
            other => Err(ProtoError::UnknownCompression(other)),
        }
    }

    pub fn from_byte(v: u8) -> Result<Self, ProtoError> {
        match v {
            0x00 => Ok(Self::Zlib),
            0x01 => Ok(Self::Snappy),
            0xFF => Ok(Self::None),
            other => Err(ProtoError::UnknownCompression(other as u16)),
        }
    }

    pub fn to_byte(self) -> u8 {
        match self {
            Self::Zlib => 0x00,
            Self::Snappy => 0x01,
            Self::None => 0xFF,
        }
    }
}

/// Compress data using the specified algorithm.
pub fn compress(
    data: &[u8],
    algorithm: CompressionAlgorithm,
    level: u32,
) -> Result<Vec<u8>, ProtoError> {
    match algorithm {
        CompressionAlgorithm::Zlib => {
            use flate2::write::DeflateEncoder;
            use flate2::Compression;
            use std::io::Write;

            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::new(level));
            encoder
                .write_all(data)
                .map_err(|e| ProtoError::CompressError(e.to_string()))?;
            encoder
                .finish()
                .map_err(|e| ProtoError::CompressError(e.to_string()))
        }
        CompressionAlgorithm::Snappy => {
            let mut encoder = snap::raw::Encoder::new();
            encoder
                .compress_vec(data)
                .map_err(|e| ProtoError::CompressError(e.to_string()))
        }
        CompressionAlgorithm::None => Ok(data.to_vec()),
    }
}

/// Decompress data using a known algorithm.
pub fn decompress_with(
    data: &[u8],
    algorithm: CompressionAlgorithm,
) -> Result<Vec<u8>, ProtoError> {
    match algorithm {
        CompressionAlgorithm::Zlib => {
            use flate2::read::DeflateDecoder;
            use std::io::Read;

            let mut decoder = DeflateDecoder::new(data);
            let mut output = Vec::new();
            decoder
                .read_to_end(&mut output)
                .map_err(|e| ProtoError::DecompressError(e.to_string()))?;
            Ok(output)
        }
        CompressionAlgorithm::Snappy => {
            let mut decoder = snap::raw::Decoder::new();
            decoder
                .decompress_vec(data)
                .map_err(|e| ProtoError::DecompressError(e.to_string()))
        }
        CompressionAlgorithm::None => Ok(data.to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zlib_roundtrip() {
        let data = b"Hello, Bedrock! This is a test of zlib compression.";
        let compressed = compress(data, CompressionAlgorithm::Zlib, 6).unwrap();
        let decompressed = decompress_with(&compressed, CompressionAlgorithm::Zlib).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn snappy_roundtrip() {
        let data = b"Hello, Bedrock! This is a test of snappy compression.";
        let compressed = compress(data, CompressionAlgorithm::Snappy, 0).unwrap();
        let decompressed = decompress_with(&compressed, CompressionAlgorithm::Snappy).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn none_roundtrip() {
        let data = b"no compression";
        let compressed = compress(data, CompressionAlgorithm::None, 0).unwrap();
        assert_eq!(compressed, data);
        let decompressed = decompress_with(&compressed, CompressionAlgorithm::None).unwrap();
        assert_eq!(decompressed, data);
    }

    #[test]
    fn zlib_empty() {
        let compressed = compress(b"", CompressionAlgorithm::Zlib, 6).unwrap();
        let decompressed = decompress_with(&compressed, CompressionAlgorithm::Zlib).unwrap();
        assert!(decompressed.is_empty());
    }

    #[test]
    fn snappy_empty() {
        let compressed = compress(b"", CompressionAlgorithm::Snappy, 0).unwrap();
        let decompressed = decompress_with(&compressed, CompressionAlgorithm::Snappy).unwrap();
        assert!(decompressed.is_empty());
    }

    #[test]
    fn algorithm_from_u16() {
        assert_eq!(
            CompressionAlgorithm::from_u16(0).unwrap(),
            CompressionAlgorithm::Zlib
        );
        assert_eq!(
            CompressionAlgorithm::from_u16(1).unwrap(),
            CompressionAlgorithm::Snappy
        );
        assert_eq!(
            CompressionAlgorithm::from_u16(0xFFFF).unwrap(),
            CompressionAlgorithm::None
        );
        assert!(CompressionAlgorithm::from_u16(42).is_err());
    }

    #[test]
    fn algorithm_from_byte() {
        assert_eq!(
            CompressionAlgorithm::from_byte(0x00).unwrap(),
            CompressionAlgorithm::Zlib
        );
        assert_eq!(
            CompressionAlgorithm::from_byte(0x01).unwrap(),
            CompressionAlgorithm::Snappy
        );
        assert_eq!(
            CompressionAlgorithm::from_byte(0xFF).unwrap(),
            CompressionAlgorithm::None
        );
        assert!(CompressionAlgorithm::from_byte(0x42).is_err());
    }
}
