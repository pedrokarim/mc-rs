//! JWT HS256. Secret stocké en DB (table `meta`, clé `jwt_secret`) — auto-généré
//! au 1er boot. Les tokens incluent un `jti` pour permettre la révocation via
//! `tokens_blacklist` (logout).

use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::Role;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject = user id UUID
    pub sub: String,
    /// Username (snapshot, pour affichage)
    pub name: String,
    /// Role
    pub role: String,
    /// JWT ID (UUID) — cherchable dans tokens_blacklist
    pub jti: String,
    /// Issued at (unix sec)
    pub iat: i64,
    /// Expiration (unix sec)
    pub exp: i64,
}

#[derive(Clone)]
pub struct JwtCodec {
    encode: EncodingKey,
    decode: DecodingKey,
    validation: Validation,
    ttl_seconds: i64,
}

impl JwtCodec {
    pub fn new(secret: &[u8], ttl_seconds: i64) -> Self {
        let mut validation = Validation::default();
        validation.leeway = 5;
        Self {
            encode: EncodingKey::from_secret(secret),
            decode: DecodingKey::from_secret(secret),
            validation,
            ttl_seconds,
        }
    }

    pub fn issue(&self, user_id: Uuid, username: &str, role: Role) -> Result<(String, Claims)> {
        let now = chrono::Utc::now().timestamp();
        let claims = Claims {
            sub: user_id.to_string(),
            name: username.to_string(),
            role: role.as_str().to_string(),
            jti: Uuid::new_v4().to_string(),
            iat: now,
            exp: now + self.ttl_seconds,
        };
        let token = encode(&Header::default(), &claims, &self.encode)
            .map_err(|e| Error::Crypto(format!("jwt encode: {e}")))?;
        Ok((token, claims))
    }

    pub fn decode(&self, token: &str) -> Result<Claims> {
        let data = decode::<Claims>(token, &self.decode, &self.validation)
            .map_err(|e| Error::Crypto(format!("jwt decode: {e}")))?;
        Ok(data.claims)
    }
}

/// Génère ou récupère le secret JWT. Persisté hex dans `meta.jwt_secret`.
pub async fn load_or_init_secret(db: &dyn crate::db::WebDb) -> Result<[u8; 32]> {
    if let Some(hex) = db.get_meta("jwt_secret").await? {
        let bytes = hex_decode(&hex)
            .ok_or_else(|| Error::Db("jwt_secret meta is not valid hex".to_string()))?;
        if bytes.len() != 32 {
            return Err(Error::Db(format!(
                "jwt_secret length={}, expected 32",
                bytes.len()
            )));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        return Ok(arr);
    }

    let mut arr = [0u8; 32];
    use rand::RngCore;
    rand::thread_rng().fill_bytes(&mut arr);
    db.set_meta("jwt_secret", &hex_encode(&arr)).await?;
    tracing::info!("[webui] generated new JWT secret, persisted in DB");
    Ok(arr)
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    for chunk in s.as_bytes().chunks(2) {
        let hi = hex_digit(chunk[0])?;
        let lo = hex_digit(chunk[1])?;
        out.push((hi << 4) | lo);
    }
    Some(out)
}

fn hex_digit(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_roundtrip() {
        let codec = JwtCodec::new(b"a-test-secret", 3600);
        let id = Uuid::new_v4();
        let (token, issued) = codec.issue(id, "bob", Role::Admin).unwrap();
        let decoded = codec.decode(&token).unwrap();
        assert_eq!(decoded.sub, issued.sub);
        assert_eq!(decoded.name, "bob");
        assert_eq!(decoded.role, "admin");
    }

    #[test]
    fn hex_roundtrip() {
        let bytes = [0x00, 0xff, 0xab, 0x12];
        let s = hex_encode(&bytes);
        assert_eq!(s, "00ffab12");
        assert_eq!(hex_decode(&s).unwrap(), bytes);
    }
}
