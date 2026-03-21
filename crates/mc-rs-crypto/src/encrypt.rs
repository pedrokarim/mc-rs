use aes::cipher::{KeyIvInit, StreamCipher};
use sha2::{Digest, Sha256};

type Aes256Ctr = ctr::Ctr64BE<aes::Aes256>;

/// AES-256-CTR "fakeGCM" encryption context.
/// Matches PocketMine's EncryptionContext exactly.
///
/// - IV = key[0..12] + \x00\x00\x00\x02
/// - Per-packet checksum: SHA-256(counter_u64_LE + payload + key)[0..8]
/// - Counter increments per packet (u64 LE)
pub struct EncryptionContext {
    key: [u8; 32],
    encrypt_cipher: Aes256Ctr,
    decrypt_cipher: Aes256Ctr,
    encrypt_counter: u64,
    decrypt_counter: u64,
}

impl EncryptionContext {
    /// Create a new encryption context from the derived AES-256 key.
    pub fn new(key: [u8; 32]) -> Self {
        let iv = Self::make_iv(&key);
        let encrypt_cipher = Aes256Ctr::new(&key.into(), &iv.into());
        let decrypt_cipher = Aes256Ctr::new(&key.into(), &iv.into());

        Self {
            key,
            encrypt_cipher,
            decrypt_cipher,
            encrypt_counter: 0,
            decrypt_counter: 0,
        }
    }

    /// Build the IV: key[0..12] + \x00\x00\x00\x02
    /// The \x02 emulates GCM's internal counter starting at 2.
    fn make_iv(key: &[u8; 32]) -> [u8; 16] {
        let mut iv = [0u8; 16];
        iv[..12].copy_from_slice(&key[..12]);
        iv[12] = 0x00;
        iv[13] = 0x00;
        iv[14] = 0x00;
        iv[15] = 0x02;
        iv
    }

    /// Encrypt a payload. Returns the encrypted bytes (payload + 8-byte checksum).
    pub fn encrypt(&mut self, payload: &[u8]) -> Vec<u8> {
        // Compute checksum: SHA-256(counter_LE + payload + key)[0..8]
        let checksum = self.compute_checksum(self.encrypt_counter, payload);

        // Plaintext = payload + checksum
        let mut plaintext = Vec::with_capacity(payload.len() + 8);
        plaintext.extend_from_slice(payload);
        plaintext.extend_from_slice(&checksum);

        // Encrypt in-place with CTR
        self.encrypt_cipher.apply_keystream(&mut plaintext);

        self.encrypt_counter += 1;
        plaintext
    }

    /// Decrypt an encrypted payload. Returns the decrypted payload (without checksum).
    pub fn decrypt(&mut self, encrypted: &[u8]) -> Result<Vec<u8>, EncryptionError> {
        if encrypted.len() < 8 {
            return Err(EncryptionError::TooShort);
        }

        // Decrypt in-place
        let mut decrypted = encrypted.to_vec();
        self.decrypt_cipher.apply_keystream(&mut decrypted);

        // Split into payload + checksum
        let payload_len = decrypted.len() - 8;
        let payload = &decrypted[..payload_len];
        let received_checksum = &decrypted[payload_len..];

        // Verify checksum
        let expected_checksum = self.compute_checksum(self.decrypt_counter, payload);
        if received_checksum != expected_checksum {
            return Err(EncryptionError::ChecksumMismatch);
        }

        self.decrypt_counter += 1;
        Ok(payload.to_vec())
    }

    /// Compute the 8-byte checksum: SHA-256(counter_u64_LE + payload + key)[0..8]
    fn compute_checksum(&self, counter: u64, payload: &[u8]) -> [u8; 8] {
        let mut hasher = Sha256::new();
        hasher.update(counter.to_le_bytes());
        hasher.update(payload);
        hasher.update(self.key);
        let hash = hasher.finalize();
        let mut checksum = [0u8; 8];
        checksum.copy_from_slice(&hash[..8]);
        checksum
    }
}

#[derive(Debug)]
pub enum EncryptionError {
    TooShort,
    ChecksumMismatch,
}

impl std::fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "encrypted data too short"),
            Self::ChecksumMismatch => write!(f, "checksum mismatch"),
        }
    }
}

impl std::error::Error for EncryptionError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        let mut key = [0u8; 32];
        for (i, b) in key.iter_mut().enumerate() {
            *b = i as u8;
        }
        key
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = test_key();
        let mut ctx = EncryptionContext::new(key);

        let payload = b"Hello, Minecraft!";
        let encrypted = ctx.encrypt(payload);
        assert_ne!(encrypted, payload.as_slice());
        assert_eq!(encrypted.len(), payload.len() + 8);

        // Need a second context for decryption (same key, fresh state)
        // Actually, in real use, encrypt and decrypt share the same CTR stream
        // but we need separate cipher instances
        let mut ctx2 = EncryptionContext::new(key);
        // Advance encrypt to match — actually we need to decrypt with a fresh ctx
        // In practice: server encrypts, client decrypts (separate contexts)
        let decrypted = ctx2.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, payload);
    }

    #[test]
    fn test_multiple_packets() {
        let key = test_key();
        let mut encrypt_ctx = EncryptionContext::new(key);
        let mut decrypt_ctx = EncryptionContext::new(key);

        for i in 0..10u8 {
            let payload = vec![i; 50];
            let encrypted = encrypt_ctx.encrypt(&payload);
            let decrypted = decrypt_ctx.decrypt(&encrypted).unwrap();
            assert_eq!(decrypted, payload);
        }
    }

    #[test]
    fn test_checksum_mismatch() {
        let key = test_key();
        let mut encrypt_ctx = EncryptionContext::new(key);
        let mut decrypt_ctx = EncryptionContext::new(key);

        let payload = b"test";
        let mut encrypted = encrypt_ctx.encrypt(payload);

        // Corrupt last byte (part of checksum)
        *encrypted.last_mut().unwrap() ^= 0xFF;

        assert!(decrypt_ctx.decrypt(&encrypted).is_err());
    }

    #[test]
    fn test_iv_construction() {
        let key = test_key();
        let iv = EncryptionContext::make_iv(&key);
        assert_eq!(&iv[..12], &key[..12]);
        assert_eq!(iv[12], 0x00);
        assert_eq!(iv[13], 0x00);
        assert_eq!(iv[14], 0x00);
        assert_eq!(iv[15], 0x02);
    }

    #[test]
    fn test_counter_increments() {
        let key = test_key();
        let mut ctx = EncryptionContext::new(key);
        assert_eq!(ctx.encrypt_counter, 0);
        ctx.encrypt(b"test1");
        assert_eq!(ctx.encrypt_counter, 1);
        ctx.encrypt(b"test2");
        assert_eq!(ctx.encrypt_counter, 2);
    }
}
