//! NBT error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NbtError {
    #[error("unexpected end of data")]
    UnexpectedEof,

    #[error("expected TAG_Compound (10) at root, got {got}")]
    ExpectedCompound { got: u8 },

    #[error("unknown tag type: {0}")]
    UnknownTagType(u8),

    #[error("invalid UTF-8 in NBT string")]
    InvalidUtf8,

    #[error("nesting too deep (limit: {limit})")]
    NestingTooDeep { limit: usize },

    #[error("negative array length: {0}")]
    NegativeLength(i32),

    #[error("VarInt error: {0}")]
    VarInt(String),
}
