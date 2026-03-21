use crate::io::{ProtoReader, ProtoWriter};

/// Decode a game packet header: VarUInt32 with packet_id in the low 10 bits.
/// Returns (packet_id, sender_sub_client, target_sub_client).
pub fn decode_packet_header(reader: &mut ProtoReader) -> Result<(u32, u8, u8), PacketHeaderError> {
    let header = reader
        .read_var_u32()
        .map_err(|_| PacketHeaderError::InvalidHeader)?;
    let packet_id = header & 0x3FF;
    let sender_sub = ((header >> 10) & 0x03) as u8;
    let target_sub = ((header >> 12) & 0x03) as u8;
    Ok((packet_id, sender_sub, target_sub))
}

/// Encode a game packet header. Usually sender_sub and target_sub are 0.
pub fn encode_packet_header(writer: &mut ProtoWriter, packet_id: u32) {
    writer.write_var_u32(packet_id & 0x3FF);
}

/// Encode a full packet: header + payload.
pub fn encode_packet(packet_id: u32, payload: &[u8]) -> Vec<u8> {
    let mut writer = ProtoWriter::with_capacity(5 + payload.len());
    encode_packet_header(&mut writer, packet_id);
    writer.write_raw(payload);
    writer.into_bytes()
}

#[derive(Debug)]
pub enum PacketHeaderError {
    InvalidHeader,
}

impl std::fmt::Display for PacketHeaderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid packet header")
    }
}

impl std::error::Error for PacketHeaderError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_roundtrip() {
        let mut writer = ProtoWriter::new();
        encode_packet_header(&mut writer, 0x01); // LoginPacket
        let mut reader = ProtoReader::new(writer.as_bytes());
        let (id, sender, target) = decode_packet_header(&mut reader).unwrap();
        assert_eq!(id, 0x01);
        assert_eq!(sender, 0);
        assert_eq!(target, 0);
    }

    #[test]
    fn test_large_packet_id() {
        let mut writer = ProtoWriter::new();
        encode_packet_header(&mut writer, 0x161); // ItemRegistry = 353
        let mut reader = ProtoReader::new(writer.as_bytes());
        let (id, _, _) = decode_packet_header(&mut reader).unwrap();
        assert_eq!(id, 0x161);
    }

    #[test]
    fn test_encode_packet() {
        let pkt = encode_packet(0x02, &[0x00, 0x00, 0x00, 0x03]);
        let mut reader = ProtoReader::new(&pkt);
        let (id, _, _) = decode_packet_header(&mut reader).unwrap();
        assert_eq!(id, 0x02);
        assert_eq!(reader.remaining(), 4);
    }
}
