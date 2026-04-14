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
        }
    }

    pub fn is_valid(&self) -> bool {
        !self.skin_data.is_empty() && self.skin_width > 0 && self.skin_height > 0
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
