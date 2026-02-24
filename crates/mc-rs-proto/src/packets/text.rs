//! Text (0x09) — Bidirectional.
//!
//! Chat messages, system messages, and raw server messages.
//! The wire format has conditional fields based on TextType.

use bytes::{Buf, BufMut};

use crate::codec::{read_string, write_string, ProtoDecode, ProtoEncode};
use crate::error::ProtoError;
use crate::types::VarUInt32;

/// Text message type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TextType {
    Raw = 0,
    Chat = 1,
    Translation = 2,
    Popup = 3,
    JukeboxPopup = 4,
    Tip = 5,
    System = 6,
    Whisper = 7,
    Announcement = 8,
    ObjectWhisper = 9,
    Object = 10,
}

impl TextType {
    fn from_u8(v: u8) -> Result<Self, ProtoError> {
        match v {
            0 => Ok(Self::Raw),
            1 => Ok(Self::Chat),
            2 => Ok(Self::Translation),
            3 => Ok(Self::Popup),
            4 => Ok(Self::JukeboxPopup),
            5 => Ok(Self::Tip),
            6 => Ok(Self::System),
            7 => Ok(Self::Whisper),
            8 => Ok(Self::Announcement),
            9 => Ok(Self::ObjectWhisper),
            10 => Ok(Self::Object),
            _ => Err(ProtoError::InvalidLogin(format!("unknown TextType: {v}"))),
        }
    }

    /// Whether this type has a SourceName field.
    fn has_source(self) -> bool {
        matches!(self, Self::Chat | Self::Whisper | Self::Announcement)
    }

    /// Whether this type has a Parameters array.
    fn has_parameters(self) -> bool {
        matches!(self, Self::Translation | Self::Popup | Self::JukeboxPopup)
    }
}

/// Text packet.
#[derive(Debug, Clone)]
pub struct Text {
    pub text_type: TextType,
    pub needs_translation: bool,
    pub source_name: String,
    pub message: String,
    pub parameters: Vec<String>,
    pub xuid: String,
    pub platform_chat_id: String,
    pub filtered_message: String,
}

impl Text {
    /// Create a Raw text message (server → client).
    pub fn raw(message: impl Into<String>) -> Self {
        Self {
            text_type: TextType::Raw,
            needs_translation: false,
            source_name: String::new(),
            message: message.into(),
            parameters: Vec::new(),
            xuid: String::new(),
            platform_chat_id: String::new(),
            filtered_message: String::new(),
        }
    }

    /// Create a System text message (server → client).
    pub fn system(message: impl Into<String>) -> Self {
        Self {
            text_type: TextType::System,
            needs_translation: false,
            source_name: String::new(),
            message: message.into(),
            parameters: Vec::new(),
            xuid: String::new(),
            platform_chat_id: String::new(),
            filtered_message: String::new(),
        }
    }
}

impl ProtoEncode for Text {
    fn proto_encode(&self, buf: &mut impl BufMut) {
        buf.put_u8(self.text_type as u8);
        buf.put_u8(self.needs_translation as u8);

        if self.text_type.has_source() {
            write_string(buf, &self.source_name);
        }

        write_string(buf, &self.message);

        if self.text_type.has_parameters() {
            VarUInt32(self.parameters.len() as u32).proto_encode(buf);
            for param in &self.parameters {
                write_string(buf, param);
            }
        }

        write_string(buf, &self.xuid);
        write_string(buf, &self.platform_chat_id);
        write_string(buf, &self.filtered_message);
    }
}

impl ProtoDecode for Text {
    fn proto_decode(buf: &mut impl Buf) -> Result<Self, ProtoError> {
        if buf.remaining() < 2 {
            return Err(ProtoError::BufferTooShort {
                needed: 2,
                remaining: buf.remaining(),
            });
        }
        let text_type = TextType::from_u8(buf.get_u8())?;
        let needs_translation = buf.get_u8() != 0;

        let source_name = if text_type.has_source() {
            read_string(buf)?
        } else {
            String::new()
        };

        let message = read_string(buf)?;

        let parameters = if text_type.has_parameters() {
            let count = VarUInt32::proto_decode(buf)?.0 as usize;
            let mut params = Vec::with_capacity(count.min(64));
            for _ in 0..count {
                params.push(read_string(buf)?);
            }
            params
        } else {
            Vec::new()
        };

        let xuid = read_string(buf)?;
        let platform_chat_id = read_string(buf)?;
        let filtered_message = read_string(buf)?;

        Ok(Self {
            text_type,
            needs_translation,
            source_name,
            message,
            parameters,
            xuid,
            platform_chat_id,
            filtered_message,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn roundtrip_raw() {
        let pkt = Text::raw("Hello, world!");
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        let decoded = Text::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded.text_type, TextType::Raw);
        assert!(!decoded.needs_translation);
        assert_eq!(decoded.source_name, "");
        assert_eq!(decoded.message, "Hello, world!");
        assert!(decoded.parameters.is_empty());
    }

    #[test]
    fn roundtrip_system() {
        let pkt = Text::system("Server shutting down");
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        let decoded = Text::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded.text_type, TextType::System);
        assert_eq!(decoded.message, "Server shutting down");
    }

    #[test]
    fn roundtrip_chat() {
        let pkt = Text {
            text_type: TextType::Chat,
            needs_translation: false,
            source_name: "Steve".into(),
            message: "hello".into(),
            parameters: Vec::new(),
            xuid: "12345".into(),
            platform_chat_id: String::new(),
            filtered_message: String::new(),
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        let decoded = Text::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded.text_type, TextType::Chat);
        assert_eq!(decoded.source_name, "Steve");
        assert_eq!(decoded.message, "hello");
        assert_eq!(decoded.xuid, "12345");
    }

    #[test]
    fn roundtrip_translation() {
        let pkt = Text {
            text_type: TextType::Translation,
            needs_translation: true,
            source_name: String::new(),
            message: "chat.type.text".into(),
            parameters: vec!["Steve".into(), "hello".into()],
            xuid: String::new(),
            platform_chat_id: String::new(),
            filtered_message: String::new(),
        };
        let mut buf = BytesMut::new();
        pkt.proto_encode(&mut buf);
        let decoded = Text::proto_decode(&mut buf.freeze()).unwrap();
        assert_eq!(decoded.text_type, TextType::Translation);
        assert!(decoded.needs_translation);
        assert_eq!(decoded.parameters.len(), 2);
        assert_eq!(decoded.parameters[0], "Steve");
        assert_eq!(decoded.parameters[1], "hello");
    }
}
