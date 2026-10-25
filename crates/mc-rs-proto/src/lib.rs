pub mod batch;
pub mod codec;
pub mod io;
pub mod packets;

/// Version du protocole réseau Bedrock supportée (Bedrock 1.26.50 / 1.26.51, r/26_u5).
///
/// Source : `.reference/bedrock-protocol-docs`, tags `v1.26.50` et `v1.26.51`
/// (`x-protocol-version` des schémas JSON ; pas de changelog Mojang pour 2193).
/// Seule source de vérité côté serveur et simulateur : ne jamais recopier la valeur en dur.
pub const CURRENT_PROTOCOL: i32 = 2193;

/// Version Minecraft correspondant à [`CURRENT_PROTOCOL`] (MOTD, StartGame, pile de packs).
pub const MINECRAFT_VERSION: &str = "1.26.51";
