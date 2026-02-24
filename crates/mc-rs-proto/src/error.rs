//! Protocol-level errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProtoError {
    #[error("buffer too short: need {needed} more bytes, have {remaining}")]
    BufferTooShort { needed: usize, remaining: usize },

    #[error("VarInt encoding error: {0}")]
    VarInt(#[from] crate::types::VarIntError),

    #[error("invalid UTF-8 string")]
    InvalidUtf8,

    #[error("decompression error: {0}")]
    DecompressError(String),

    #[error("compression error: {0}")]
    CompressError(String),

    #[error("unknown compression algorithm: {0}")]
    UnknownCompression(u16),

    #[error("packet batch is empty")]
    EmptyBatch,

    #[error("protocol version mismatch: expected {expected}, got {got}")]
    ProtocolVersionMismatch { expected: i32, got: i32 },

    #[error("unknown packet id: 0x{0:02X}")]
    UnknownPacketId(u32),

    #[error("JWT decode error: {0}")]
    JwtDecode(String),

    #[error("JSON parse error: {0}")]
    JsonParse(String),

    #[error("invalid login data: {0}")]
    InvalidLogin(String),

    #[error("encryption error: {0}")]
    EncryptionError(String),

    #[error("packet checksum mismatch")]
    ChecksumMismatch,

    #[error("invalid data: {0}")]
    InvalidData(String),
}
