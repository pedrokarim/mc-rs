//! AES-256-CFB8 packet encryption with SHA-256 checksums.

use aes::Aes256;
use bytes::{BufMut, Bytes, BytesMut};
use cfb8::cipher::generic_array::GenericArray;
use cfb8::cipher::KeyIvInit;
use cfb8::cipher::{BlockDecryptMut, BlockEncryptMut};
use cfb8::{Decryptor, Encryptor};
use sha2::{Digest, Sha256};

use crate::CryptoError;

/// Stateful packet encryption using AES-256-CFB8.
///
/// The cipher state is continuous across packets — each packet continues
/// the stream from where the previous one left off. Separate cipher
/// instances are used for send and receive directions.
pub struct PacketEncryption {
    encrypt_cipher: Encryptor<Aes256>,
    decrypt_cipher: Decryptor<Aes256>,
    aes_key: [u8; 32],
    send_counter: u64,
    recv_counter: u64,
}

impl PacketEncryption {
    /// Create a new encryption context.
    pub fn new(aes_key: &[u8; 32], iv: &[u8; 16]) -> Self {
        Self {
            encrypt_cipher: Encryptor::<Aes256>::new(aes_key.into(), iv.into()),
            decrypt_cipher: Decryptor::<Aes256>::new(aes_key.into(), iv.into()),
            aes_key: *aes_key,
            send_counter: 0,
            recv_counter: 0,
        }
    }

    /// Encrypt a compressed batch payload.
    ///
    /// 1. Compute checksum = SHA256(counter_le + payload + key)[0..8]
    /// 2. Append checksum to payload
    /// 3. AES-256-CFB8 encrypt the whole thing in-place
    pub fn encrypt(&mut self, payload: &[u8]) -> Bytes {
        let checksum = compute_checksum(self.send_counter, payload, &self.aes_key);
        self.send_counter += 1;

        let mut data = BytesMut::with_capacity(payload.len() + 8);
        data.put_slice(payload);
        data.put_slice(&checksum);

        // Encrypt byte-by-byte using BlockEncryptMut (preserves cipher state)
        for byte in data.iter_mut() {
            let mut block = GenericArray::clone_from_slice(std::slice::from_ref(byte));
            self.encrypt_cipher.encrypt_block_mut(&mut block);
            *byte = block[0];
        }

        data.freeze()
    }

    /// Decrypt a received encrypted payload.
    ///
    /// 1. AES-256-CFB8 decrypt in-place
    /// 2. Split into payload + 8-byte checksum
    /// 3. Verify checksum
    pub fn decrypt(&mut self, data: &[u8]) -> Result<Bytes, CryptoError> {
        if data.len() < 8 {
            return Err(CryptoError::ChecksumMismatch);
        }

        let mut buf = data.to_vec();

        // Decrypt byte-by-byte using BlockDecryptMut (preserves cipher state)
        for byte in buf.iter_mut() {
            let mut block = GenericArray::clone_from_slice(std::slice::from_ref(byte));
            self.decrypt_cipher.decrypt_block_mut(&mut block);
            *byte = block[0];
        }

        let payload_len = buf.len() - 8;
        let payload = &buf[..payload_len];
        let received_checksum = &buf[payload_len..];

        let expected = compute_checksum(self.recv_counter, payload, &self.aes_key);
        self.recv_counter += 1;

        if received_checksum != expected {
            return Err(CryptoError::ChecksumMismatch);
        }

        Ok(Bytes::copy_from_slice(payload))
    }
}

/// Compute the 8-byte SHA-256 checksum for packet integrity.
///
/// ```text
/// checksum = SHA256(counter_le_u64 + payload + aes_key)[0..8]
/// ```
fn compute_checksum(counter: u64, payload: &[u8], aes_key: &[u8; 32]) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(counter.to_le_bytes());
    hasher.update(payload);
    hasher.update(aes_key);
    let hash = hasher.finalize();
    let mut result = [0u8; 8];
    result.copy_from_slice(&hash[..8]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key_iv() -> ([u8; 32], [u8; 16]) {
        let key = [0x42u8; 32];
        let iv = [0x42u8; 16];
        (key, iv)
    }

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let (key, iv) = test_key_iv();
        let mut enc = PacketEncryption::new(&key, &iv);

        let payload = b"hello world compressed batch data";
        let encrypted = enc.encrypt(payload);

        // Encrypted data should be different from plaintext
        assert_ne!(&encrypted[..], payload.as_slice());
        // Encrypted data should be 8 bytes longer (checksum)
        assert_eq!(encrypted.len(), payload.len() + 8);

        // Create a separate decryptor with same key/iv
        let mut dec = PacketEncryption::new(&key, &iv);
        let decrypted = dec.decrypt(&encrypted).unwrap();
        assert_eq!(&decrypted[..], payload.as_slice());
    }

    #[test]
    fn bad_checksum_rejected() {
        let (key, iv) = test_key_iv();
        let mut enc = PacketEncryption::new(&key, &iv);
        let mut dec = PacketEncryption::new(&key, &iv);

        let encrypted = enc.encrypt(b"test data");
        let mut corrupted = encrypted.to_vec();
        // Flip a bit in the encrypted data
        if let Some(last) = corrupted.last_mut() {
            *last ^= 0x01;
        }

        assert!(dec.decrypt(&corrupted).is_err());
    }

    #[test]
    fn multi_packet_counter() {
        let (key, iv) = test_key_iv();
        let mut enc = PacketEncryption::new(&key, &iv);
        let mut dec = PacketEncryption::new(&key, &iv);

        // Send multiple packets — counters must stay in sync
        for i in 0..10 {
            let payload = format!("packet {i}");
            let encrypted = enc.encrypt(payload.as_bytes());
            let decrypted = dec.decrypt(&encrypted).unwrap();
            assert_eq!(&decrypted[..], payload.as_bytes());
        }
    }

    #[test]
    fn empty_payload() {
        let (key, iv) = test_key_iv();
        let mut enc = PacketEncryption::new(&key, &iv);
        let mut dec = PacketEncryption::new(&key, &iv);

        let encrypted = enc.encrypt(b"");
        assert_eq!(encrypted.len(), 8); // just checksum
        let decrypted = dec.decrypt(&encrypted).unwrap();
        assert!(decrypted.is_empty());
    }

    #[test]
    fn too_short_data_rejected() {
        let (key, iv) = test_key_iv();
        let mut dec = PacketEncryption::new(&key, &iv);
        // Less than 8 bytes = no room for checksum
        assert!(dec.decrypt(&[0u8; 7]).is_err());
    }
}
