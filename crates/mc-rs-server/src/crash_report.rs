//! Crash report generator.

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone)]
pub struct CrashReport {
    pub timestamp: u64,
    pub title: String,
    pub description: String,
    pub stack_trace: Vec<String>,
    pub thread_name: String,
    pub system_details: SystemDetails,
}

#[derive(Debug, Clone, Default)]
pub struct SystemDetails {
    pub os: String,
    pub java_version: String,
    pub memory_used_mb: u64,
    pub memory_allocated_mb: u64,
    pub memory_max_mb: u64,
    pub rust_version: String,
    pub cpu: String,
}

impl CrashReport {
    pub fn now(title: impl Into<String>) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            timestamp,
            title: title.into(),
            description: String::new(),
            stack_trace: Vec::new(),
            thread_name: "Server thread".into(),
            system_details: SystemDetails::default(),
        }
    }

    pub fn format(&self) -> String {
        format!(
            "---- MC-RS Crash Report ----\nTime: {}\nTitle: {}\nDescription: {}\nStack:\n{}",
            self.timestamp,
            self.title,
            self.description,
            self.stack_trace.join("\n")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_contains_title() {
        let r = CrashReport::now("Test");
        assert!(r.format().contains("Test"));
    }
}
