//! Kick / disconnect reasons — port PMMP kick messages + bedrock disconnect reasons.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectReason {
    /// Kicked par un op ou plugin.
    KickedByAdmin,
    /// Raison générique.
    Unknown,
    /// Timeout raknet.
    Timeout,
    /// Duplicate login (même xuid déjà connecté).
    DuplicateLogin,
    /// Version incompatible.
    OutdatedClient,
    OutdatedServer,
    /// Permission refusée.
    NotWhitelisted,
    /// Banni.
    Banned,
    /// Ip banned.
    BannedIp,
    /// Serveur plein.
    ServerFull,
    /// Shutdown serveur.
    ServerShutdown,
    /// Transfer vers un autre serveur.
    Transferred,
    /// Crash client.
    BadPacket,
    /// Authentification invalide.
    InvalidAuth,
}

impl DisconnectReason {
    pub fn default_message(&self) -> &'static str {
        match self {
            Self::KickedByAdmin => "Kicked by operator.",
            Self::Unknown => "Disconnected.",
            Self::Timeout => "Timed out.",
            Self::DuplicateLogin => "Logged in from another location.",
            Self::OutdatedClient => "Your client is outdated. Please update to the latest version.",
            Self::OutdatedServer => "This server is outdated. Please contact the administrator.",
            Self::NotWhitelisted => "You are not whitelisted on this server.",
            Self::Banned => "You are banned from this server.",
            Self::BannedIp => "Your IP address is banned from this server.",
            Self::ServerFull => "Server is full.",
            Self::ServerShutdown => "Server is shutting down.",
            Self::Transferred => "Transferred to another server.",
            Self::BadPacket => "Bad packet received.",
            Self::InvalidAuth => "Authentication failed.",
        }
    }
}

pub fn kick_player(message: impl Into<String>) -> DisconnectPayload {
    DisconnectPayload {
        reason: DisconnectReason::KickedByAdmin,
        message: message.into(),
        hide_disconnect_screen: false,
    }
}

#[derive(Debug, Clone)]
pub struct DisconnectPayload {
    pub reason: DisconnectReason,
    pub message: String,
    pub hide_disconnect_screen: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kick_has_message() {
        let p = kick_player("You broke the rules");
        assert_eq!(p.reason, DisconnectReason::KickedByAdmin);
        assert_eq!(p.message, "You broke the rules");
    }
}
