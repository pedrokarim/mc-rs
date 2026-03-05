use thiserror::Error;

#[derive(Debug, Error)]
pub enum RakNetError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("packet too short: expected at least {expected} bytes, got {actual}")]
    PacketTooShort { expected: usize, actual: usize },

    #[error("invalid magic bytes")]
    InvalidMagic,

    #[error("unknown packet id: 0x{0:02X}")]
    UnknownPacketId(u8),

    #[error("invalid address version: {0}")]
    InvalidAddressVersion(u8),

    #[error("invalid reliability type: {0}")]
    InvalidReliability(u8),

    #[error("MTU out of range: {0}")]
    MtuOutOfRange(u16),

    #[error("fragment error: {0}")]
    FragmentError(String),

    #[error("invalid UTF-8 in packet")]
    InvalidUtf8,
}
