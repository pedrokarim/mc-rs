//! Skin data — port PMMP `src/entity/Skin.php` + `SkinData` (LoginPacket).

#[derive(Debug, Clone)]
pub struct Skin {
    pub skin_id: String,
    pub skin_resource_patch: String,
    pub skin_data: Vec<u8>, // raw RGBA bytes
    pub skin_width: u32,
    pub skin_height: u32,
    pub cape_id: String,
    pub cape_data: Vec<u8>,
    pub cape_width: u32,
    pub cape_height: u32,
    pub geometry_data: String, // JSON
    pub animation_data: String,
    pub premium: bool,
    pub persona: bool,
    pub cape_on_classic: bool,
    pub arm_size: String,
    pub skin_color: String,
    pub animations: Vec<SkinAnimation>,
    /// Claim `ProfileHash` du login (protocole 2168), renvoyé tel quel dans
    /// le SerializedSkin.
    pub profile_hash: String,
    /// Claim `PlayFabId` du login, renvoyé tel quel dans le SerializedSkin.
    pub play_fab_id: String,
    /// Claim `SkinGeometryDataEngineVersion` (base64 d'une version) relayé
    /// comme PMMP `ClientDataToSkinDataHelper` et BDS ; vide si absent.
    pub geometry_data_engine_version: String,
}

/// Une image RGBA de `width` × `height` pixels occupe exactement
/// `width * height * 4` octets (image vide comprise : 0 × 0).
fn image_size_matches(width: u32, height: u32, data: &[u8]) -> bool {
    (width as u64) * (height as u64) * 4 == data.len() as u64
}

#[derive(Debug, Clone)]
pub struct SkinAnimation {
    pub image_width: u32,
    pub image_height: u32,
    pub image_data: Vec<u8>,
    pub anim_type: SkinAnimationType,
    pub frames: f32,
    pub expression_type: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinAnimationType {
    Head = 1,
    Body32x32 = 2,
    Body128x128 = 3,
}

impl Skin {
    /// Skin minimal par défaut (Steve) utilisé si aucun skin n'est reçu.
    /// PMMP `SkinInfo::DEFAULT`.
    pub fn default_steve() -> Self {
        Self {
            skin_id: "Standard_Custom".into(),
            skin_resource_patch: "{\"geometry\":{\"default\":\"geometry.humanoid.custom\"}}".into(),
            skin_data: vec![0; 64 * 64 * 4],
            skin_width: 64,
            skin_height: 64,
            cape_id: String::new(),
            cape_data: Vec::new(),
            cape_width: 0,
            cape_height: 0,
            geometry_data: String::new(),
            animation_data: String::new(),
            premium: false,
            persona: false,
            cape_on_classic: false,
            arm_size: "wide".into(),
            skin_color: "#0".into(),
            animations: Vec::new(),
            profile_hash: String::new(),
            play_fab_id: String::new(),
            geometry_data_engine_version: String::new(),
        }
    }

    /// Image du skin présente et de taille cohérente (largeur × hauteur × 4
    /// octets RGBA), comme l'exige gophertunnel `Skin.validate`.
    pub fn is_valid(&self) -> bool {
        self.skin_width > 0
            && self.skin_height > 0
            && image_size_matches(self.skin_width, self.skin_height, &self.skin_data)
    }

