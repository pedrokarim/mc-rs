//! Backend Postgres via `sqlx`. Feature `postgres`.
//!
//! Placeholders `$1, $2, ...` (vs `?` pour SQLite) et types natifs `BYTEA` /
//! `BIGINT`. Migrations dans `migrations-postgres/`.

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::Row;
use uuid::Uuid;

use super::{AuditEntry, AuditFilter, Role, User, WebDb};
use crate::error::{Error, Result};

pub struct PostgresDb {
    pool: PgPool,
}

impl PostgresDb {
    /// `url` est l'URL complète `postgres://user:pass@host:port/db`.
    pub async fn open(url: &str) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(url)
            .await
            .map_err(|e| Error::Db(format!("postgres connect: {e}")))?;
        Ok(Self { pool })
    }
}

fn uuid_bytes(id: &Uuid) -> Vec<u8> {
    id.as_bytes().to_vec()
}

fn uuid_from_bytes(bytes: &[u8]) -> Result<Uuid> {
    if bytes.len() != 16 {
        return Err(Error::Db(format!("uuid bytea len={}, expected 16", bytes.len())));
    }
    let mut arr = [0u8; 16];
    arr.copy_from_slice(bytes);
    Ok(Uuid::from_bytes(arr))
}

#[async_trait]
impl WebDb for PostgresDb {
    async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations-postgres")
            .run(&self.pool)
            .await
            .map_err(|e| Error::Db(format!("migration: {e}")))
    }

    async fn user_count(&self) -> Result<u64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(row.0 as u64)
    }

    async fn create_user(&self, username: &str, password_hash: &str, role: Role) -> Result<User> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now().timestamp();
        sqlx::query(
            "INSERT INTO users (id, username, password_hash, role, created_at) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(uuid_bytes(&id))
        .bind(username)
        .bind(password_hash)
        .bind(role.as_str())
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Db(format!("create_user: {e}")))?;
        Ok(User {
            id,
            username: username.to_string(),
            password_hash: password_hash.to_string(),
            role,
            created_at: now,
            last_login_at: None,
        })
    }

    async fn find_user_by_name(&self, name: &str) -> Result<Option<User>> {
        let row = sqlx::query(
            "SELECT id, username, password_hash, role, created_at, last_login_at \
             FROM users WHERE LOWER(username) = LOWER($1)",
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| Error::Db(e.to_string()))?;
        match row {
            None => Ok(None),
            Some(row) => Ok(Some(row_to_user(&row)?)),
        }
    }

    async fn list_users(&self) -> Result<Vec<User>> {
        let rows = sqlx::query(
            "SELECT id, username, password_hash, role, created_at, last_login_at \
             FROM users ORDER BY username",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| Error::Db(e.to_string()))?;
        rows.iter().map(row_to_user).collect()
    }

    async fn delete_user(&self, id: Uuid) -> Result<()> {
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(uuid_bytes(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(())
    }

    async fn update_password(&self, id: Uuid, hash: &str) -> Result<()> {
        sqlx::query("UPDATE users SET password_hash = $1 WHERE id = $2")
            .bind(hash)
            .bind(uuid_bytes(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(())
    }

    async fn update_role(&self, id: Uuid, role: Role) -> Result<()> {
        sqlx::query("UPDATE users SET role = $1 WHERE id = $2")
            .bind(role.as_str())
            .bind(uuid_bytes(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(())
    }

    async fn touch_login(&self, id: Uuid, ts: i64) -> Result<()> {
        sqlx::query("UPDATE users SET last_login_at = $1 WHERE id = $2")
            .bind(ts)
            .bind(uuid_bytes(&id))
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(())
    }

    async fn audit_log(
        &self,
        user_id: Option<Uuid>,
        username_snapshot: Option<&str>,
        action: &str,
        detail: JsonValue,
    ) -> Result<()> {
        let ts = chrono::Utc::now().timestamp();
        let detail_str = detail.to_string();
        let uid_bytes = user_id.map(|u| uuid_bytes(&u));
        sqlx::query(
            "INSERT INTO audit_log (ts, user_id, username_snapshot, action, detail) \
             VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(ts)
        .bind(uid_bytes)
        .bind(username_snapshot)
        .bind(action)
        .bind(detail_str)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Db(e.to_string()))?;
        Ok(())
    }

    async fn audit_page(
        &self,
        limit: u32,
        offset: u32,
        filter: AuditFilter,
    ) -> Result<Vec<AuditEntry>> {
        let mut sql = String::from(
            "SELECT id, ts, user_id, username_snapshot, action, detail FROM audit_log WHERE 1=1",
        );
        let mut n = 1;
        if filter.user_id.is_some() {
            sql.push_str(&format!(" AND user_id = ${}", n));
            n += 1;
        }
        if filter.action_prefix.is_some() {
            sql.push_str(&format!(" AND action LIKE ${}", n));
            n += 1;
        }
        if filter.ts_from.is_some() {
            sql.push_str(&format!(" AND ts >= ${}", n));
            n += 1;
        }
        if filter.ts_to.is_some() {
            sql.push_str(&format!(" AND ts < ${}", n));
            n += 1;
        }
        sql.push_str(&format!(" ORDER BY ts DESC LIMIT ${} OFFSET ${}", n, n + 1));

        let mut q = sqlx::query(&sql);
        if let Some(uid) = filter.user_id {
            q = q.bind(uuid_bytes(&uid));
        }
        if let Some(ref prefix) = filter.action_prefix {
            q = q.bind(format!("{prefix}%"));
        }
        if let Some(from) = filter.ts_from {
            q = q.bind(from);
        }
        if let Some(to) = filter.ts_to {
            q = q.bind(to);
        }
        q = q.bind(limit as i64).bind(offset as i64);

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let detail_str: String =
                row.try_get("detail").map_err(|e| Error::Db(e.to_string()))?;
            let detail: JsonValue = serde_json::from_str(&detail_str).unwrap_or(JsonValue::Null);
            let user_id_bytes: Option<Vec<u8>> = row.try_get("user_id").ok();
            let user_id = match user_id_bytes {
                Some(b) if !b.is_empty() => Some(uuid_from_bytes(&b)?),
                _ => None,
            };
            out.push(AuditEntry {
                id: row.try_get("id").map_err(|e| Error::Db(e.to_string()))?,
                ts: row.try_get("ts").map_err(|e| Error::Db(e.to_string()))?,
                user_id,
                username_snapshot: row.try_get("username_snapshot").ok(),
                action: row.try_get("action").map_err(|e| Error::Db(e.to_string()))?,
                detail,
            });
        }
        Ok(out)
    }

    async fn audit_count(&self, filter: AuditFilter) -> Result<u64> {
        let mut sql = String::from("SELECT COUNT(*) FROM audit_log WHERE 1=1");
        let mut n = 1;
        if filter.user_id.is_some() {
            sql.push_str(&format!(" AND user_id = ${}", n));
            n += 1;
        }
        if filter.action_prefix.is_some() {
            sql.push_str(&format!(" AND action LIKE ${}", n));
            n += 1;
        }
        if filter.ts_from.is_some() {
            sql.push_str(&format!(" AND ts >= ${}", n));
            n += 1;
        }
        if filter.ts_to.is_some() {
            sql.push_str(&format!(" AND ts < ${}", n));
            n += 1;
        }
        let _ = n;
        let mut q = sqlx::query_as::<_, (i64,)>(&sql);
        if let Some(uid) = filter.user_id {
            q = q.bind(uuid_bytes(&uid));
        }
        if let Some(ref prefix) = filter.action_prefix {
            q = q.bind(format!("{prefix}%"));
        }
        if let Some(from) = filter.ts_from {
            q = q.bind(from);
        }
        if let Some(to) = filter.ts_to {
            q = q.bind(to);
        }
        let row = q
            .fetch_one(&self.pool)
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(row.0 as u64)
    }

    async fn blacklist_token(&self, jti: &str, exp_unix: i64) -> Result<()> {
        sqlx::query(
            "INSERT INTO tokens_blacklist (jti, exp) VALUES ($1, $2) \
             ON CONFLICT (jti) DO UPDATE SET exp = EXCLUDED.exp",
        )
        .bind(jti)
        .bind(exp_unix)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Db(e.to_string()))?;
        Ok(())
    }

    async fn is_blacklisted(&self, jti: &str) -> Result<bool> {
        let row: Option<(i64,)> = sqlx::query_as("SELECT exp FROM tokens_blacklist WHERE jti = $1")
            .bind(jti)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(row.is_some())
    }

    async fn cleanup_expired_tokens(&self, now_unix: i64) -> Result<u64> {
        let res = sqlx::query("DELETE FROM tokens_blacklist WHERE exp < $1")
            .bind(now_unix)
            .execute(&self.pool)
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(res.rows_affected())
    }

    async fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM meta WHERE key = $1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(row.map(|r| r.0))
    }

    async fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO meta (key, value) VALUES ($1, $2) \
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value",
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(|e| Error::Db(e.to_string()))?;
        Ok(())
    }
}

fn row_to_user(row: &sqlx::postgres::PgRow) -> Result<User> {
    let id_bytes: Vec<u8> = row.try_get("id").map_err(|e| Error::Db(e.to_string()))?;
    let role_str: String = row.try_get("role").map_err(|e| Error::Db(e.to_string()))?;
    Ok(User {
        id: uuid_from_bytes(&id_bytes)?,
        username: row.try_get("username").map_err(|e| Error::Db(e.to_string()))?,
        password_hash: row
            .try_get("password_hash")
            .map_err(|e| Error::Db(e.to_string()))?,
        role: Role::from_str(&role_str)
            .ok_or_else(|| Error::Db(format!("unknown role '{role_str}'")))?,
        created_at: row
            .try_get("created_at")
            .map_err(|e| Error::Db(e.to_string()))?,
        last_login_at: row.try_get("last_login_at").ok(),
    })
}
