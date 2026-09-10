pub mod batch;
pub mod codec;
pub mod io;
pub mod packets;

/// Version du protocole réseau Bedrock supportée (Bedrock 1.26.45, r/26_u4 hotfix 5).
///
/// Source : `.reference/bedrock-protocol-docs/legacy_changelogs/changelog_2169_08_18_26.md`.
/// Seule source de vérité côté serveur et simulateur : ne jamais recopier la valeur en dur.
pub const CURRENT_PROTOCOL: i32 = 2169;

/// Version Minecraft correspondant à [`CURRENT_PROTOCOL`] (MOTD, StartGame, pile de packs).
pub const MINECRAFT_VERSION: &str = "1.26.45";
