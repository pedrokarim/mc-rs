//! UpdateAdventureSettings (0xBC) — Server -> Client.
//!
//! World-scoped adventure flags (PvM/PvP/build lock/name tags/auto jump).

use bytes::BufMut;

use crate::codec::ProtoEncode;

/// Update world adventure settings flags.
#[derive(Debug, Clone)]
pub struct UpdateAdventureSettings {
    pub no_attacking_mobs: bool,
    pub no_attacking_players: bool,
    pub world_immutable: bool,
    pub show_name_tags: bool,
    pub auto_jump: bool,
}

impl Default for UpdateAdventureSettings {
    fn default() -> Self {
        Self {
            no_attacking_mobs: false,
            no_attacking_players: false,
            world_immutable: false,
            show_name_tags: true,
            auto_jump: true,
        }
    }
}

impl ProtoEncode for UpdateAdventureSettings {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.no_attacking_mobs as u8);
        buf.put_u8(self.no_attacking_players as u8);
        buf.put_u8(self.world_immutable as u8);
        buf.put_u8(self.show_name_tags as u8);
        buf.put_u8(self.auto_jump as u8);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_default_flags() {
        let mut buf = BytesMut::new();
        UpdateAdventureSettings::default().proto_encode(&mut buf);
        assert_eq!(buf.len(), 5);
        assert_eq!(&buf[..], &[0, 0, 0, 1, 1]);
    }
}
