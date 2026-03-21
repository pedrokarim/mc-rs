use crate::codec::*;
use bytes::{BufMut, BytesMut};

/// PlayerListPacket — add a single player
pub fn encode_add(
    uuid: &[u8; 16],
    actor_unique_id: i64,
    username: &str,
    xuid: &str,
    platform_chat_id: &str,
    build_platform: i32,
    skin_data: &[u8],
    is_teacher: bool,
    is_host: bool,
    is_sub_client: bool,
    color_argb: u32,
    skin_verified: bool,
) -> BytesMut {
    let mut buf = BytesMut::new();
    buf.put_u8(0); // type = ADD
    write_unsigned_varint32(&mut buf, 1); // 1 entry

    // UUID
    write_bedrock_uuid(&mut buf, uuid);
    // actor unique id
    write_signed_varlong(&mut buf, actor_unique_id);
    // username
    write_string(&mut buf, username);
    // xuid
    write_string(&mut buf, xuid);
    // platform chat id
    write_string(&mut buf, platform_chat_id);
    // build platform
    buf.put_i32_le(build_platform);
    // skin data (pre-serialized)
    buf.extend_from_slice(skin_data);
    // isTeacher
    buf.put_u8(is_teacher as u8);
    // isHost
    buf.put_u8(is_host as u8);
    // isSubClient
    buf.put_u8(is_sub_client as u8);
    // Name-tag color (ARGB, LE)
    buf.put_u32_le(color_argb);
    // Skin verified flag (written after entries in PMMP; same effect for 1 entry)
    buf.put_u8(skin_verified as u8);

    buf
}

/// Bedrock UUID wire format: two little-endian 64-bit chunks.
/// PMMP: strrev(first 8 bytes) + strrev(last 8 bytes).
fn write_bedrock_uuid(buf: &mut BytesMut, uuid: &[u8; 16]) {
    let mut p1 = [0u8; 8];
    p1.copy_from_slice(&uuid[0..8]);
    p1.reverse();
    buf.put_slice(&p1);

    let mut p2 = [0u8; 8];
    p2.copy_from_slice(&uuid[8..16]);
    p2.reverse();
    buf.put_slice(&p2);
}

/// Build minimal skin data matching PocketMine's SkinData serialization exactly.
/// Format: CommonTypes::putSkin() from BedrockProtocol
pub fn build_minimal_skin() -> Vec<u8> {
    let mut buf = Vec::new();

    write_string_vec(&mut buf, "Standard_Custom");
    write_string_vec(&mut buf, "");
    write_string_vec(
        &mut buf,
        r#"{"geometry":{"default":"geometry.humanoid.custom"}}"#,
    );

    // Skin image (SkinImage): width(u32_le) + height(u32_le) + data(String)
    buf.extend_from_slice(&64u32.to_le_bytes());
    buf.extend_from_slice(&64u32.to_le_bytes());
    let skin_pixels = vec![0u8; 64 * 64 * 4];
    write_unsigned_varint32_vec(&mut buf, skin_pixels.len() as u32);
    buf.extend_from_slice(&skin_pixels);

    // Animations count (u32 LE)
    buf.extend_from_slice(&0u32.to_le_bytes());

    // Cape image (SkinImage): width=0, height=0, data=""
    buf.extend_from_slice(&0u32.to_le_bytes());
    buf.extend_from_slice(&0u32.to_le_bytes());
    write_unsigned_varint32_vec(&mut buf, 0);

    write_string_vec(&mut buf, ""); // geometryData
    write_string_vec(&mut buf, ""); // geometryDataEngineVersion
    write_string_vec(&mut buf, ""); // animationData
    write_string_vec(&mut buf, ""); // capeId
    write_string_vec(&mut buf, "Standard_Custom_custom"); // fullSkinId
    write_string_vec(&mut buf, "wide"); // armSize
    write_string_vec(&mut buf, "#0"); // skinColor

    buf.extend_from_slice(&0u32.to_le_bytes()); // personaPieces count
    buf.extend_from_slice(&0u32.to_le_bytes()); // pieceTintColors count

    buf.push(0); // isPremiumSkin
    buf.push(0); // isPersonaSkin
    buf.push(0); // isCapeOnClassicSkin
    buf.push(1); // isPrimaryUser
    buf.push(0); // isOverridingPlayerAppearance

    buf
}

fn write_string_vec(buf: &mut Vec<u8>, s: &str) {
    write_unsigned_varint32_vec(buf, s.len() as u32);
    buf.extend_from_slice(s.as_bytes());
}

fn write_unsigned_varint32_vec(buf: &mut Vec<u8>, mut v: u32) {
    loop {
        if v & !0x7F == 0 {
            buf.push(v as u8);
            return;
        }
        buf.push((v & 0x7F | 0x80) as u8);
        v >>= 7;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_entry_contains_color_and_verified_flag() {
        let skin = build_minimal_skin();
        let uuid = [0xAB; 16];
        let color = 0xFFFF_FFFF;
        let out = encode_add(
            &uuid, 1, "Player", "", "", 7, &skin, false, false, false, color, true,
        );

        assert!(out.len() > skin.len());
        let end = out.len();
        assert_eq!(&out[end - 5..end - 1], &color.to_le_bytes());
        assert_eq!(out[end - 1], 1);
    }

    #[test]
    fn uuid_is_written_in_bedrock_endian_layout() {
        let mut uuid = [0u8; 16];
        for (i, b) in uuid.iter_mut().enumerate() {
            *b = i as u8;
        }

        let out = encode_add(
            &uuid, 1, "p", "", "", 7, &build_minimal_skin(), false, false, false, 0xFFFF_FFFF,
            false,
        );

        // type(1) + count(1) then UUID(16)
        let wire = &out[2..18];
        assert_eq!(wire, &[7, 6, 5, 4, 3, 2, 1, 0, 15, 14, 13, 12, 11, 10, 9, 8]);
    }
}
