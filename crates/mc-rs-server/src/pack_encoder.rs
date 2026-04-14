//! Resource/behavior pack encoder — ZIP / UUID tracking.

#[derive(Debug, Clone)]
pub struct ResourcePackInfo {
    pub uuid: String,
    pub version: String,
    pub size: u64,
    pub content_key: Option<String>,
    pub sub_pack_name: String,
    pub content_identity: String,
    pub has_scripts: bool,
}

/// Encrypt packet bytes if content_key set (AES).
pub fn encrypts_content(key: &Option<String>) -> bool {
    key.as_ref().map(|k| !k.is_empty()).unwrap_or(false)
}

/// Default chunk size (1MB).
pub const CHUNK_SIZE: u64 = 1024 * 1024;

/// Compute number of chunks.
pub fn num_chunks(total: u64) -> u64 {
    total.div_ceil(CHUNK_SIZE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_count_exact() {
        assert_eq!(num_chunks(CHUNK_SIZE * 3), 3);
    }

    #[test]
    fn chunk_count_partial() {
        assert_eq!(num_chunks(CHUNK_SIZE + 1), 2);
    }
}
