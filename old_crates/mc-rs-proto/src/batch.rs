//! Game packet batch encoding/decoding (0xFE payload layer).

use std::io::Cursor;

use bytes::{Buf, BufMut, Bytes, BytesMut};

use crate::codec::{ProtoDecode, ProtoEncode};
use crate::compression::{compress, decompress_with, CompressionAlgorithm};
use crate::error::ProtoError;
use crate::types::VarUInt32;

/// Configuration for the game packet batch codec.
#[derive(Debug, Clone)]
pub struct BatchConfig {
    /// Compression algorithm to use for outgoing packets.
    pub compression: CompressionAlgorithm,
    /// Compression level (0-9 for zlib). Ignored for snappy/none.
    pub compression_level: u32,
    /// Packets smaller than this threshold are sent uncompressed.
    pub compression_threshold: usize,
    /// Whether compression has been negotiated (false before NetworkSettings exchange).
    pub compression_enabled: bool,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            compression: CompressionAlgorithm::Zlib,
            compression_level: 7,
            compression_threshold: 256,
            compression_enabled: false,
        }
    }
}

/// Decode a batch payload (0xFE already stripped by RakNet) into individual sub-packets.
///
/// Each returned `Bytes` contains a single sub-packet (VarUInt32 packet_id + packet data).
pub fn decode_batch(data: Bytes, config: &BatchConfig) -> Result<Vec<Bytes>, ProtoError> {
    let decompressed = if config.compression_enabled {
        if data.is_empty() {
            return Err(ProtoError::EmptyBatch);
        }

        let algo_byte = data[0];
        let algorithm = CompressionAlgorithm::from_byte(algo_byte)?;
        let compressed = &data[1..];
        let raw = decompress_with(compressed, algorithm)?;
        Bytes::from(raw)
    } else {
        data
    };

    // Split into individual sub-packets
    let mut cursor = Cursor::new(&decompressed[..]);
    let mut packets = Vec::new();

    while cursor.has_remaining() {
        let len = VarUInt32::proto_decode(&mut cursor)?.0 as usize;
        if cursor.remaining() < len {
            return Err(ProtoError::BufferTooShort {
                needed: len,
                remaining: cursor.remaining(),
            });
        }
        let start = cursor.position() as usize;
        packets.push(decompressed.slice(start..start + len));
        cursor.set_position((start + len) as u64);
    }

    Ok(packets)
}

/// Encode multiple sub-packets into a single batch payload ready for RakNet.
///
/// The caller must prepend 0xFE before handing this to `RakNetServer::send_to`.
/// Each sub-packet should already contain `VarUInt32(packet_id) + data`.
pub fn encode_batch(packets: &[Bytes], config: &BatchConfig) -> Result<Bytes, ProtoError> {
    // Build the uncompressed batch (VarUInt32 length + data for each sub-packet)
    let mut batch = BytesMut::new();
    for packet in packets {
        VarUInt32(packet.len() as u32).proto_encode(&mut batch);
        batch.put_slice(packet);
    }

    if !config.compression_enabled {
        return Ok(batch.freeze());
    }

    // Decide whether to compress based on threshold
    let algorithm = if batch.len() < config.compression_threshold {
        CompressionAlgorithm::None
    } else {
        config.compression
    };

    let compressed = compress(&batch, algorithm, config.compression_level)?;

    // Prepend algorithm byte
    let mut output = BytesMut::with_capacity(1 + compressed.len());
    output.put_u8(algorithm.to_byte());
    output.put_slice(&compressed);

    Ok(output.freeze())
}

/// Convenience: wrap a single sub-packet into a batch.
pub fn encode_single(packet: Bytes, config: &BatchConfig) -> Result<Bytes, ProtoError> {
    encode_batch(&[packet], config)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_packet(id: u32, data: &[u8]) -> Bytes {
        let mut buf = BytesMut::new();
        VarUInt32(id).proto_encode(&mut buf);
        buf.put_slice(data);
        buf.freeze()
    }

    #[test]
    fn batch_single_no_compression() {
        let config = BatchConfig::default(); // compression_enabled = false
        let pkt = make_packet(0x01, b"hello");
        let encoded = encode_batch(std::slice::from_ref(&pkt), &config).unwrap();
        let decoded = decode_batch(encoded, &config).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], pkt);
    }

    #[test]
    fn batch_multiple_no_compression() {
        let config = BatchConfig::default();
        let p1 = make_packet(0x01, b"login");
        let p2 = make_packet(0x8F, b"settings");
        let p3 = make_packet(0x0B, b"startgame");

        let encoded = encode_batch(&[p1.clone(), p2.clone(), p3.clone()], &config).unwrap();
        let decoded = decode_batch(encoded, &config).unwrap();
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0], p1);
        assert_eq!(decoded[1], p2);
        assert_eq!(decoded[2], p3);
    }

    #[test]
    fn batch_single_zlib() {
        let config = BatchConfig {
            compression: CompressionAlgorithm::Zlib,
            compression_level: 6,
            compression_threshold: 0, // Always compress
            compression_enabled: true,
        };
        let pkt = make_packet(0x01, b"hello world from bedrock");
        let encoded = encode_batch(std::slice::from_ref(&pkt), &config).unwrap();
        // First byte should be algorithm indicator
        assert_eq!(encoded[0], 0x00); // Zlib
        let decoded = decode_batch(encoded, &config).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], pkt);
    }

    #[test]
    fn batch_single_snappy() {
        let config = BatchConfig {
            compression: CompressionAlgorithm::Snappy,
            compression_level: 0,
            compression_threshold: 0,
            compression_enabled: true,
        };
        let pkt = make_packet(0x0B, b"startgame data goes here");
        let encoded = encode_batch(std::slice::from_ref(&pkt), &config).unwrap();
        assert_eq!(encoded[0], 0x01); // Snappy
        let decoded = decode_batch(encoded, &config).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], pkt);
    }

    #[test]
    fn batch_below_threshold_not_compressed() {
        let config = BatchConfig {
            compression: CompressionAlgorithm::Zlib,
            compression_level: 6,
            compression_threshold: 9999, // Very high threshold
            compression_enabled: true,
        };
        let pkt = make_packet(0x01, b"small");
        let encoded = encode_batch(std::slice::from_ref(&pkt), &config).unwrap();
        // First byte should be 0xFF (None) since below threshold
        assert_eq!(encoded[0], 0xFF);
        let decoded = decode_batch(encoded, &config).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], pkt);
    }

    #[test]
    fn batch_roundtrip_multiple_zlib() {
        let config = BatchConfig {
            compression: CompressionAlgorithm::Zlib,
            compression_level: 6,
            compression_threshold: 0,
            compression_enabled: true,
        };
        let packets: Vec<Bytes> = (0..10)
            .map(|i| make_packet(i, format!("packet data {i}").as_bytes()))
            .collect();
        let encoded = encode_batch(&packets, &config).unwrap();
        let decoded = decode_batch(encoded, &config).unwrap();
        assert_eq!(decoded.len(), 10);
        for (i, pkt) in decoded.iter().enumerate() {
            assert_eq!(*pkt, packets[i]);
        }
    }

    #[test]
    fn batch_empty_no_compression() {
        let config = BatchConfig::default();
        let encoded = encode_batch(&[], &config).unwrap();
        let decoded = decode_batch(encoded, &config).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn encode_single_convenience() {
        let config = BatchConfig::default();
        let pkt = make_packet(0x01, b"test");
        let encoded = encode_single(pkt.clone(), &config).unwrap();
        let decoded = decode_batch(encoded, &config).unwrap();
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0], pkt);
    }
}
