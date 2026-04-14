//! PlayerList packet utilities.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerListAction {
    Add,
    Remove,
}

#[derive(Debug, Clone)]
pub struct PlayerListRecord {
    pub uuid: String,
    pub entity_id: i64,
    pub username: String,
    pub xuid: String,
    pub platform_chat_id: String,
    pub build_platform: i32,
    pub skin: Option<SkinData>,
    pub is_teacher: bool,
    pub is_host: bool,
}

#[derive(Debug, Clone)]
pub struct SkinData {
    pub skin_id: String,
    pub geometry_data: Vec<u8>,
    pub skin_resource_patch: String,
    pub skin_data: Vec<u8>,
    pub cape_id: String,
    pub full_id: String,
    pub arm_size: String,
    pub skin_color: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_equality() {
        assert_eq!(PlayerListAction::Add, PlayerListAction::Add);
    }
}
