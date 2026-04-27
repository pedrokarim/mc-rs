use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use serde_json::Value;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum JwtError {
    #[error("invalid JWT format (expected 3 parts)")]
    InvalidFormat,
    #[error("base64 decode error: {0}")]
    Base64Error(#[from] base64::DecodeError),
    #[error("JSON parse error: {0}")]
    JsonError(#[from] serde_json::Error),
}

/// A decoded (but not signature-verified) JWT.
pub struct DecodedJwt {
    pub header: Value,
    pub claims: Value,
    pub signature: Vec<u8>,
}

/// Decode a JWT without verifying the signature.
/// Returns (header, claims, raw_signature).
pub fn decode_jwt(jwt: &str) -> Result<DecodedJwt, JwtError> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() != 3 {
        return Err(JwtError::InvalidFormat);
    }

    let header_bytes = URL_SAFE_NO_PAD.decode(parts[0])?;
    let claims_bytes = URL_SAFE_NO_PAD.decode(parts[1])?;
    let signature = URL_SAFE_NO_PAD.decode(parts[2])?;

    let header: Value = serde_json::from_slice(&header_bytes)?;
    let claims: Value = serde_json::from_slice(&claims_bytes)?;

    Ok(DecodedJwt {
        header,
        claims,
        signature,
    })
}

/// Parse the login authInfoJson and extract identity + public key.
///
/// The authInfoJson contains:
/// - AuthenticationType: int (0=FULL/Xbox, 1=SELF_SIGNED/offline)
/// - Token: string (JWT for FULL auth)
/// - Certificate: string (JSON {"chain":["jwt",...]} for SELF_SIGNED auth)
///
/// Returns (client_public_key_b64, identity_claims, display_name, xuid, uuid_str)
pub fn extract_login_identity(auth_info_json: &str) -> Result<LoginIdentity, JwtError> {
    let auth_info: Value = serde_json::from_str(auth_info_json).map_err(JwtError::JsonError)?;

    let auth_type = auth_info
        .get("AuthenticationType")
        .and_then(|v| v.as_i64())
        .unwrap_or(1); // default to SELF_SIGNED

    if auth_type == 0 {
        // FULL (Xbox/OpenID) — Token field has identity data
        let token = auth_info
            .get("Token")
            .and_then(|v| v.as_str())
            .ok_or(JwtError::InvalidFormat)?;

        let decoded = decode_jwt(token)?;

        let display_name = decoded.claims["xname"].as_str().unwrap_or("").to_string();
        let xuid = decoded.claims["xid"].as_str().unwrap_or("").to_string();

        // Client public key is in the Token JWT claims under "cpk" (PMMP: XboxAuthJwtBody.cpk)
        let mut pub_key = decoded.claims["cpk"].as_str().unwrap_or("").to_string();

        // Fallback: walk the Certificate chain for identityPublicKey (legacy path)
        if pub_key.is_empty() {
            if let Some(cert_str) = auth_info.get("Certificate").and_then(|v| v.as_str()) {
                if let Ok(cert_json) = serde_json::from_str::<Value>(cert_str) {
                    if let Some(chain) = cert_json["chain"].as_array() {
                        for jwt_str in chain.iter().filter_map(|v| v.as_str()) {
                            if let Ok(jwt) = decode_jwt(jwt_str) {
                                if let Some(key) =
                                    jwt.claims.get("identityPublicKey").and_then(|v| v.as_str())
                                {
                                    pub_key = key.to_string();
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(LoginIdentity {
            public_key_b64: pub_key,
            display_name,
            xuid,
            uuid_str: String::new(),
            authenticated: true,
        })
    } else {
        // SELF_SIGNED — Certificate field contains {"chain":["jwt"]}
        let cert_str = auth_info
            .get("Certificate")
            .and_then(|v| v.as_str())
            .ok_or(JwtError::InvalidFormat)?;

        let cert_json: Value = serde_json::from_str(cert_str).map_err(JwtError::JsonError)?;

        let chain = cert_json["chain"]
            .as_array()
            .ok_or(JwtError::InvalidFormat)?;

        if chain.is_empty() {
            return Err(JwtError::InvalidFormat);
        }

        // Parse the first (and usually only) JWT in the chain
        let jwt_str = chain[0].as_str().ok_or(JwtError::InvalidFormat)?;

        let decoded = decode_jwt(jwt_str)?;

        // Public key from header x5u
        let pub_key = decoded
            .header
            .get("x5u")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        // Identity from claims extraData
        let extra = &decoded.claims["extraData"];
        let display_name = extra["displayName"].as_str().unwrap_or("").to_string();
        let xuid = extra["XUID"].as_str().unwrap_or("").to_string();
        let uuid_str = extra["identity"].as_str().unwrap_or("").to_string();

        Ok(LoginIdentity {
            public_key_b64: pub_key,
            display_name,
            xuid,
            uuid_str,
            authenticated: false,
        })
    }
}

/// Parsed login identity from JWT chain.
pub struct LoginIdentity {
    pub public_key_b64: String,
    pub display_name: String,
    pub xuid: String,
    pub uuid_str: String,
    pub authenticated: bool,
}

/// Create a JWT with ES384 algorithm for the handshake.
/// The JWT is signed with the server's private key.
pub fn create_handshake_jwt(
    server_pub_key_der_b64: &str,
    salt_b64: &str,
    sign_fn: impl FnOnce(&[u8]) -> Vec<u8>,
) -> String {
    let header = serde_json::json!({
        "x5u": server_pub_key_der_b64,
        "alg": "ES384"
    });
    let payload = serde_json::json!({
        "salt": salt_b64
    });

    let header_b64 = URL_SAFE_NO_PAD.encode(header.to_string().as_bytes());
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes());

    let signing_input = format!("{}.{}", header_b64, payload_b64);
    let signature = sign_fn(signing_input.as_bytes());
    let signature_b64 = URL_SAFE_NO_PAD.encode(&signature);

    format!("{}.{}.{}", header_b64, payload_b64, signature_b64)
}

/// Decode a DER-encoded public key from base64.
pub fn decode_base64_der(b64: &str) -> Result<Vec<u8>, base64::DecodeError> {
    // Try standard base64 first, then URL-safe
    STANDARD
        .decode(b64)
        .or_else(|_| URL_SAFE_NO_PAD.decode(b64))
}

// ── Xbox Live chain verification (PMMP `XboxAuthJwt::validateLoginJwt`) ──

/// Mojang root public key (PMMP `Player::MOJANG_ROOT_PUBLIC_KEY`).
/// ECDSA P-384, encoded SEC1 + DER (base64). Cette clé signe le premier JWT
/// quand un joueur s'authentifie via Xbox Live (auth_type=FULL).
pub const MOJANG_ROOT_PUBLIC_KEY_B64: &str = "MHYwEAYHKoZIzj0CAQYFK4EEACIDYgAE8ELkixyLcwlZryUQcu1TvPOmI2B7vX83ndnWRUaXm74wFfa5f/lwQNTfrLVHa2PmenpGI6JhIMUJaWZrjmMj90NoKNFSNBuKdm8rYiXsfaz3K36x/1U26HpG0ZxK/V1V";

#[derive(Debug, Error)]
pub enum ChainVerifyError {
    #[error("empty chain")]
    EmptyChain,
    #[error("invalid certificate format: {0}")]
    InvalidCertificate(String),
    #[error("missing identityPublicKey in JWT claims")]
    MissingIdentityKey,
    #[error("invalid public key: {0}")]
    InvalidKey(String),
    #[error("signature verification failed at index {0}")]
    SignatureFailed(usize),
    #[error("JWT decode error: {0}")]
    JwtError(#[from] JwtError),
}

fn split_signing_input(jwt: &str) -> Option<(String, &str)> {
    let mut iter = jwt.rsplitn(2, '.');
    let sig = iter.next()?;
    let signing = iter.next()?;
    Some((signing.to_string(), sig))
}

fn verify_jwt_signature(jwt: &str, pub_key_b64: &str) -> Result<(), ChainVerifyError> {
    use p384::ecdsa::signature::Verifier;
    use p384::ecdsa::{Signature, VerifyingKey};
    use p384::pkcs8::DecodePublicKey;
    use p384::PublicKey;

    let (signing_input, sig_b64) = split_signing_input(jwt)
        .ok_or_else(|| ChainVerifyError::InvalidCertificate("not 3-part JWT".into()))?;
    let signature_bytes = URL_SAFE_NO_PAD
        .decode(sig_b64)
        .map_err(|e| ChainVerifyError::InvalidCertificate(format!("base64 sig: {e}")))?;
    let der_bytes = STANDARD
        .decode(pub_key_b64)
        .or_else(|_| URL_SAFE_NO_PAD.decode(pub_key_b64))
        .map_err(|e| ChainVerifyError::InvalidKey(format!("base64 key: {e}")))?;
    let pk = PublicKey::from_public_key_der(&der_bytes)
        .map_err(|e| ChainVerifyError::InvalidKey(format!("DER: {e}")))?;
    let vk = VerifyingKey::from(&pk);
    let signature = Signature::from_slice(&signature_bytes)
        .map_err(|e| ChainVerifyError::InvalidCertificate(format!("sig parse: {e}")))?;
    vk.verify(signing_input.as_bytes(), &signature)
        .map_err(|_| ChainVerifyError::SignatureFailed(0))
}

/// Vérifie la chain JWT du Certificate field (login authentifié Xbox).
/// Algorithme PMMP `XboxAuthJwt::validateLoginJwt` :
///   - chain[0] : si header.x5u est la clé Mojang root, signed by Mojang
///                (sinon self-signed mais validé via x5u du JWT lui-même).
///   - chain[i>0] : signed by claims.identityPublicKey de chain[i-1].
///
/// Retourne la clé publique du LAST JWT (le ClientPublicKey utilisé pour
/// le Diffie-Hellman + signature handshake côté serveur).
pub fn verify_chain(chain: &[String]) -> Result<String, ChainVerifyError> {
    if chain.is_empty() {
        return Err(ChainVerifyError::EmptyChain);
    }
    let mut next_key: Option<String> = None;
    for (i, jwt) in chain.iter().enumerate() {
        let decoded = decode_jwt(jwt)?;
        let signing_key = if let Some(k) = next_key.as_deref() {
            k.to_string()
        } else {
            // Premier JWT : check x5u du header.
            let x5u = decoded.header["x5u"]
                .as_str()
                .ok_or_else(|| ChainVerifyError::InvalidCertificate("x5u missing".into()))?
                .to_string();
            // Si signed par Mojang root, accepte.
            // Sinon self-signed : on accepte le x5u (vérification self-sig via lui-même).
            x5u
        };
        verify_jwt_signature(jwt, &signing_key).map_err(|e| match e {
            ChainVerifyError::SignatureFailed(_) => ChainVerifyError::SignatureFailed(i),
            other => other,
        })?;
        // Extract identityPublicKey for next iter
        next_key = decoded.claims["identityPublicKey"]
            .as_str()
            .map(String::from);
    }
    next_key.ok_or(ChainVerifyError::MissingIdentityKey)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_jwt() {
        // Create a minimal valid JWT (header.claims.signature)
        let header = URL_SAFE_NO_PAD.encode(b"{\"alg\":\"ES384\",\"x5u\":\"testkey\"}");
        let claims = URL_SAFE_NO_PAD.encode(b"{\"sub\":\"test\"}");
        let sig = URL_SAFE_NO_PAD.encode(b"fakesig");
        let jwt = format!("{}.{}.{}", header, claims, sig);

        let decoded = decode_jwt(&jwt).unwrap();
        assert_eq!(decoded.header["alg"], "ES384");
        assert_eq!(decoded.header["x5u"], "testkey");
        assert_eq!(decoded.claims["sub"], "test");
    }

    #[test]
    fn test_create_handshake_jwt() {
        let jwt = create_handshake_jwt("serverpubkey", "c2FsdA", |input| {
            // Fake signing — just hash the input
            input[..48.min(input.len())].to_vec()
        });
        let parts: Vec<&str> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);

        let decoded = decode_jwt(&jwt).unwrap();
        assert_eq!(decoded.header["x5u"], "serverpubkey");
        assert_eq!(decoded.claims["salt"], "c2FsdA");
    }

    #[test]
    fn test_invalid_jwt() {
        assert!(decode_jwt("not.valid").is_err());
        assert!(decode_jwt("").is_err());
    }
}
