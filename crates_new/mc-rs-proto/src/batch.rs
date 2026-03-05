use crate::codec::{read_unsigned_varint32, write_unsigned_varint32};
use bytes::{Buf, BufMut, Bytes, BytesMut};
use std::io::Cursor;

pub fn compress_snappy(data: &[u8]) -> Result<Vec<u8>, String> {
    snap::raw::Encoder::new()
        .compress_vec(data)
        .map_err(|e| e.to_string())
}

pub fn decompress_snappy(data: &[u8]) -> Result<Vec<u8>, String> {
    snap::raw::Decoder::new()
        .decompress_vec(data)
        .map_err(|e| e.to_string())
}

pub fn compress_zlib(data: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Write;
    let mut encoder = flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(data).map_err(|e| e.to_string())?;
    encoder.finish().map_err(|e| e.to_string())
}

pub fn decompress_zlib(data: &[u8]) -> Result<Vec<u8>, String> {
    use std::io::Read;
    let mut decoder = flate2::read::DeflateDecoder::new(data);
    let mut output = Vec::new();
    decoder
        .read_to_end(&mut output)
        .map_err(|e| e.to_string())?;
    Ok(output)
}

pub fn decode_batch(data: &[u8], compression_enabled: bool) -> Result<Vec<Bytes>, String> {
    let decompressed = if compression_enabled {
        if data.is_empty() {
            return Err("empty batch".into());
        }
        match data[0] {
            0x00 => Bytes::from(decompress_zlib(&data[1..])?),
            0x01 => Bytes::from(decompress_snappy(&data[1..])?),
            0xFF => Bytes::copy_from_slice(&data[1..]),
            other => return Err(format!("unknown compression algo: 0x{other:02X}")),
        }
    } else {
        Bytes::copy_from_slice(data)
    };

    let mut cursor = Cursor::new(&decompressed[..]);
    let mut packets = Vec::new();
    while cursor.has_remaining() {
        let len = read_unsigned_varint32(&mut cursor).map_err(|e| e.to_string())? as usize;
        if cursor.remaining() < len {
            return Err(format!(
                "packet len {len} > remaining {}",
                cursor.remaining()
            ));
        }
        let start = cursor.position() as usize;
        packets.push(decompressed.slice(start..start + len));
        cursor.set_position((start + len) as u64);
    }
    Ok(packets)
}

/// Encode a batch using Zlib (algorithm=0), matching PocketMine default.
pub fn encode_batch(framed_packets: &[BytesMut], compression_enabled: bool) -> Bytes {
    let mut batch = BytesMut::new();
    for pkt in framed_packets {
        batch.extend_from_slice(pkt);
    }

    if !compression_enabled {
        return batch.freeze();
    }

    match compress_zlib(&batch) {
        Ok(compressed) => {
            let mut output = BytesMut::with_capacity(1 + compressed.len());
            output.put_u8(0x00); // Zlib
            output.extend_from_slice(&compressed);
            output.freeze()
        }
        Err(_) => {
            let mut output = BytesMut::with_capacity(1 + batch.len());
            output.put_u8(0xFF); // None (fallback)
            output.extend_from_slice(&batch);
            output.freeze()
        }
    }
}