    /// Parse une `Skin` depuis les claims JSON du `client_data_jwt` envoyé par
    /// le client à la phase Login (réf PMMP `LoginPacketHandler::handleSkinData`).
    /// Tous les champs absents tombent sur le défaut Steve. Retourne `None`
    /// uniquement si on ne trouve aucun champ skin du tout.
    pub fn from_client_data(claims: &serde_json::Value) -> Option<Self> {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD;
        let s = |k: &str| claims.get(k).and_then(|v| v.as_str()).unwrap_or("");
        let u = |k: &str| claims.get(k).and_then(|v| v.as_u64()).unwrap_or(0) as u32;
        let b = |k: &str| claims.get(k).and_then(|v| v.as_bool()).unwrap_or(false);
        let bin = |k: &str| b64.decode(s(k)).unwrap_or_default();

        let skin_id = s("SkinId").to_string();
        if skin_id.is_empty() && s("SkinResourcePatch").is_empty() && s("SkinData").is_empty() {
            return None;
        }

        Some(Self {
            skin_id,
            // SkinResourcePatch est b64 d'un JSON `{"geometry":{"default":"..."}}`
            skin_resource_patch: String::from_utf8(bin("SkinResourcePatch")).unwrap_or_else(|_| {
                "{\"geometry\":{\"default\":\"geometry.humanoid.custom\"}}".into()
            }),
            skin_data: bin("SkinData"),
            skin_width: u("SkinImageWidth"),
            skin_height: u("SkinImageHeight"),
            cape_id: s("CapeId").to_string(),
            cape_data: bin("CapeData"),
            cape_width: u("CapeImageWidth"),
            cape_height: u("CapeImageHeight"),
            geometry_data: String::from_utf8(bin("SkinGeometryData")).unwrap_or_default(),
            animation_data: String::from_utf8(bin("SkinAnimationData")).unwrap_or_default(),
            premium: b("PremiumSkin"),
            persona: b("PersonaSkin"),
            cape_on_classic: b("CapeOnClassicSkin"),
            arm_size: s("ArmSize").to_string(),
            skin_color: s("SkinColor").to_string(),
            animations: Vec::new(),
            profile_hash: s("ProfileHash").to_string(),
            play_fab_id: s("PlayFabId").to_string(),
            geometry_data_engine_version: String::from_utf8(bin("SkinGeometryDataEngineVersion"))
                .unwrap_or_default(),
        })
    }

    /// Convertit en `SerializedSkin` wire-format (mc-rs-proto). Tombe sur le
    /// défaut Steve si la skin n'a pas de données valides.
    pub fn to_serialized(&self) -> mc_rs_proto::packets::player::SerializedSkin {
        if !self.is_valid() {
            return mc_rs_proto::packets::player::SerializedSkin::default();
        }
        // Une cape de taille incohérente ferait rejeter tout le skin : on la
        // retire plutôt que d'envoyer des dimensions fausses.
        let cape_ok = image_size_matches(self.cape_width, self.cape_height, &self.cape_data);
        mc_rs_proto::packets::player::SerializedSkin {
            skin_id: self.skin_id.clone(),
            // PlayFabId du claim de login, comme dragonfly `parseSkin`.
            play_fab_id: self.play_fab_id.clone(),
            skin_resource_patch: self.skin_resource_patch.clone(),
            skin_width: self.skin_width,
            skin_height: self.skin_height,
            skin_data: self.skin_data.clone(),
            cape_width: if cape_ok { self.cape_width } else { 0 },
            cape_height: if cape_ok { self.cape_height } else { 0 },
            cape_data: if cape_ok {
                self.cape_data.clone()
            } else {
                Vec::new()
            },
            geometry_data: self.geometry_data.clone(),
            // Vide : `write_serialized_skin` envoie `MINECRAFT_VERSION`.
            geometry_data_engine_version: self.geometry_data_engine_version.clone(),
            animation_data: self.animation_data.clone(),
            cape_id: self.cape_id.clone(),
            full_skin_id: format!("{}{}", self.skin_id, self.cape_id),
            arm_size: self.arm_size.clone(),
            skin_color: self.skin_color.clone(),
            premium: self.premium,
            persona: self.persona,
            persona_cape_on_classic: self.cape_on_classic,
            primary_user: true,
            profile_hash: self.profile_hash.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_steve_is_valid() {
        let s = Skin::default_steve();
        assert!(s.is_valid());
        assert_eq!(s.skin_width, 64);
    }
}
