//! Chat channel / whisper system.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatChannel {
    Global,   // /say
    Local,    // by distance
    Team,     // team chat
    Operator, // admins
    Whisper,  // DM
}

/// Max chat range for Local channel.
pub const LOCAL_RANGE: f64 = 64.0;
/// Max message length.
pub const MAX_MESSAGE_LENGTH: usize = 256;

pub fn sanitize_message(msg: &str) -> String {
    msg.chars()
        .filter(|c| !c.is_control())
        .take(MAX_MESSAGE_LENGTH)
        .collect()
}

/// Per-channel prefix format.
pub fn channel_prefix(channel: ChatChannel) -> &'static str {
    match channel {
        ChatChannel::Global => "",
        ChatChannel::Local => "[Local]",
        ChatChannel::Team => "[Team]",
        ChatChannel::Operator => "[Op]",
        ChatChannel::Whisper => "[Whisper]",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_long_message() {
        let long = "a".repeat(500);
        assert_eq!(sanitize_message(&long).len(), MAX_MESSAGE_LENGTH);
    }

    #[test]
    fn strip_control_chars() {
        let msg = "hello\x00\x07world";
        assert_eq!(sanitize_message(msg), "helloworld");
    }
}
