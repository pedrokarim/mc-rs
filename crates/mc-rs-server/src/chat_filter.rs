//! Basic chat profanity filter.

const BANNED_PATTERNS: &[&str] = &[
    // Just a few examples — real list would be way bigger.
    "fuck", "shit",
];

pub fn is_profanity(text: &str) -> bool {
    let lower = text.to_lowercase();
    BANNED_PATTERNS.iter().any(|p| lower.contains(p))
}

pub fn censor(text: &str) -> String {
    let mut censored = text.to_string();
    for p in BANNED_PATTERNS {
        let replacement: String = std::iter::repeat('*').take(p.len()).collect();
        // Case-insensitive replace.
        let lower = censored.to_lowercase();
        while let Some(idx) = lower.find(p) {
            let real_idx = idx; // positions match since replacement is same length
            censored.replace_range(real_idx..real_idx + p.len(), &replacement);
            // Re-evaluate lower (we only one p at a time).
            if !censored.to_lowercase().contains(p) {
                break;
            }
        }
    }
    censored
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_profanity() {
        assert!(is_profanity("what the fuck"));
    }

    #[test]
    fn clean_passes() {
        assert!(!is_profanity("hello world"));
    }
}
