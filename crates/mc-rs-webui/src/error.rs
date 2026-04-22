//! Erreurs top-level du crate webui.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("bad webui config: {0}")]
    BadConfig(String),

    #[error("bind failed: {0}")]
    Bind(#[source] std::io::Error),

    #[error("axum runtime error: {0}")]
    Runtime(String),

    #[error("database error: {0}")]
    Db(String),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("internal error: {0}")]
    Internal(String),
}
