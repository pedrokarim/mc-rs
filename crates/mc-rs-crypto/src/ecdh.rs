//! ECDH P-384 key exchange.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use p384::ecdsa::SigningKey;
use p384::pkcs8::{DecodePublicKey, EncodePublicKey};
use p384::{PublicKey, SecretKey};
use rand::rngs::OsRng;

use crate::CryptoError;

/// Server-side ECDH P-384 key pair for the encryption handshake.
pub struct ServerKeyPair {
    secret: SecretKey,
}

impl ServerKeyPair {
    /// Generate a new random P-384 key pair.
    pub fn generate() -> Self {
        Self {
            secret: SecretKey::random(&mut OsRng),
        }
    }

    /// Get the public key.
    pub fn public_key(&self) -> PublicKey {
        self.secret.public_key()
    }

    /// Compute the ECDH shared secret with a client's public key.
    ///
    /// Returns the raw 48-byte shared secret.
    pub fn shared_secret(&self, client_public: &PublicKey) -> [u8; 48] {
        let shared =
            p384::ecdh::diffie_hellman(self.secret.to_nonzero_scalar(), client_public.as_affine());
        let raw = shared.raw_secret_bytes();
        let mut result = [0u8; 48];
        result.copy_from_slice(raw.as_slice());
        result
    }

    /// Get an ECDSA signing key (for JWT signing).
    pub fn signing_key(&self) -> SigningKey {
        SigningKey::from(self.secret.clone())
    }

    /// Export the public key as SPKI DER bytes.
    pub fn public_key_der(&self) -> Vec<u8> {
        self.secret
            .public_key()
            .to_public_key_der()
            .expect("public key DER encoding should not fail")
            .as_ref()
            .to_vec()
    }

    /// Export the public key as base64-encoded SPKI DER.
    pub fn public_key_base64(&self) -> String {
        STANDARD.encode(self.public_key_der())
    }
}

/// Parse a client's public key from a base64-encoded SPKI DER string.
///
/// This is the `identityPublicKey` from the login JWT.
pub fn parse_client_public_key(base64_der: &str) -> Result<PublicKey, CryptoError> {
    let der_bytes = STANDARD
        .decode(base64_der)
        .map_err(|e| CryptoError::Base64(e.to_string()))?;
    PublicKey::from_public_key_der(&der_bytes)
        .map_err(|e| CryptoError::InvalidPublicKey(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_keypair() {
        let kp = ServerKeyPair::generate();
        let der = kp.public_key_der();
        // P-384 SPKI DER is 120 bytes
        assert_eq!(der.len(), 120);
    }

    #[test]
    fn shared_secret_matches() {
        // Two keypairs should derive the same shared secret when crossed
        let server = ServerKeyPair::generate();
        let client = ServerKeyPair::generate();

        let ss1 = server.shared_secret(&client.public_key());
        let ss2 = client.shared_secret(&server.public_key());
        assert_eq!(ss1, ss2);
    }

    #[test]
    fn public_key_der_roundtrip() {
        let kp = ServerKeyPair::generate();
        let b64 = kp.public_key_base64();
        let parsed = parse_client_public_key(&b64).unwrap();
        assert_eq!(parsed, kp.public_key());
    }

    #[test]
    fn parse_invalid_base64() {
        assert!(parse_client_public_key("not-valid-base64!!!").is_err());
    }

    #[test]
    fn parse_invalid_der() {
        let b64 = STANDARD.encode(b"not a valid DER key");
        assert!(parse_client_public_key(&b64).is_err());
    }
}
