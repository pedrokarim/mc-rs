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

// ---------------------------------------------------------------------------
// Client data (skin, device info) from the client_data JWT
// ---------------------------------------------------------------------------

/// A skin or cape image (raw RGBA pixels).
#[derive(Debug, Clone)]
pub struct SkinImage {
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
}

impl SkinImage {
    pub fn empty() -> Self {
        Self {
            width: 0,
            height: 0,
            data: Vec::new(),
        }
    }
}

/// Client data extracted from the `client_data` JWT sent during login.
/// Contains skin information, device details, and other client-specific data.
#[derive(Debug, Clone)]
pub struct ClientData {
    pub skin_id: String,
    pub skin_image: SkinImage,
    pub cape_id: String,
    pub cape_image: SkinImage,
    pub skin_resource_patch: String,
    pub skin_geometry_data: String,
    pub skin_color: String,
    pub arm_size: String,
    pub persona_skin: bool,
    pub device_id: String,
    pub device_os: i32,
    pub play_fab_id: String,
}

impl Default for ClientData {
    fn default() -> Self {
        Self {
            skin_id: String::new(),
            skin_image: SkinImage::empty(),
            cape_id: String::new(),
            cape_image: SkinImage::empty(),
            skin_resource_patch: String::new(),
            skin_geometry_data: String::new(),
            skin_color: String::new(),
            arm_size: "wide".to_string(),
            persona_skin: false,
            device_id: String::new(),
            device_os: 0,
            play_fab_id: String::new(),
        }
    }
}

/// Raw deserialization target for client_data JWT payload.
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ClientDataRaw {
    #[serde(default)]
    skin_id: String,
    #[serde(default)]
    skin_data: String,
    #[serde(default)]
    skin_image_width: u32,
    #[serde(default)]
    skin_image_height: u32,
    #[serde(default)]
    cape_id: String,
    #[serde(default)]
    cape_data: String,
    #[serde(default)]
    cape_image_width: u32,
    #[serde(default)]
    cape_image_height: u32,
    #[serde(default)]
    skin_resource_patch: String,
    #[serde(default)]
    skin_geometry_data: String,
    #[serde(default)]
    skin_color: String,
    #[serde(default)]
    arm_size: String,
    #[serde(default)]
    persona_skin: bool,
    #[serde(default, rename = "DeviceId")]
    device_id: String,
    #[serde(default, rename = "DeviceOS")]
    device_os: i32,
    #[serde(default, rename = "PlayFabId")]
    play_fab_id: String,
}

/// Extract client data (skin, device info) from the client_data JWT.
pub fn extract_client_data(client_data_jwt: &str) -> Result<ClientData, ProtoError> {
    let (_, payload) = decode_jwt_unverified(client_data_jwt)?;
    let raw: ClientDataRaw = serde_json::from_value(payload)
        .map_err(|e| ProtoError::JsonParse(format!("client_data: {e}")))?;

    let skin_data = decode_base64_standard(&raw.skin_data).unwrap_or_default();
    let cape_data = decode_base64_standard(&raw.cape_data).unwrap_or_default();

    Ok(ClientData {
        skin_id: raw.skin_id,
        skin_image: SkinImage {
            width: raw.skin_image_width,
            height: raw.skin_image_height,
            data: skin_data,
        },
        cape_id: raw.cape_id,
        cape_image: SkinImage {
            width: raw.cape_image_width,
            height: raw.cape_image_height,
            data: cape_data,
        },
        skin_resource_patch: raw.skin_resource_patch,
        skin_geometry_data: raw.skin_geometry_data,
        skin_color: raw.skin_color,
        arm_size: if raw.arm_size.is_empty() {
            "wide".to_string()
        } else {
            raw.arm_size
        },
        persona_skin: raw.persona_skin,
        device_id: raw.device_id,
        device_os: raw.device_os,
        play_fab_id: raw.play_fab_id,
    })
}

/// Decode base64url (try without padding first, then with padding).
fn decode_base64url(input: &str) -> Result<Vec<u8>, base64::DecodeError> {
    URL_SAFE_NO_PAD
        .decode(input)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(input))
}

/// Decode standard base64 (skin data uses standard encoding, not URL-safe).
fn decode_base64_standard(input: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
    STANDARD_NO_PAD
        .decode(input)
        .or_else(|_| STANDARD.decode(input))
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
    fn extract_client_data_basic() {
        let payload = serde_json::json!({
            "SkinId": "standard.steve",
            "SkinData": base64::engine::general_purpose::STANDARD.encode([0u8; 64 * 32 * 4]),
            "SkinImageWidth": 64,
            "SkinImageHeight": 32,
            "CapeId": "",
            "CapeData": "",
            "CapeImageWidth": 0,
            "CapeImageHeight": 0,
            "SkinResourcePatch": "{}",
            "SkinGeometryData": "{}",
            "SkinColor": "#0",
            "ArmSize": "wide",
            "PersonaSkin": false,
            "DeviceId": "device123",
            "DeviceOS": 7,
            "PlayFabId": "pfab123"
        });
        let jwt = make_jwt(&sample_header(), &payload);
        let data = extract_client_data(&jwt).unwrap();
        assert_eq!(data.skin_id, "standard.steve");
        assert_eq!(data.skin_image.width, 64);
        assert_eq!(data.skin_image.height, 32);
        assert_eq!(data.skin_image.data.len(), 64 * 32 * 4);
        assert_eq!(data.device_os, 7);
        assert_eq!(data.device_id, "device123");
        assert_eq!(data.arm_size, "wide");
    }

    #[test]
    fn extract_client_data_defaults() {
        // Minimal payload — all fields should default gracefully
        let payload = serde_json::json!({});
        let jwt = make_jwt(&sample_header(), &payload);
        let data = extract_client_data(&jwt).unwrap();
        assert_eq!(data.skin_id, "");
        assert_eq!(data.skin_image.data.len(), 0);
        assert_eq!(data.device_os, 0);
        assert_eq!(data.arm_size, "wide"); // default fallback
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
