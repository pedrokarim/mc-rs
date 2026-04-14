//! Events serveur — port sélectif de `.reference/PocketMine-MP/src/event/server/*`.

use super::Event;
use crate::cancellable_event;
use std::net::SocketAddr;

/// Port de `ServerStartEvent.php` (inexistant en PMMP mais courant dans d'autres APIs ;
/// PMMP fait directement `PluginEnableEvent`/`PluginDisableEvent`). Ici on l'expose
/// pour permettre aux plugins de hook le démarrage complet.
pub struct ServerStartEvent {
    pub server_version: String,
    pub protocol_version: u32,
}
impl Event for ServerStartEvent {}

/// Port de `ServerCommandEvent.php`. Cancellable. Executée en console.
pub struct ServerCommandEvent {
    pub command: String,
    pub cancelled: bool,
}
impl Event for ServerCommandEvent {}
cancellable_event!(ServerCommandEvent);

/// Port de `CommandEvent.php` (joueur). Cancellable + éditable.
pub struct PlayerCommandPreprocessEvent {
    pub player_addr: SocketAddr,
    pub command: String,
    pub cancelled: bool,
}
impl Event for PlayerCommandPreprocessEvent {}
cancellable_event!(PlayerCommandPreprocessEvent);

/// Port de `DataPacketSendEvent.php`. Intercepte tous les paquets clientbound.
/// Cancellable — un plugin peut bloquer l'envoi.
pub struct DataPacketSendEvent {
    pub target_addr: SocketAddr,
    pub packet_id: u32,
    pub cancelled: bool,
}
impl Event for DataPacketSendEvent {}
cancellable_event!(DataPacketSendEvent);

/// Port de `DataPacketReceiveEvent.php`. Intercepte tous les paquets serverbound.
pub struct DataPacketReceiveEvent {
    pub source_addr: SocketAddr,
    pub packet_id: u32,
    pub cancelled: bool,
}
impl Event for DataPacketReceiveEvent {}
cancellable_event!(DataPacketReceiveEvent);
