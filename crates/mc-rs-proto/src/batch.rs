use flate2::read::DeflateDecoder;
use flate2::write::DeflateEncoder;
use flate2::Compression;
use std::io::{Read, Write};

use crate::io::{ProtoReader, ProtoWriter};

/// Compression algorithm IDs used in the batch header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CompressionAlgorithm {
    Zlib = 0x00,
    Snappy = 0x01,
    None = 0xFF,
}

impl CompressionAlgorithm {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x00 => Some(Self::Zlib),
            0x01 => Some(Self::Snappy),
            0xFF => Some(Self::None),
            _ => None,
        }
    }
}

/// Decode a batch payload (after 0xFE + algo byte) into individual packet buffers.
/// Each packet in the batch: VarUInt32(length) + packet_data
pub fn decode_batch(raw: &[u8], algo: CompressionAlgorithm) -> Result<Vec<Vec<u8>>, BatchError> {
    // Decompress
    let decompressed = match algo {
        CompressionAlgorithm::Zlib => {
            let mut decoder = DeflateDecoder::new(raw);
            let mut out = Vec::new();
            decoder
                .read_to_end(&mut out)
                .map_err(|e| BatchError::DecompressError(e.to_string()))?;
            out
        }
        CompressionAlgorithm::Snappy => {
            snap::raw::Decoder::new()
                .decompress_vec(raw)
                .map_err(|e| BatchError::DecompressError(e.to_string()))?
        }
        CompressionAlgorithm::None => raw.to_vec(),
    };

    // Parse individual packets
    let mut reader = ProtoReader::new(&decompressed);
    let mut packets = Vec::new();

    while reader.remaining() > 0 {
        let len = reader.read_var_u32().map_err(|_| BatchError::InvalidLength)? as usize;
        if len == 0 || reader.remaining() < len {
            break;
        }
        let data = reader.read_raw(len).map_err(|_| BatchError::InvalidLength)?;
        packets.push(data);
    }

    Ok(packets)
}

/// Encode multiple packet buffers into a compressed batch payload.
/// Returns the bytes AFTER 0xFE (i.e., algo_byte + compressed_data).
pub fn encode_batch(
    packets: &[Vec<u8>],
    algo: CompressionAlgorithm,
    compression_level: u32,
) -> Vec<u8> {
    // Build uncompressed payload: concat of [VarUInt32(len) + data]
    let mut payload = ProtoWriter::with_capacity(1024);
    for pkt in packets {
        payload.write_var_u32(pkt.len() as u32);
        payload.write_raw(pkt);
    }

    let raw = payload.into_bytes();

    // Compress
    let compressed = match algo {
        CompressionAlgorithm::Zlib => {
            let level = Compression::new(compression_level);
            let mut encoder = DeflateEncoder::new(Vec::new(), level);
            encoder.write_all(&raw).unwrap();
            encoder.finish().unwrap()
        }
        CompressionAlgorithm::Snappy => snap::raw::Encoder::new()
            .compress_vec(&raw)
            .unwrap_or_else(|_| raw.clone()),
        CompressionAlgorithm::None => raw,
    };

    // Prepend algorithm byte
    let mut result = Vec::with_capacity(1 + compressed.len());
    result.push(algo as u8);
    result.extend_from_slice(&compressed);
    result
}

/// Wrap batch payload with the 0xFE header for sending over RakNet.
pub fn wrap_batch(batch_payload: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(1 + batch_payload.len());
    result.push(0xFE);
    result.extend_from_slice(batch_payload);
    result
}

#[derive(Debug)]
pub enum BatchError {
    DecompressError(String),
    InvalidLength,
}

impl std::fmt::Display for BatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DecompressError(e) => write!(f, "decompression error: {}", e),
            Self::InvalidLength => write!(f, "invalid packet length in batch"),
        }
    }
}

impl std::error::Error for BatchError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_roundtrip_zlib() {
        let packets = vec![vec![0x01, 0x02, 0x03], vec![0x04, 0x05]];
        let encoded = encode_batch(&packets, CompressionAlgorithm::Zlib, 6);
        assert_eq!(encoded[0], 0x00); // zlib algo byte

        let decoded = decode_batch(&encoded[1..], CompressionAlgorithm::Zlib).unwrap();
        assert_eq!(decoded, packets);
    }

    #[test]
    fn test_batch_roundtrip_none() {
        let packets = vec![vec![0xAA, 0xBB]];
        let encoded = encode_batch(&packets, CompressionAlgorithm::None, 0);
        assert_eq!(encoded[0], 0xFF); // none algo byte

        let decoded = decode_batch(&encoded[1..], CompressionAlgorithm::None).unwrap();
        assert_eq!(decoded, packets);
    }

    #[test]
    fn test_batch_roundtrip_snappy() {
        let packets = vec![vec![0x01; 100], vec![0x02; 50]];
        let encoded = encode_batch(&packets, CompressionAlgorithm::Snappy, 0);
        assert_eq!(encoded[0], 0x01); // snappy algo byte

        let decoded = decode_batch(&encoded[1..], CompressionAlgorithm::Snappy).unwrap();
        assert_eq!(decoded, packets);
    }

    #[test]
    fn test_wrap_batch() {
        let payload = vec![0x00, 0x01, 0x02];
        let wrapped = wrap_batch(&payload);
        assert_eq!(wrapped[0], 0xFE);
        assert_eq!(&wrapped[1..], &payload);
    }
}
