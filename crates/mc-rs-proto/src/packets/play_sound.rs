//! PlaySound (0x56) — Server → Client.
//!
//! Plays a sound effect at a specific position.

use bytes::BufMut;

use crate::codec::{write_string, ProtoEncode};
use crate::types::BlockPos;

/// PlaySound packet.
pub struct PlaySound {
    /// Sound identifier (e.g. "random.levelup", "mob.zombie.say").
    pub sound_name: String,
    /// Block position (world coordinates multiplied by 8).
    pub position: BlockPos,
    /// Volume (0.0–1.0+).
    pub volume: f32,
    /// Pitch (0.0–2.0, 1.0 = normal).
    pub pitch: f32,
}

impl PlaySound {
    /// Create a PlaySound packet at the given world coordinates.
    pub fn new(
        sound_name: impl Into<String>,
        x: f32,
        y: f32,
        z: f32,
        volume: f32,
        pitch: f32,
    ) -> Self {
        Self {
            sound_name: sound_name.into(),
            position: BlockPos::new((x * 8.0) as i32, (y * 8.0) as i32, (z * 8.0) as i32),
            volume,
            pitch,
        }
    }
}

impl ProtoEncode for PlaySound {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        write_string(buf, &self.sound_name);
        self.position.proto_encode(buf);
        buf.put_f32_le(self.volume);
        buf.put_f32_le(self.pitch);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_play_sound() {
        let pkt = PlaySound::new("random.levelup", 10.0, 64.0, -5.0, 1.0, 1.0);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
        assert_eq!(pkt.position.x, 80); // 10 * 8
        assert_eq!(pkt.position.y, 512); // 64 * 8
        assert_eq!(pkt.position.z, -40); // -5 * 8
    }

    #[test]
    fn encode_play_sound_custom_volume_pitch() {
        let pkt = PlaySound::new("mob.zombie.say", 0.0, 0.0, 0.0, 0.5, 1.5);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(!buf.is_empty());
        assert_eq!(pkt.volume, 0.5);
        assert_eq!(pkt.pitch, 1.5);
    }
}
