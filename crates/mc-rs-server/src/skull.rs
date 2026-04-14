//! Skulls / heads — port PMMP `src/block/Skull.php`.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkullType {
    Skeleton = 0,
    WitherSkeleton = 1,
    Zombie = 2,
    Player = 3,
    Creeper = 4,
    Dragon = 5,
    Piglin = 6,
    CustomHead = 7,
}

impl SkullType {
    pub fn identifier(&self) -> &'static str {
        match self {
            Self::Skeleton => "minecraft:skeleton_skull",
            Self::WitherSkeleton => "minecraft:wither_skeleton_skull",
            Self::Zombie => "minecraft:zombie_head",
            Self::Player => "minecraft:player_head",
            Self::Creeper => "minecraft:creeper_head",
            Self::Dragon => "minecraft:dragon_head",
            Self::Piglin => "minecraft:piglin_head",
            Self::CustomHead => "minecraft:custom_head",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Skull {
    pub kind: SkullType,
    pub rotation: u8,            // 0-15
    pub player_name: Option<String>,
    pub player_uuid: Option<uuid::Uuid>,
    pub custom_skin_texture: Option<String>, // base64 URL for custom
}

impl Skull {
    pub fn new(kind: SkullType) -> Self {
        Self {
            kind,
            rotation: 0,
            player_name: None,
            player_uuid: None,
            custom_skin_texture: None,
        }
    }

    pub fn set_player(&mut self, name: impl Into<String>, uuid: uuid::Uuid) {
        self.kind = SkullType::Player;
        self.player_name = Some(name.into());
        self.player_uuid = Some(uuid);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skeleton_skull_id() {
        let s = Skull::new(SkullType::Skeleton);
        assert_eq!(s.kind.identifier(), "minecraft:skeleton_skull");
    }

    #[test]
    fn set_player_converts_to_player_head() {
        let mut s = Skull::new(SkullType::CustomHead);
        s.set_player("Alice", uuid::Uuid::nil());
        assert_eq!(s.kind, SkullType::Player);
        assert_eq!(s.player_name, Some("Alice".to_string()));
    }
}
