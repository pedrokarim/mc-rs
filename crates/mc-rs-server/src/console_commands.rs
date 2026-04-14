//! Console-only commands.

/// Commands restricted to console / operator permission.
pub fn console_only_commands() -> &'static [&'static str] {
    &[
        "stop",
        "save-all",
        "save-on",
        "save-off",
        "stop",
        "reload",
        "restart",
        "whitelist",
        "ban-ip",
        "unban-ip",
        "pardon",
        "pardon-ip",
        "memory",
        "status",
        "timings",
    ]
}

pub fn is_console_only(cmd: &str) -> bool {
    let lower = cmd.to_lowercase();
    let trimmed = lower.trim_start_matches('/');
    let first = trimmed.split_whitespace().next().unwrap_or("");
    console_only_commands().contains(&first)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_is_console() {
        assert!(is_console_only("stop"));
        assert!(is_console_only("/stop"));
    }
}
