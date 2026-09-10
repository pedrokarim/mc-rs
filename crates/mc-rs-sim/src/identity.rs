//! Identité d'un joueur simulé : keypair P-384 + chaîne JWT auto-signée.
//!
//! Le serveur mc-rs accepte le mode `AuthenticationType = 1` (SELF_SIGNED,
//! offline) : pas besoin d'authentification Xbox Live. On génère une paire de
//! clés, on forge une chaîne d'identité auto-signée, et le serveur nous traite
//! comme un joueur offline (`auth=offline`). C'est exactement ce qu'il faut
//! pour simuler une population sans compte Microsoft.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Value};

use mc_rs_crypto::ecdh::ServerKeyPair;

/// Identité complète d'un joueur simulé.
pub struct SimIdentity {
    /// Paire de clés P-384 : sert à signer la chaîne JWT ET au Diffie-Hellman
    /// du chiffrement (même clé que le `x5u` annoncé au serveur).
    pub keypair: ServerKeyPair,
    pub uuid: String,
    pub display_name: String,
    pub xuid: String,
}

impl SimIdentity {
    /// Génère une identité fraîche pour `display_name`.
    pub fn generate(display_name: &str) -> Self {
        let keypair = ServerKeyPair::generate();
        let uuid = uuid::Uuid::new_v4().to_string();
        // xuid synthétique unique (numérique, comme un vrai XUID) pour distinguer
        // les bots dans les registres serveur sans collision.
        let xuid = format!("{}", rand::random::<u64>() >> 1);
        Self {
            keypair,
            uuid,
            display_name: display_name.to_string(),
            xuid,
        }
    }

    /// Clé publique DER encodée base64 (valeur du champ `x5u`).
    pub fn public_key_b64(&self) -> String {
        self.keypair.public_key_base64()
    }

    /// `chain_data` (authInfoJson) à mettre dans le paquet Login.
    ///
    /// Format attendu par `jwt::extract_login_identity` (branche SELF_SIGNED) :
    /// `{"AuthenticationType":1,"Certificate":"{\"chain\":[\"<jwt>\"]}"}`.
    pub fn auth_info_json(&self) -> String {
        let cert = json!({ "chain": [self.chain_jwt()] }).to_string();
        json!({
            "AuthenticationType": 1,
            "Certificate": cert,
        })
        .to_string()
    }

    /// JWT `client_data` (données client + skin). Minimal mais plausible ;
    /// le serveur tombe sur un skin par défaut si le parse échoue.
    pub fn client_data_jwt(&self) -> String {
        let pub_b64 = self.public_key_b64();
        let header = json!({ "alg": "ES384", "x5u": pub_b64 });
        let claims = json!({
            "GameVersion": mc_rs_proto::MINECRAFT_VERSION,
            "DeviceOS": 7,
            "DeviceModel": "mc-rs-sim",
            "DeviceId": self.uuid,
            "ClientRandomId": rand::random::<i64>(),
            "LanguageCode": "en_US",
            "CurrentInputMode": 1,
            "DefaultInputMode": 1,
            "UIProfile": 0,
            "GuiScale": 0,
            "PlayerName": self.display_name,
            "ServerAddress": "127.0.0.1:19132",
            "ThirdPartyName": self.display_name,
            "ThirdPartyNameOnly": false,
            "SelfSignedId": self.uuid,
            "SkinId": format!("{}-sim", self.uuid),
            "SkinResourcePatch": URL_SAFE_NO_PAD
                .encode(br#"{"geometry":{"default":"geometry.humanoid.custom"}}"#),
            "SkinData": "",
            "SkinImageWidth": 0,
            "SkinImageHeight": 0,
            "SkinGeometryData": "",
            "SkinGeometryDataEngineVersion": "",
            "AnimatedImageData": [],
            "PersonaSkin": false,
            "PremiumSkin": false,
            "CapeData": "",
            "CapeId": "",
            "CapeImageWidth": 0,
            "CapeImageHeight": 0,
            "CapeOnClassicSkin": false,
            "ArmSize": "wide",
            "SkinColor": "#0",
            "PersonaPieces": [],
            "PieceTintColors": [],
        });
        self.sign_jwt(&header, &claims)
    }

    /// JWT de la chaîne d'identité (auto-signé).
    fn chain_jwt(&self) -> String {
        let pub_b64 = self.public_key_b64();
        let header = json!({ "alg": "ES384", "x5u": pub_b64 });
        let claims = json!({
            "identityPublicKey": pub_b64,
            "certificateAuthority": true,
            "extraData": {
                "displayName": self.display_name,
                "identity": self.uuid,
                "XUID": self.xuid,
                "titleId": "0",
            },
        });
        self.sign_jwt(&header, &claims)
    }

    /// Signe `header.claims` en ES384 (P-384 + SHA-384), format JWT compact.
    fn sign_jwt(&self, header: &Value, claims: &Value) -> String {
        let h = URL_SAFE_NO_PAD.encode(header.to_string());
        let c = URL_SAFE_NO_PAD.encode(claims.to_string());
        let signing_input = format!("{h}.{c}");
        let sig = self.keypair.sign(signing_input.as_bytes());
        let s = URL_SAFE_NO_PAD.encode(sig);
        format!("{h}.{c}.{s}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mc_rs_crypto::jwt;

    #[test]
    fn test_auth_info_parses_as_offline_identity() {
        let id = SimIdentity::generate("TestBot");
        let auth = id.auth_info_json();
        let parsed = jwt::extract_login_identity(&auth).expect("parse identity");
        assert!(!parsed.authenticated); // SELF_SIGNED
        assert_eq!(parsed.display_name, "TestBot");
        assert_eq!(parsed.public_key_b64, id.public_key_b64());
        assert_eq!(parsed.xuid, id.xuid);
    }

    #[test]
    fn test_chain_jwt_is_valid_and_self_signed() {
        let id = SimIdentity::generate("Sig");
        let auth: Value = serde_json::from_str(&id.auth_info_json()).unwrap();
        let cert: Value = serde_json::from_str(auth["Certificate"].as_str().unwrap()).unwrap();
        let chain: Vec<String> = cert["chain"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        // La chaîne auto-signée doit vérifier sa propre signature.
        let key = jwt::verify_chain(&chain).expect("self-signed chain verifies");
        assert_eq!(key, id.public_key_b64());
    }
}
