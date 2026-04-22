//! Argon2id password hashing. Params OWASP 2024 : m=19456, t=2, p=1.
//!
//! `hash_password` renvoie un PHC string (format standard `$argon2id$...$hash`)
//! stocké tel quel en base. `verify_password` parse le PHC et vérifie.

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::{Algorithm, Argon2, Params, Version};

use crate::error::{Error, Result};

fn argon2() -> Argon2<'static> {
    let params = Params::new(19_456, 2, 1, None)
        .expect("argon2 params hardcoded and valid");
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
}

pub fn hash_password(plaintext: &str) -> Result<String> {
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon2()
        .hash_password(plaintext.as_bytes(), &salt)
        .map_err(|e| Error::Crypto(format!("argon2 hash: {e}")))?;
    Ok(hash.to_string())
}

pub fn verify_password(plaintext: &str, phc: &str) -> Result<bool> {
    let parsed = PasswordHash::new(phc).map_err(|e| Error::Crypto(format!("phc parse: {e}")))?;
    match argon2().verify_password(plaintext.as_bytes(), &parsed) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(Error::Crypto(format!("argon2 verify: {e}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_ok() {
        let phc = hash_password("hunter2").unwrap();
        assert!(verify_password("hunter2", &phc).unwrap());
        assert!(!verify_password("wrong", &phc).unwrap());
    }

    #[test]
    fn bad_phc_errors() {
        let err = verify_password("x", "not a phc").unwrap_err();
        assert!(matches!(err, Error::Crypto(_)));
    }
}
