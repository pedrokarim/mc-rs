//! JWT creation and ES384 signing for the encryption handshake.

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use p384::ecdsa::{signature::Signer, Signature};

use crate::ecdh::ServerKeyPair;
use crate::CryptoError;

/// Create a signed JWT for the ServerToClientHandshake packet.
///
/// The JWT contains the server's public key and a random salt,
/// signed with ES384 using the server's ECDH private key.
///
/// ```text
/// Header:  {"alg":"ES384","x5u":"<base64 server pubkey DER>"}
/// Payload: {"salt":"<base64 16-byte salt>","signedToken":"<base64 server pubkey DER>"}
/// ```
pub fn create_handshake_jwt(
    keypair: &ServerKeyPair,
    salt: &[u8; 16],
) -> Result<String, CryptoError> {
    let pubkey_b64 = keypair.public_key_base64();

    let header = serde_json::json!({
        "alg": "ES384",
        "x5u": pubkey_b64,
    });

    let payload = serde_json::json!({
        "salt": STANDARD.encode(salt),
        "signedToken": pubkey_b64,
    });

    let header_b64 = URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&header).map_err(|e| CryptoError::JwtSign(e.to_string()))?);
    let payload_b64 = URL_SAFE_NO_PAD
        .encode(serde_json::to_vec(&payload).map_err(|e| CryptoError::JwtSign(e.to_string()))?);

    let message = format!("{header_b64}.{payload_b64}");

    let signing_key = keypair.signing_key();
    let signature: Signature = signing_key
        .try_sign(message.as_bytes())
        .map_err(|e| CryptoError::JwtSign(e.to_string()))?;
    let sig_b64 = URL_SAFE_NO_PAD.encode(signature.to_bytes());

    Ok(format!("{message}.{sig_b64}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecdh::ServerKeyPair;

    #[test]
    fn create_jwt_has_three_parts() {
        let kp = ServerKeyPair::generate();
        let salt = [0x42u8; 16];
        let jwt = create_handshake_jwt(&kp, &salt).unwrap();

        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn jwt_header_contains_es384() {
        let kp = ServerKeyPair::generate();
        let salt = [0x01u8; 16];
        let jwt = create_handshake_jwt(&kp, &salt).unwrap();

        let header_b64 = jwt.split('.').next().unwrap();
        let header_bytes = URL_SAFE_NO_PAD.decode(header_b64).unwrap();
        let header: serde_json::Value = serde_json::from_slice(&header_bytes).unwrap();

        assert_eq!(header["alg"], "ES384");
        assert!(header["x5u"].as_str().is_some());
    }

    #[test]
    fn jwt_payload_contains_salt_and_token() {
        let kp = ServerKeyPair::generate();
        let salt = [0xABu8; 16];
        let jwt = create_handshake_jwt(&kp, &salt).unwrap();

        let payload_b64 = jwt.split('.').nth(1).unwrap();
        let payload_bytes = URL_SAFE_NO_PAD.decode(payload_b64).unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&payload_bytes).unwrap();

        assert!(payload["salt"].as_str().is_some());
        assert!(payload["signedToken"].as_str().is_some());

        // Verify salt decodes to our original
        let decoded_salt = STANDARD.decode(payload["salt"].as_str().unwrap()).unwrap();
        assert_eq!(decoded_salt, &salt);
    }
}
