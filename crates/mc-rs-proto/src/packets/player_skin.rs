//! PlayerSkin (0x5D) — Bidirectional.
//!
//! Sent by the client when the player changes their skin. The server
//! updates the stored skin data and broadcasts the packet to all other
//! connected players.

use bytes::{Buf, BufMut};

use crate::codec::{read_string, write_string, ProtoDecode, ProtoEncode};
use crate::error::ProtoError;
use crate::jwt::{ClientData, SkinImage};
use crate::types::{Uuid, VarUInt32};

use super::player_list::encode_skin_data;

/// The PlayerSkin packet.
pub struct PlayerSkin {
    pub uuid: Uuid,
    pub skin_data: ClientData,
    pub new_skin_name: String,
    pub old_skin_name: String,
}

fn decode_skin_image(buf: &mut impl Buf) -> Result<SkinImage, ProtoError> {
    if buf.remaining() < 8 {
        return Err(ProtoError::BufferTooShort {
            needed: 8,
            remaining: buf.remaining(),
        });
    }
    let width = buf.get_i32_le() as u32;
    let height = buf.get_i32_le() as u32;
    let len = VarUInt32::proto_decode(buf)?.0 as usize;
    if buf.remaining() < len {
        return Err(ProtoError::BufferTooShort {
            needed: len,
            remaining: buf.remaining(),
        });
    }
    let mut data = vec![0u8; len];
    buf.copy_to_slice(&mut data);
    Ok(SkinImage {
        width,
        height,
        data,
    })
}

fn decode_skin_data(buf: &mut impl Buf) -> Result<ClientData, ProtoError> {
    let skin_id = read_string(buf)?;
    let play_fab_id = read_string(buf)?;
    let skin_resource_patch = read_string(buf)?;
    let skin_image = decode_skin_image(buf)?;

    // Animations
    if buf.remaining() < 4 {
        return Err(ProtoError::BufferTooShort {
            needed: 4,
            remaining: buf.remaining(),
        });
    }
    let anim_count = buf.get_i32_le() as usize;
    for _ in 0..anim_count {
        let _image = decode_skin_image(buf)?;
        if buf.remaining() < 12 {
            return Err(ProtoError::BufferTooShort {
                needed: 12,
                remaining: buf.remaining(),
            });
        }
        buf.advance(12); // animation_type(u32_le) + frames(f32_le) + expression_type(u32_le)
    }

    let cape_image = decode_skin_image(buf)?;
    let skin_geometry_data = read_string(buf)?;
    let _geometry_engine_version = read_string(buf)?;
    let _animation_data = read_string(buf)?;
    let cape_id = read_string(buf)?;
    let _full_skin_id = read_string(buf)?;
    let arm_size = read_string(buf)?;
    let skin_color = read_string(buf)?;

    // Persona pieces
    if buf.remaining() < 4 {
        return Err(ProtoError::BufferTooShort {
            needed: 4,
            remaining: buf.remaining(),
        });
    }
    let piece_count = buf.get_i32_le() as usize;
    for _ in 0..piece_count {
        let _piece_id = read_string(buf)?;
        let _piece_type = read_string(buf)?;
        let _pack_id = read_string(buf)?;
        if buf.remaining() < 1 {
            return Err(ProtoError::BufferTooShort {
                needed: 1,
                remaining: buf.remaining(),
            });
        }
        buf.advance(1); // is_default_piece (bool)
        let _product_id = read_string(buf)?;
    }

    // Piece tint colors
    if buf.remaining() < 4 {
        return Err(ProtoError::BufferTooShort {
            needed: 4,
            remaining: buf.remaining(),
        });
    }
    let tint_count = buf.get_i32_le() as usize;
    for _ in 0..tint_count {
        let _piece_type = read_string(buf)?;
        if buf.remaining() < 4 {
            return Err(ProtoError::BufferTooShort {
                needed: 4,
                remaining: buf.remaining(),
            });
        }
        let color_count = buf.get_i32_le() as usize;
        for _ in 0..color_count {
            let _color = read_string(buf)?;
        }
    }

    // Trailing booleans
    if buf.remaining() < 5 {
        return Err(ProtoError::BufferTooShort {
            needed: 5,
            remaining: buf.remaining(),
        });
    }
    let _is_premium = buf.get_u8() != 0;
    let persona_skin = buf.get_u8() != 0;
    let _is_persona_cape_on_classic = buf.get_u8() != 0;
    let _is_primary_user = buf.get_u8() != 0;
    let _override_appearance = buf.get_u8() != 0;

    Ok(ClientData {
        skin_id,
        skin_image,
        cape_id,
        cape_image,
        skin_resource_patch,
        skin_geometry_data,
        skin_color,
        arm_size,
        persona_skin,
        device_id: String::new(),
        device_os: 0,
        play_fab_id,
    })
}

