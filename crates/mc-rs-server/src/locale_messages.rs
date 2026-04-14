//! Standard translation keys.

pub const TRANSLATION_KEYS: &[&str] = &[
    // Chat
    "chat.type.text",
    "chat.type.announcement",
    "chat.type.emote",
    // Commands
    "commands.generic.usage",
    "commands.generic.permission",
    "commands.generic.notFound",
    "commands.generic.syntax",
    // Death
    "death.attack.player",
    "death.attack.player.item",
    "death.attack.arrow",
    "death.attack.arrow.item",
    "death.attack.lava",
    "death.attack.inFire",
    "death.attack.onFire",
    "death.attack.drown",
    "death.attack.starve",
    "death.attack.outOfWorld",
    "death.attack.inWall",
    "death.attack.explosion",
    "death.attack.generic",
    "death.attack.thorns",
    "death.attack.magic",
    "death.attack.wither",
    "death.attack.lightningBolt",
    "death.fell.accident.generic",
    // Player events
    "multiplayer.player.joined",
    "multiplayer.player.left",
    "multiplayer.disconnect.kicked",
    // Game events
    "gameMode.changed",
    "gameMode.survival",
    "gameMode.creative",
    "gameMode.adventure",
    "gameMode.spectator",
    // Advancements
    "chat.type.advancement.task",
    "chat.type.advancement.goal",
    "chat.type.advancement.challenge",
];

/// Get format string for translation key.
pub fn get_format(key: &str) -> Option<&'static str> {
    match key {
        "chat.type.text" => Some("<%1$s> %2$s"),
        "chat.type.announcement" => Some("[%1$s] %2$s"),
        "multiplayer.player.joined" => Some("%1$s joined the game"),
        "multiplayer.player.left" => Some("%1$s left the game"),
        "commands.generic.usage" => Some("Usage: %1$s"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_format_has_placeholders() {
        assert!(get_format("chat.type.text").unwrap().contains("%1$s"));
    }
}
