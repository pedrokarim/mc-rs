//! AES-256 key derivation from ECDH shared secret.

use sha2::{Digest, Sha256};

/// Derive an AES-256 key and IV from a salt and ECDH shared secret.
///
/// ```text
/// aes_key = SHA256(salt + shared_secret)    // 32 bytes
/// iv      = aes_key[0..16]                  // first 16 bytes
/// ```
pub fn derive_key(salt: &[u8; 16], shared_secret: &[u8]) -> ([u8; 32], [u8; 16]) {
    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update(shared_secret);
    let hash = hasher.finalize();

    let mut aes_key = [0u8; 32];
    aes_key.copy_from_slice(&hash);

    let mut iv = [0u8; 16];
    iv.copy_from_slice(&aes_key[..16]);

    (aes_key, iv)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_key_deterministic() {
        let salt = [0x42u8; 16];
        let shared = [0xABu8; 48];

        let (key1, iv1) = derive_key(&salt, &shared);
        let (key2, iv2) = derive_key(&salt, &shared);

        assert_eq!(key1, key2);
        assert_eq!(iv1, iv2);
    }

    #[test]
    fn iv_is_first_16_bytes_of_key() {
        let salt = [0x01u8; 16];
        let shared = [0x02u8; 48];

        let (key, iv) = derive_key(&salt, &shared);
        assert_eq!(&key[..16], &iv);
    }

    #[test]
    fn different_salt_different_key() {
        let shared = [0xFFu8; 48];
        let (key1, _) = derive_key(&[0x00u8; 16], &shared);
        let (key2, _) = derive_key(&[0x01u8; 16], &shared);
        assert_ne!(key1, key2);
    }
}
