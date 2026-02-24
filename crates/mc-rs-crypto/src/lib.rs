//! Cryptography: ECDH P-384, AES-256-CFB8, key derivation, JWT signing.

pub mod aes;
pub mod ecdh;
pub mod jwt_sign;
pub mod key_derive;

pub use aes::PacketEncryption;
pub use ecdh::{parse_client_public_key, ServerKeyPair};
pub use jwt_sign::create_handshake_jwt;
pub use key_derive::derive_key;

use thiserror::Error;

/// Cryptographic operation errors.
#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("ECDH error: {0}")]
    Ecdh(String),

    #[error("invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("packet checksum mismatch")]
    ChecksumMismatch,

    #[error("JWT signing error: {0}")]
    JwtSign(String),

    #[error("base64 decode error: {0}")]
    Base64(String),
}
