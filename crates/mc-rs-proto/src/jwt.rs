//! Simple JWT parser for Bedrock login (no signature verification).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde::Deserialize;

use crate::error::ProtoError;

/// JWT header fields we care about.
#[derive(Debug, Clone, Deserialize)]
pub struct JwtHeader {
    pub alg: String,
    pub x5u: Option<String>,
}

/// Identity data from the last JWT in the chain (`extraData` field).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityData {
    #[serde(rename = "XUID")]
    pub xuid: Option<String>,
    pub identity: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
}

/// Claims from an identity JWT.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityClaims {
    #[serde(default)]
    pub extra_data: Option<IdentityData>,
    pub identity_public_key: Option<String>,
}

/// Extracted login data from the JWT chain.
#[derive(Debug, Clone)]
pub struct LoginData {
    pub xuid: String,
    pub identity: String,
    pub display_name: String,
    pub identity_public_key: String,
}

/// Decode a JWT without verifying the signature.
///
/// Returns the parsed header and the raw payload as a `serde_json::Value`.
pub fn decode_jwt_unverified(token: &str) -> Result<(JwtHeader, serde_json::Value), ProtoError> {
    let parts: Vec<&str> = token.splitn(3, '.').collect();
    if parts.len() != 3 {
        return Err(ProtoError::JwtDecode(format!(
            "expected 3 parts, got {}",
            parts.len()
        )));
    }

    let header_bytes = decode_base64url(parts[0])
        .map_err(|e| ProtoError::JwtDecode(format!("header base64: {e}")))?;
    let payload_bytes = decode_base64url(parts[1])
        .map_err(|e| ProtoError::JwtDecode(format!("payload base64: {e}")))?;

    let header: JwtHeader = serde_json::from_slice(&header_bytes)
        .map_err(|e| ProtoError::JsonParse(format!("JWT header: {e}")))?;
    let payload: serde_json::Value = serde_json::from_slice(&payload_bytes)
        .map_err(|e| ProtoError::JsonParse(format!("JWT payload: {e}")))?;

    Ok((header, payload))
}

/// Extract player identity from the JWT chain (last JWT with `extraData`).
pub fn extract_login_data(chain: &[String]) -> Result<LoginData, ProtoError> {
    if chain.is_empty() {
        return Err(ProtoError::InvalidLogin("empty JWT chain".into()));
    }

    for jwt_str in chain.iter().rev() {
        let (_, payload) = decode_jwt_unverified(jwt_str)?;
        if let Ok(claims) = serde_json::from_value::<IdentityClaims>(payload) {
            if let Some(extra) = claims.extra_data {
                return Ok(LoginData {
                    xuid: extra.xuid.unwrap_or_default(),
                    identity: extra.identity,
                    display_name: extra.display_name,
                    identity_public_key: claims.identity_public_key.unwrap_or_default(),
                });
            }
        }
    }

    Err(ProtoError::InvalidLogin(
        "no identity data found in JWT chain".into(),
    ))
}

/// Decode base64url (try without padding first, then with padding).
fn decode_base64url(input: &str) -> Result<Vec<u8>, base64::DecodeError> {
    URL_SAFE_NO_PAD
        .decode(input)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(input))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: encode a JSON value as a base64url JWT part (no padding).
    fn encode_part(value: &serde_json::Value) -> String {
        URL_SAFE_NO_PAD.encode(serde_json::to_vec(value).unwrap())
    }

    /// Helper: build a fake JWT from header + payload JSON values.
    fn make_jwt(header: &serde_json::Value, payload: &serde_json::Value) -> String {
        format!(
            "{}.{}.fake_signature",
            encode_part(header),
            encode_part(payload)
        )
    }

    fn sample_header() -> serde_json::Value {
        serde_json::json!({
            "alg": "ES384",
            "x5u": "some_key"
        })
    }

    fn sample_identity_payload() -> serde_json::Value {
        serde_json::json!({
            "extraData": {
                "XUID": "1234567890",
                "identity": "12345678-1234-1234-1234-123456789012",
                "displayName": "TestPlayer"
            },
            "identityPublicKey": "MHYwEAtest..."
        })
    }

    #[test]
    fn decode_valid_jwt() {
        let jwt = make_jwt(&sample_header(), &sample_identity_payload());
        let (header, payload) = decode_jwt_unverified(&jwt).unwrap();
        assert_eq!(header.alg, "ES384");
        assert_eq!(header.x5u.as_deref(), Some("some_key"));
        assert_eq!(
            payload["extraData"]["displayName"].as_str(),
            Some("TestPlayer")
        );
    }

    #[test]
    fn decode_jwt_missing_part() {
        assert!(decode_jwt_unverified("header.payload").is_err());
        assert!(decode_jwt_unverified("single").is_err());
    }

    #[test]
    fn decode_jwt_invalid_base64() {
        assert!(decode_jwt_unverified("!!!.!!!.!!!").is_err());
    }

    #[test]
    fn decode_jwt_invalid_json() {
        let not_json = URL_SAFE_NO_PAD.encode(b"not json");
        let jwt = format!("{not_json}.{not_json}.sig");
        assert!(decode_jwt_unverified(&jwt).is_err());
    }

    #[test]
    fn extract_login_data_valid_chain() {
        let jwt1 = make_jwt(
            &sample_header(),
            &serde_json::json!({
                "certificateAuthority": true,
                "identityPublicKey": "key1"
            }),
        );
        let jwt2 = make_jwt(
            &sample_header(),
            &serde_json::json!({
                "certificateAuthority": true,
                "identityPublicKey": "key2"
            }),
        );
        let jwt3 = make_jwt(&sample_header(), &sample_identity_payload());

        let chain = vec![jwt1, jwt2, jwt3];
        let data = extract_login_data(&chain).unwrap();
        assert_eq!(data.display_name, "TestPlayer");
        assert_eq!(data.xuid, "1234567890");
        assert_eq!(data.identity, "12345678-1234-1234-1234-123456789012");
        assert_eq!(data.identity_public_key, "MHYwEAtest...");
    }

    #[test]
    fn extract_login_data_no_extra_data() {
        let jwt = make_jwt(
            &sample_header(),
            &serde_json::json!({
                "identityPublicKey": "key"
            }),
        );
        assert!(extract_login_data(&[jwt]).is_err());
    }

    #[test]
    fn extract_login_data_empty_chain() {
        assert!(extract_login_data(&[]).is_err());
    }

    #[test]
    fn base64url_with_padding() {
        // Ensure padded base64url also works
        let data = b"test data!";
        let encoded_padded = base64::engine::general_purpose::URL_SAFE.encode(data);
        let result = decode_base64url(&encoded_padded).unwrap();
        assert_eq!(result, data);
    }

    #[test]
    fn extract_login_data_xuid_missing() {
        let jwt = make_jwt(
            &sample_header(),
            &serde_json::json!({
                "extraData": {
                    "identity": "uuid-here",
                    "displayName": "NoXuid"
                },
                "identityPublicKey": "key"
            }),
        );
        let data = extract_login_data(&[jwt]).unwrap();
        assert_eq!(data.display_name, "NoXuid");
        assert_eq!(data.xuid, ""); // defaults to empty
    }
}
