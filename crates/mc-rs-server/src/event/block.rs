//! Events bloc — port sélectif de `.reference/PocketMine-MP/src/event/block/*`.

use super::Event;
use crate::cancellable_event;
use std::net::SocketAddr;

/// Port de `BlockBreakEvent.php`. Cancellable. Éditable : drops, item use, xp.
pub struct BlockBreakEvent {
    pub player_addr: SocketAddr,
    pub position: [i32; 3],
    pub old_block_runtime_id: u32,
    pub new_block_runtime_id: u32,
    pub drops: Vec<mc_rs_proto::packets::player::ItemStack>,
    pub xp_drop: u32,
    pub cancelled: bool,
}
impl Event for BlockBreakEvent {}
cancellable_event!(BlockBreakEvent);

/// Port de `BlockPlaceEvent.php`. Cancellable.
pub struct BlockPlaceEvent {
    pub player_addr: SocketAddr,
    pub position: [i32; 3],
    pub block_runtime_id: u32,
    pub replaced_block_runtime_id: u32,
    pub cancelled: bool,
}
impl Event for BlockPlaceEvent {}
cancellable_event!(BlockPlaceEvent);

/// Port de `BlockUpdateEvent.php`. Fired quand un bloc reçoit un update voisin.
pub struct BlockUpdateEvent {
    pub position: [i32; 3],
    pub block_runtime_id: u32,
    pub cancelled: bool,
}
impl Event for BlockUpdateEvent {}
cancellable_event!(BlockUpdateEvent);

/// Port de `BlockGrowEvent.php`.
pub struct BlockGrowEvent {
    pub position: [i32; 3],
    pub old_block_runtime_id: u32,
    pub new_block_runtime_id: u32,
    pub cancelled: bool,
}
impl Event for BlockGrowEvent {}
cancellable_event!(BlockGrowEvent);
