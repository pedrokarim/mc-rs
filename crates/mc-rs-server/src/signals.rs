//! Signal handling (SIGINT/SIGTERM).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalKind {
    Interrupt, // Ctrl+C
    Terminate,
    Quit,
    Hangup,
    UserDefined1,
    UserDefined2,
}

/// On shutdown signal, should save world gracefully.
pub fn graceful_shutdown_kinds() -> &'static [SignalKind] {
    &[SignalKind::Interrupt, SignalKind::Terminate, SignalKind::Quit]
}

/// Shutdown timeout before force-kill (30s).
pub const SHUTDOWN_TIMEOUT_SECS: u64 = 30;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupt_is_graceful() {
        assert!(graceful_shutdown_kinds().contains(&SignalKind::Interrupt));
    }
}