impl ProtoDecode for PlayerSkin {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        let uuid = Uuid::proto_decode(buf)?;
        let skin_data = decode_skin_data(buf)?;
        let new_skin_name = read_string(buf)?;
        let old_skin_name = read_string(buf)?;
        // is_verified (bool) — consume if present
        if buf.remaining() >= 1 {
            buf.advance(1);
        }
        Ok(Self {
            uuid,
            skin_data,
            new_skin_name,
            old_skin_name,
        })
    }
}

impl ProtoEncode for PlayerSkin {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        self.uuid.proto_encode(buf);
        encode_skin_data(buf, &self.skin_data);
        // Skin image is not enough — we need the full_skin_id etc. that
        // encode_skin_data already writes. Now add the trailing fields:
        write_string(buf, &self.new_skin_name);
        write_string(buf, &self.old_skin_name);
        buf.put_u8(1); // is_verified = true
    }
}

/// Encode a PlayerSkin packet for broadcast using the given skin image
/// helper from `player_list`. This is a convenience for relaying skin
/// updates to other players.
pub fn encode_player_skin_packet(
    buf: &mut impl BufMut,
    uuid: &Uuid,
    skin_data: &ClientData,
    new_skin_name: &str,
    old_skin_name: &str,
) {
    uuid.proto_encode(buf);
    encode_skin_data(buf, skin_data);
    write_string(buf, new_skin_name);
    write_string(buf, old_skin_name);
    buf.put_u8(1); // is_verified = true
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn packet_id_is_correct() {
        assert_eq!(super::super::id::PLAYER_SKIN, 0x5D);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let skin = PlayerSkin {
            uuid: Uuid::new(42, 99),
            skin_data: ClientData::default(),
            new_skin_name: "NewSkin".into(),
            old_skin_name: "OldSkin".into(),
        };
        let mut buf = BytesMut::new();
        skin.proto_encode(&mut buf);

        // Decode it back
        let decoded = PlayerSkin::proto_decode(&mut buf.as_ref()).unwrap();
        assert_eq!(decoded.uuid, Uuid::new(42, 99));
        assert_eq!(decoded.new_skin_name, "NewSkin");
        assert_eq!(decoded.old_skin_name, "OldSkin");
        assert_eq!(decoded.skin_data.skin_id, "");
    }

    #[test]
    fn encode_player_skin_packet_helper() {
        let mut buf = BytesMut::new();
        encode_player_skin_packet(
            &mut buf,
            &Uuid::new(1, 2),
            &ClientData::default(),
            "test",
            "old",
        );
        assert!(buf.len() > 16); // At minimum UUID + skin data + strings
    }

    #[test]
    fn decode_skin_image_empty() {
        // width=0, height=0, len=0
        let data = [0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut cursor = &data[..];
        let img = decode_skin_image(&mut cursor).unwrap();
        assert_eq!(img.width, 0);
        assert_eq!(img.height, 0);
        assert!(img.data.is_empty());
    }
}
