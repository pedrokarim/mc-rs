//! SetTitle (0x58) — Server → Client.
//!
//! Displays a title, subtitle, or actionbar text, or sets timing/clears.

use bytes::BufMut;

use crate::codec::{write_string, ProtoEncode};
use crate::types::VarInt;

/// Title action types.
pub const TITLE_CLEAR: i32 = 0;
pub const TITLE_RESET: i32 = 1;
pub const TITLE_SET_TITLE: i32 = 2;
pub const TITLE_SET_SUBTITLE: i32 = 3;
pub const TITLE_SET_ACTIONBAR: i32 = 4;
pub const TITLE_SET_TIMES: i32 = 5;

/// SetTitle packet.
pub struct SetTitle {
    /// Action type (0=clear, 1=reset, 2=title, 3=subtitle, 4=actionbar, 5=times).
    pub title_type: i32,
    /// Text to display (for types 2, 3, 4).
    pub text: String,
    /// Fade-in time in ticks (for type 5).
    pub fade_in: i32,
    /// Stay time in ticks (for type 5).
    pub stay: i32,
    /// Fade-out time in ticks (for type 5).
    pub fade_out: i32,
    /// Xbox user ID (empty string).
    pub xuid: String,
    /// Platform online ID (empty string).
    pub platform_id: String,
}

impl SetTitle {
    /// Create a title text packet.
    pub fn title(text: impl Into<String>) -> Self {
        Self {
            title_type: TITLE_SET_TITLE,
            text: text.into(),
            fade_in: 0,
            stay: 0,
            fade_out: 0,
            xuid: String::new(),
            platform_id: String::new(),
        }
    }

    /// Create a subtitle text packet.
    pub fn subtitle(text: impl Into<String>) -> Self {
        Self {
            title_type: TITLE_SET_SUBTITLE,
            text: text.into(),
            fade_in: 0,
            stay: 0,
            fade_out: 0,
            xuid: String::new(),
            platform_id: String::new(),
        }
    }

    /// Create an actionbar text packet.
    pub fn actionbar(text: impl Into<String>) -> Self {
        Self {
            title_type: TITLE_SET_ACTIONBAR,
            text: text.into(),
            fade_in: 0,
            stay: 0,
            fade_out: 0,
            xuid: String::new(),
            platform_id: String::new(),
        }
    }

    /// Create a clear title packet.
    pub fn clear() -> Self {
        Self {
            title_type: TITLE_CLEAR,
            text: String::new(),
            fade_in: 0,
            stay: 0,
            fade_out: 0,
            xuid: String::new(),
            platform_id: String::new(),
        }
    }

    /// Create a reset title packet.
    pub fn reset() -> Self {
        Self {
            title_type: TITLE_RESET,
            text: String::new(),
            fade_in: 0,
            stay: 0,
            fade_out: 0,
            xuid: String::new(),
            platform_id: String::new(),
        }
    }

    /// Create a times packet.
    pub fn times(fade_in: i32, stay: i32, fade_out: i32) -> Self {
        Self {
            title_type: TITLE_SET_TIMES,
            text: String::new(),
            fade_in,
            stay,
            fade_out,
            xuid: String::new(),
            platform_id: String::new(),
        }
    }
}

impl ProtoEncode for SetTitle {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        VarInt(self.title_type).proto_encode(buf);
        write_string(buf, &self.text);
        VarInt(self.fade_in).proto_encode(buf);
        VarInt(self.stay).proto_encode(buf);
        VarInt(self.fade_out).proto_encode(buf);
        write_string(buf, &self.xuid);
        write_string(buf, &self.platform_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn encode_title_text() {
        let pkt = SetTitle::title("Hello World");
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert!(buf.len() > 0);
        assert_eq!(pkt.title_type, TITLE_SET_TITLE);
        assert_eq!(pkt.text, "Hello World");
    }

    #[test]
    fn encode_title_clear() {
        let pkt = SetTitle::clear();
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(pkt.title_type, TITLE_CLEAR);
        assert!(pkt.text.is_empty());
    }

    #[test]
    fn encode_title_times() {
        let pkt = SetTitle::times(10, 70, 20);
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(pkt.title_type, TITLE_SET_TIMES);
        assert_eq!(pkt.fade_in, 10);
        assert_eq!(pkt.stay, 70);
        assert_eq!(pkt.fade_out, 20);
    }

    #[test]
    fn encode_actionbar() {
        let pkt = SetTitle::actionbar("Score: 42");
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        assert_eq!(pkt.title_type, TITLE_SET_ACTIONBAR);
        assert_eq!(pkt.text, "Score: 42");
    }
}
