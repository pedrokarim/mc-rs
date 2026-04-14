//! Server log / audit trail.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Notice,
    Warning,
    Error,
    Critical,
    Alert,
    Emergency,
}

impl LogLevel {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Trace => "TRACE",
            Self::Debug => "DEBUG",
            Self::Info => "INFO",
            Self::Notice => "NOTICE",
            Self::Warning => "WARN",
            Self::Error => "ERROR",
            Self::Critical => "CRIT",
            Self::Alert => "ALERT",
            Self::Emergency => "EMERG",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: u64,
    pub level: LogLevel,
    pub source: String,
    pub message: String,
}

/// Format log entry.
pub fn format_line(entry: &LogEntry) -> String {
    format!(
        "[{}] [{}] [{}] {}",
        entry.timestamp,
        entry.level.label(),
        entry.source,
        entry.message
    )
}

/// Max log file size before rotation (10 MB).
pub const ROTATION_SIZE: u64 = 10 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_has_level() {
        let e = LogEntry {
            timestamp: 0,
            level: LogLevel::Info,
            source: "test".into(),
            message: "hello".into(),
        };
        assert!(format_line(&e).contains("INFO"));
    }
}
