//! LoginPacket (0x01) — Client → Server.

use bytes::Buf;

use crate::codec::ProtoDecode;
use crate::error::ProtoError;
use crate::types::VarUInt32;

/// Login packet containing the client's protocol version and JWT chain.
///
/// Wire format:
/// ```text
/// int32_be:   protocol_version
/// VarUInt32:  payload_length
///   int32_le: chain_data_length
///   bytes:    chain_data (JSON: {"chain": ["jwt1","jwt2","jwt3"]})
///   int32_le: client_data_length
///   bytes:    client_data (raw JWT string)
/// ```
#[derive(Debug, Clone)]
pub struct LoginPacket {
    /// Protocol version (big-endian i32).
    pub protocol_version: i32,
    /// JWT strings from the identity chain.
    pub chain_data: Vec<String>,
    /// Raw client data JWT (skin, device info, etc.).
    pub client_data_jwt: String,
}

impl ProtoDecode for LoginPacket {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        if buf.remaining() < 4 {
            return Err(ProtoError::BufferTooShort {
                needed: 4,
                remaining: buf.remaining(),
            });
        }
        let protocol_version = buf.get_i32(); // BE

        // Payload length (VarUInt32) — wraps the chain + client data.
        let payload_length = VarUInt32::proto_decode(buf)?.0 as usize;
        if buf.remaining() < payload_length {
            return Err(ProtoError::BufferTooShort {
                needed: payload_length,
                remaining: buf.remaining(),
            });
        }

        // Chain data
        if buf.remaining() < 4 {
            return Err(ProtoError::BufferTooShort {
                needed: 4,
                remaining: buf.remaining(),
            });
        }
        let chain_data_length = buf.get_i32_le() as usize;
        if buf.remaining() < chain_data_length {
            return Err(ProtoError::BufferTooShort {
                needed: chain_data_length,
                remaining: buf.remaining(),
            });
        }
        let chain_bytes = buf.copy_to_bytes(chain_data_length);
        let chain_data = parse_chain_json(&chain_bytes)?;

        // Client data
        if buf.remaining() < 4 {
            return Err(ProtoError::BufferTooShort {
                needed: 4,
                remaining: buf.remaining(),
            });
        }
        let client_data_length = buf.get_i32_le() as usize;
        if buf.remaining() < client_data_length {
            return Err(ProtoError::BufferTooShort {
                needed: client_data_length,
                remaining: buf.remaining(),
            });
        }
        let client_data_bytes = buf.copy_to_bytes(client_data_length);
        let client_data_jwt =
            String::from_utf8(client_data_bytes.to_vec()).map_err(|_| ProtoError::InvalidUtf8)?;

        Ok(Self {
            protocol_version,
            chain_data,
            client_data_jwt,
        })
    }
}

/// Parse the chain JSON: `{"chain": ["jwt1", "jwt2", "jwt3"]}`
fn parse_chain_json(data: &[u8]) -> Result<Vec<String>, ProtoError> {
    let value: serde_json::Value =
        serde_json::from_slice(data).map_err(|e| ProtoError::JsonParse(e.to_string()))?;

    let chain_array = value
        .get("chain")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ProtoError::InvalidLogin("missing 'chain' array in login data".into()))?;

    let mut chain = Vec::with_capacity(chain_array.len());
    for item in chain_array {
        let s = item
            .as_str()
            .ok_or_else(|| ProtoError::InvalidLogin("chain item is not a string".into()))?;
        chain.push(s.to_owned());
    }

    Ok(chain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::{BufMut, BytesMut};

    use crate::codec::ProtoEncode;

    /// Helper: build a LoginPacket's raw bytes for testing.
    fn build_login_bytes(protocol_version: i32, chain_json: &str, client_data: &str) -> BytesMut {
        // Build the inner payload
        let chain_bytes = chain_json.as_bytes();
        let client_bytes = client_data.as_bytes();
        let payload_len = 4 + chain_bytes.len() + 4 + client_bytes.len();

        let mut buf = BytesMut::new();
        buf.put_i32(protocol_version); // BE
        VarUInt32(payload_len as u32).proto_encode(&mut buf);
        buf.put_i32_le(chain_bytes.len() as i32);
        buf.put_slice(chain_bytes);
        buf.put_i32_le(client_bytes.len() as i32);
        buf.put_slice(client_bytes);
        buf
    }

    #[test]
    fn decode_login_packet() {
        let chain_json = r#"{"chain":["jwt1.payload1.sig1","jwt2.payload2.sig2"]}"#;
        let client_data = "client.jwt.data";
        let buf = build_login_bytes(924, chain_json, client_data);

        let pkt = LoginPacket::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(pkt.protocol_version, 924);
        assert_eq!(pkt.chain_data.len(), 2);
        assert_eq!(pkt.chain_data[0], "jwt1.payload1.sig1");
        assert_eq!(pkt.chain_data[1], "jwt2.payload2.sig2");
        assert_eq!(pkt.client_data_jwt, "client.jwt.data");
    }

    #[test]
    fn decode_login_packet_three_chain() {
        let chain_json = r#"{"chain":["jwt1.p.s","jwt2.p.s","jwt3.p.s"]}"#;
        let buf = build_login_bytes(924, chain_json, "cd.p.s");

        let pkt = LoginPacket::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(pkt.chain_data.len(), 3);
    }

    #[test]
    fn decode_login_packet_truncated() {
        let buf = BytesMut::from(&[0x00, 0x00, 0x02][..]);
        assert!(LoginPacket::proto_decode(&mut buf.freeze()).is_err());
    }

    #[test]
    fn decode_login_packet_invalid_chain_json() {
        let buf = build_login_bytes(924, "not json", "cd");
        assert!(LoginPacket::proto_decode(&mut buf.freeze()).is_err());
    }

    #[test]
    fn decode_login_packet_missing_chain_key() {
        let buf = build_login_bytes(924, r#"{"notchain":[]}"#, "cd");
        assert!(LoginPacket::proto_decode(&mut buf.freeze()).is_err());
    }
}
