//! Abstraction DB : trait [`WebDb`] + factory par URL.
//!
//! Implems feature-gated :
//! - `sqlite` (défaut) : [`sqlite::SqliteDb`]
//! - `postgres` (futur) : [`postgres::PostgresDb`]
//! - `mongodb` (futur) : [`mongo::MongoDb`]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{Error, Result};

#[cfg(feature = "sqlite")]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "mongodb")]
pub mod mongo;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
    Moderator,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Admin => "admin",
            Role::Moderator => "moderator",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(Role::Admin),
            "moderator" => Some(Role::Moderator),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub role: Role,
    pub created_at: i64,
    pub last_login_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub ts: i64,
    pub user_id: Option<Uuid>,
    pub username_snapshot: Option<String>,
    pub action: String,
    pub detail: JsonValue,
}

#[derive(Debug, Default, Clone)]
pub struct AuditFilter {
    pub user_id: Option<Uuid>,
    pub action_prefix: Option<String>,
    /// Unix seconds inclusive.
    pub ts_from: Option<i64>,
    /// Unix seconds exclusive.
    pub ts_to: Option<i64>,
}

#[async_trait]
pub trait WebDb: Send + Sync {
    async fn migrate(&self) -> Result<()>;

    // ── Users ──
    async fn user_count(&self) -> Result<u64>;
    async fn create_user(&self, username: &str, password_hash: &str, role: Role) -> Result<User>;
    async fn find_user_by_name(&self, name: &str) -> Result<Option<User>>;
    async fn list_users(&self) -> Result<Vec<User>>;
    async fn delete_user(&self, id: Uuid) -> Result<()>;
    async fn update_password(&self, id: Uuid, hash: &str) -> Result<()>;
    async fn update_role(&self, id: Uuid, role: Role) -> Result<()>;
    async fn touch_login(&self, id: Uuid, ts: i64) -> Result<()>;

    // ── Audit ──
    async fn audit_log(
        &self,
        user_id: Option<Uuid>,
        username_snapshot: Option<&str>,
        action: &str,
        detail: JsonValue,
    ) -> Result<()>;
    async fn audit_page(
        &self,
        limit: u32,
        offset: u32,
        filter: AuditFilter,
    ) -> Result<Vec<AuditEntry>>;
    async fn audit_count(&self, filter: AuditFilter) -> Result<u64>;

    // ── JWT revocation ──
    async fn blacklist_token(&self, jti: &str, exp_unix: i64) -> Result<()>;
    async fn is_blacklisted(&self, jti: &str) -> Result<bool>;
    async fn cleanup_expired_tokens(&self, now_unix: i64) -> Result<u64>;

    // ── Meta KV (JWT secret, etc.) ──
    async fn get_meta(&self, key: &str) -> Result<Option<String>>;
    async fn set_meta(&self, key: &str, value: &str) -> Result<()>;
}

/// Instancie le backend DB correspondant au scheme de l'URL.
pub async fn open_db(url: &str) -> Result<Arc<dyn WebDb>> {
    if let Some(rest) = url.strip_prefix("sqlite://") {
        #[cfg(feature = "sqlite")]
        {
            let db = sqlite::SqliteDb::open(rest).await?;
            return Ok(Arc::new(db));
        }
        #[cfg(not(feature = "sqlite"))]
        {
            let _ = rest;
            return Err(Error::BadConfig(
                "compiled without `sqlite` feature, but database_url uses sqlite://".to_string(),
            ));
        }
    }

    if url.starts_with("postgres://") || url.starts_with("postgresql://") {
        #[cfg(feature = "postgres")]
        {
            let db = postgres::PostgresDb::open(url).await?;
            return Ok(Arc::new(db));
        }
        #[cfg(not(feature = "postgres"))]
        {
            return Err(Error::BadConfig(
                "compiled without `postgres` feature, but database_url uses postgres://"
                    .to_string(),
            ));
        }
    }

    if url.starts_with("mongodb://") || url.starts_with("mongodb+srv://") {
        #[cfg(feature = "mongodb")]
        {
            let db = mongo::MongoDb::open(url).await?;
            return Ok(Arc::new(db));
        }
        #[cfg(not(feature = "mongodb"))]
        {
            return Err(Error::BadConfig(
                "compiled without `mongodb` feature, but database_url uses mongodb://".to_string(),
            ));
        }
    }

    Err(Error::BadConfig(format!(
        "unknown database_url scheme: {url}"
    )))
}
