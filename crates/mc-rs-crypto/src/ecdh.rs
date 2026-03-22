use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use p384::ecdsa::signature::Signer;
use p384::ecdsa::{Signature, SigningKey};
use p384::pkcs8::DecodePublicKey;
use p384::{PublicKey, SecretKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EcdhError {
    #[error("failed to decode public key: {0}")]
    PublicKeyDecode(String),
    #[error("ECDH computation failed")]
    EcdhFailed,
    #[error("base64 decode error: {0}")]
    Base64Error(String),
}

/// Server ECDH keypair for key exchange.
pub struct ServerKeyPair {
    secret_key: SecretKey,
    pub public_key: PublicKey,
}

impl ServerKeyPair {
    /// Generate a new random P-384 keypair.
    pub fn generate() -> Self {
        let secret_key = SecretKey::random(&mut OsRng);
        let public_key = secret_key.public_key();
        Self {
            secret_key,
            public_key,
        }
    }

    /// Get the public key as DER-encoded base64 (for the x5u JWT field).
    pub fn public_key_base64(&self) -> String {
        use p384::pkcs8::EncodePublicKey;
        let der = self
            .public_key
            .to_public_key_der()
            .expect("failed to encode public key");
        STANDARD.encode(der.as_bytes())
    }

    /// Compute ECDH shared secret with a client's public key.
    /// Returns the raw shared secret bytes (48 bytes for P-384).
    pub fn compute_shared_secret(&self, client_pub_key: &PublicKey) -> Vec<u8> {
        use p384::ecdh::diffie_hellman;
        let shared = diffie_hellman(
            self.secret_key.to_nonzero_scalar(),
            client_pub_key.as_affine(),
        );
        shared.raw_secret_bytes().to_vec()
    }

    /// Derive AES-256 key from shared secret and salt.
    /// key = SHA-256(salt + shared_secret)
    pub fn derive_aes_key(&self, client_pub_key: &PublicKey, salt: &[u8]) -> [u8; 32] {
        let shared_secret = self.compute_shared_secret(client_pub_key);
        let mut hasher = Sha256::new();
        hasher.update(salt);
        hasher.update(&shared_secret);
        let result = hasher.finalize();
        let mut key = [0u8; 32];
        key.copy_from_slice(&result);
        key
    }

    /// Sign data with ES384 (ECDSA P-384 + SHA-384).
    pub fn sign(&self, data: &[u8]) -> Vec<u8> {
        let signing_key = SigningKey::from(&self.secret_key);
        let signature: Signature = signing_key.sign(data);
        signature.to_bytes().to_vec()
    }
}

/// Parse a client's public key from base64-encoded DER (from JWT x5u/identityPublicKey).
pub fn parse_client_public_key(b64: &str) -> Result<PublicKey, EcdhError> {
    let der_bytes = STANDARD
        .decode(b64)
        .or_else(|_| URL_SAFE_NO_PAD.decode(b64))
        .map_err(|e| EcdhError::Base64Error(e.to_string()))?;

    PublicKey::from_public_key_der(&der_bytes)
        .map_err(|e| EcdhError::PublicKeyDecode(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use p384::elliptic_curve::sec1::ToEncodedPoint;

    #[test]
    fn test_generate_keypair() {
        let kp = ServerKeyPair::generate();
        let b64 = kp.public_key_base64();
        assert!(!b64.is_empty());
        // Should be valid base64
        let decoded = STANDARD.decode(&b64).unwrap();
        assert!(decoded.len() > 50); // DER public key is ~120 bytes
    }

    #[test]
    fn test_ecdh_self() {
        // Test ECDH with two generated keypairs
        let server = ServerKeyPair::generate();
        let client = ServerKeyPair::generate();

        let salt = b"test_salt_16byte";
        let key1 = server.derive_aes_key(&client.public_key, salt);

        // Client side: same operation with reversed keys
        let shared1 = server.compute_shared_secret(&client.public_key);
        let shared2 = client.compute_shared_secret(&server.public_key);
        assert_eq!(shared1, shared2); // ECDH should produce same secret

        let key2 = client.derive_aes_key(&server.public_key, salt);
        assert_eq!(key1, key2); // Both sides derive same AES key
    }

    #[test]
    fn test_sign() {
        let kp = ServerKeyPair::generate();
        let sig = kp.sign(b"hello world");
        assert!(!sig.is_empty());
        // P-384 ECDSA signature is 96 bytes (2 * 48)
        assert_eq!(sig.len(), 96);
    }

    #[test]
    fn test_parse_own_public_key() {
        let kp = ServerKeyPair::generate();
        let b64 = kp.public_key_base64();
        let parsed = parse_client_public_key(&b64).unwrap();
        // Should be the same key
        assert_eq!(
            kp.public_key.to_encoded_point(false),
            parsed.to_encoded_point(false)
        );
    }
}
