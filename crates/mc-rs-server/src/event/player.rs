//! Events joueur — port sélectif de `.reference/PocketMine-MP/src/event/player/*`.

use super::Event;
use crate::cancellable_event;
use std::net::SocketAddr;

/// Port de `PlayerJoinEvent.php`. Fired après que le joueur soit pleinement
/// in-game. PMMP porte un message de join éditable.
pub struct PlayerJoinEvent {
    pub player_addr: SocketAddr,
    pub display_name: String,
    pub xuid: String,
    pub entity_runtime_id: u64,
    pub position: [f32; 3],
    pub gamemode: i32,
    /// Message broadcast à tous. Par défaut `"{name} joined the game"`.
    pub join_message: String,
}
impl Event for PlayerJoinEvent {}

/// Port de `PlayerQuitEvent.php`.
pub struct PlayerQuitEvent {
    pub player_addr: SocketAddr,
    pub display_name: String,
    pub xuid: String,
    pub entity_runtime_id: u64,
    pub quit_message: String,
    /// PMMP `getQuitReason()`. « Client Disconnect », « Kicked », « Server Stopped », etc.
    pub quit_reason: String,
}
impl Event for PlayerQuitEvent {}

/// Port de `PlayerChatEvent.php`. Cancellable : si annulé, le message n'est pas
/// broadcast. Éditable : un plugin peut modifier le format et les destinataires.
pub struct PlayerChatEvent {
    pub player_addr: SocketAddr,
    pub sender_name: String,
    pub message: String,
    pub format: String,
    pub cancelled: bool,
}
impl Event for PlayerChatEvent {}
cancellable_event!(PlayerChatEvent);

/// Port simplifié de `PlayerMoveEvent.php`.
pub struct PlayerMoveEvent {
    pub player_addr: SocketAddr,
    pub from: [f32; 3],
    pub to: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub cancelled: bool,
}
impl Event for PlayerMoveEvent {}
cancellable_event!(PlayerMoveEvent);

/// Port de `PlayerDeathEvent.php`. Éditable : drops + death message + keep inventory.
pub struct PlayerDeathEvent {
    pub player_addr: SocketAddr,
    pub death_message: String,
    pub drops: Vec<mc_rs_proto::packets::player::ItemStack>,
    pub xp_drop: u32,
    pub keep_inventory: bool,
    pub keep_xp: bool,
}
impl Event for PlayerDeathEvent {}

/// Port de `PlayerRespawnEvent.php`.
pub struct PlayerRespawnEvent {
    pub player_addr: SocketAddr,
    pub respawn_position: [f32; 3],
}
impl Event for PlayerRespawnEvent {}

/// Port de `PlayerGameModeChangeEvent.php`.
pub struct PlayerGameModeChangeEvent {
    pub player_addr: SocketAddr,
    pub new_gamemode: i32,
    pub cancelled: bool,
}
impl Event for PlayerGameModeChangeEvent {}
cancellable_event!(PlayerGameModeChangeEvent);

/// Port de `PlayerInteractEvent.php` — clic droit sur bloc / air / entité.
pub struct PlayerInteractEvent {
    pub player_addr: SocketAddr,
    pub action: InteractAction,
    pub block_position: Option<[i32; 3]>,
    pub block_face: Option<i32>,
    pub cancelled: bool,
}
impl Event for PlayerInteractEvent {}
cancellable_event!(PlayerInteractEvent);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractAction {
    LeftClickBlock,
    RightClickBlock,
    LeftClickAir,
    RightClickAir,
    PhysicalPressure, // buttons / pressure plates
}

/// Port de `PlayerDropItemEvent.php`. Cancellable — si annulé l'item n'est pas drop.
pub struct PlayerDropItemEvent {
    pub player_addr: SocketAddr,
    pub item: mc_rs_proto::packets::player::ItemStack,
    pub cancelled: bool,
}
impl Event for PlayerDropItemEvent {}
cancellable_event!(PlayerDropItemEvent);

/// Port de `PlayerItemHeldEvent.php`. Switch de slot hotbar.
pub struct PlayerItemHeldEvent {
    pub player_addr: SocketAddr,
    pub old_slot: u8,
    pub new_slot: u8,
    pub cancelled: bool,
}
impl Event for PlayerItemHeldEvent {}
cancellable_event!(PlayerItemHeldEvent);

/// Port de `PlayerExperienceChangeEvent.php`.
pub struct PlayerExperienceChangeEvent {
    pub player_addr: SocketAddr,
    pub old_level: i32,
    pub new_level: i32,
    pub old_progress: f32,
    pub new_progress: f32,
    pub cancelled: bool,
}
impl Event for PlayerExperienceChangeEvent {}
cancellable_event!(PlayerExperienceChangeEvent);
