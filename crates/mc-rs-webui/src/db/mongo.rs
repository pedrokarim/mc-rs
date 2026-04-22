//! Backend MongoDB via le crate `mongodb`. Feature `mongodb`.
//!
//! Collections :
//! - `users` : `{ _id: BinData<16>, username, password_hash, role, created_at, last_login_at }`
//! - `audit_log` : `{ _id: ObjectId, ts, user_id?, username_snapshot?, action, detail: string }`
//! - `tokens_blacklist` : `{ _id: jti, exp }`
//! - `meta` : `{ _id: key, value }`

use async_trait::async_trait;
use mongodb::bson::{doc, Binary, Bson, Document};
use mongodb::options::{ClientOptions, IndexOptions};
use mongodb::{Client, Collection, IndexModel};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use super::{AuditEntry, AuditFilter, Role, User, WebDb};
use crate::error::{Error, Result};

pub struct MongoDb {
    users: Collection<Document>,
    audit: Collection<Document>,
    tokens: Collection<Document>,
    meta: Collection<Document>,
}

impl MongoDb {
    pub async fn open(url: &str) -> Result<Self> {
        let mut opts = ClientOptions::parse(url)
            .await
            .map_err(|e| Error::Db(format!("mongodb parse url: {e}")))?;
        if opts.app_name.is_none() {
            opts.app_name = Some("mc-rs-webui".into());
        }
        // Choix du nom de DB : soit fourni dans l'URL, soit "mc_rs_webui".
        let db_name = opts
            .default_database
            .clone()
            .unwrap_or_else(|| "mc_rs_webui".to_string());

        let client = Client::with_options(opts)
            .map_err(|e| Error::Db(format!("mongodb client: {e}")))?;
        let database = client.database(&db_name);
        Ok(Self {
            users: database.collection("users"),
            audit: database.collection("audit_log"),
            tokens: database.collection("tokens_blacklist"),
            meta: database.collection("meta"),
        })
    }
}

fn uuid_binary(id: &Uuid) -> Bson {
    Bson::Binary(Binary {
        subtype: mongodb::bson::spec::BinarySubtype::Uuid,
        bytes: id.as_bytes().to_vec(),
    })
}

fn binary_to_uuid(b: &Binary) -> Result<Uuid> {
    if b.bytes.len() != 16 {
        return Err(Error::Db(format!("uuid binary len={}", b.bytes.len())));
    }
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&b.bytes);
    Ok(Uuid::from_bytes(arr))
}

fn doc_to_user(d: &Document) -> Result<User> {
    let id = match d.get("_id") {
        Some(Bson::Binary(b)) => binary_to_uuid(b)?,
        _ => return Err(Error::Db("user._id missing or wrong type".into())),
    };
    let username = d
        .get_str("username")
        .map_err(|e| Error::Db(e.to_string()))?
        .to_string();
    let password_hash = d
        .get_str("password_hash")
        .map_err(|e| Error::Db(e.to_string()))?
        .to_string();
    let role_str = d.get_str("role").map_err(|e| Error::Db(e.to_string()))?;
    let role = Role::from_str(role_str)
        .ok_or_else(|| Error::Db(format!("unknown role '{role_str}'")))?;
    let created_at = d
        .get_i64("created_at")
        .map_err(|e| Error::Db(e.to_string()))?;
    let last_login_at = d.get_i64("last_login_at").ok();
    Ok(User {
        id,
        username,
        password_hash,
        role,
        created_at,
        last_login_at,
    })
}

#[async_trait]
impl WebDb for MongoDb {
    async fn migrate(&self) -> Result<()> {
        // Mongo n'a pas de DDL — on s'assure juste des index uniques / triés.
        self.users
            .create_index(
                IndexModel::builder()
                    .keys(doc! { "username": 1 })
                    .options(IndexOptions::builder().unique(true).build())
                    .build(),
            )
            .await
            .map_err(|e| Error::Db(format!("index users.username: {e}")))?;
        self.audit
            .create_index(IndexModel::builder().keys(doc! { "ts": -1 }).build())
            .await
            .map_err(|e| Error::Db(format!("index audit.ts: {e}")))?;
        self.audit
            .create_index(IndexModel::builder().keys(doc! { "user_id": 1, "ts": -1 }).build())
            .await
            .map_err(|e| Error::Db(format!("index audit.user_id: {e}")))?;
        self.tokens
            .create_index(IndexModel::builder().keys(doc! { "exp": 1 }).build())
            .await
            .map_err(|e| Error::Db(format!("index tokens.exp: {e}")))?;
        Ok(())
    }

    async fn user_count(&self) -> Result<u64> {
        self.users
            .count_documents(doc! {})
            .await
            .map_err(|e| Error::Db(e.to_string()))
    }

    async fn create_user(&self, username: &str, password_hash: &str, role: Role) -> Result<User> {
        let id = Uuid::new_v4();
        let now = chrono::Utc::now().timestamp();
        let d = doc! {
            "_id": uuid_binary(&id),
            "username": username,
            "password_hash": password_hash,
            "role": role.as_str(),
            "created_at": now,
        };
        self.users
            .insert_one(d)
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
        let pattern = format!("^{}$", regex_escape(name));
        let filter = doc! { "username": { "$regex": pattern, "$options": "i" } };
        let found = self
            .users
            .find_one(filter)
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        match found {
            None => Ok(None),
            Some(d) => Ok(Some(doc_to_user(&d)?)),
        }
    }

    async fn list_users(&self) -> Result<Vec<User>> {
        let mut cursor = self
            .users
            .find(doc! {})
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        let mut out = Vec::new();
        use futures_util::TryStreamExt;
        while let Some(d) = cursor
            .try_next()
            .await
            .map_err(|e| Error::Db(e.to_string()))?
        {
            out.push(doc_to_user(&d)?);
        }
        out.sort_by(|a, b| a.username.cmp(&b.username));
        Ok(out)
    }

    async fn delete_user(&self, id: Uuid) -> Result<()> {
        self.users
            .delete_one(doc! { "_id": uuid_binary(&id) })
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(())
    }

    async fn update_password(&self, id: Uuid, hash: &str) -> Result<()> {
        self.users
            .update_one(
                doc! { "_id": uuid_binary(&id) },
                doc! { "$set": { "password_hash": hash } },
            )
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(())
    }

    async fn update_role(&self, id: Uuid, role: Role) -> Result<()> {
        self.users
            .update_one(
                doc! { "_id": uuid_binary(&id) },
                doc! { "$set": { "role": role.as_str() } },
            )
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(())
    }

    async fn touch_login(&self, id: Uuid, ts: i64) -> Result<()> {
        self.users
            .update_one(
                doc! { "_id": uuid_binary(&id) },
                doc! { "$set": { "last_login_at": ts } },
            )
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
        let mut d = doc! {
            "ts": ts,
            "action": action,
            "detail": detail.to_string(),
        };
        if let Some(uid) = user_id {
            d.insert("user_id", uuid_binary(&uid));
        }
        if let Some(name) = username_snapshot {
            d.insert("username_snapshot", name);
        }
        self.audit
            .insert_one(d)
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
        let mut f = Document::new();
        if let Some(uid) = filter.user_id {
            f.insert("user_id", uuid_binary(&uid));
        }
        if let Some(ref prefix) = filter.action_prefix {
            f.insert(
                "action",
                doc! { "$regex": format!("^{}", regex_escape(prefix)) },
            );
        }
        let mut ts_range = Document::new();
        if let Some(from) = filter.ts_from {
            ts_range.insert("$gte", from);
        }
        if let Some(to) = filter.ts_to {
            ts_range.insert("$lt", to);
        }
        if !ts_range.is_empty() {
            f.insert("ts", ts_range);
        }

        let mut cursor = self
            .audit
            .find(f)
            .sort(doc! { "ts": -1 })
            .skip(offset as u64)
            .limit(limit as i64)
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        use futures_util::TryStreamExt;
        let mut out = Vec::new();
        while let Some(d) = cursor
            .try_next()
            .await
            .map_err(|e| Error::Db(e.to_string()))?
        {
            let ts = d.get_i64("ts").map_err(|e| Error::Db(e.to_string()))?;
            let action = d
                .get_str("action")
                .map_err(|e| Error::Db(e.to_string()))?
                .to_string();
            let detail_str = d
                .get_str("detail")
                .map_err(|e| Error::Db(e.to_string()))?;
            let detail: JsonValue = serde_json::from_str(detail_str).unwrap_or(JsonValue::Null);
            let user_id = match d.get("user_id") {
                Some(Bson::Binary(b)) => Some(binary_to_uuid(b)?),
                _ => None,
            };
            let username_snapshot = d.get_str("username_snapshot").ok().map(String::from);
            // Pour l'id numérique : on utilise timestamp du ObjectId ou 0 (Mongo n'a pas de id auto-incrémenté).
            let id = d.get_object_id("_id").map(|o| o.timestamp().timestamp_millis()).unwrap_or(ts);
            out.push(AuditEntry {
                id,
                ts,
                user_id,
                username_snapshot,
                action,
                detail,
            });
        }
        Ok(out)
    }

    async fn audit_count(&self, filter: AuditFilter) -> Result<u64> {
        let mut f = Document::new();
        if let Some(uid) = filter.user_id {
            f.insert("user_id", uuid_binary(&uid));
        }
        if let Some(ref prefix) = filter.action_prefix {
            f.insert(
                "action",
                doc! { "$regex": format!("^{}", regex_escape(prefix)) },
            );
        }
        let mut ts_range = Document::new();
        if let Some(from) = filter.ts_from {
            ts_range.insert("$gte", from);
        }
        if let Some(to) = filter.ts_to {
            ts_range.insert("$lt", to);
        }
        if !ts_range.is_empty() {
            f.insert("ts", ts_range);
        }
        self.audit
            .count_documents(f)
            .await
            .map_err(|e| Error::Db(e.to_string()))
    }

    async fn blacklist_token(&self, jti: &str, exp_unix: i64) -> Result<()> {
        self.tokens
            .update_one(
                doc! { "_id": jti },
                doc! { "$set": { "exp": exp_unix } },
            )
            .upsert(true)
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(())
    }

    async fn is_blacklisted(&self, jti: &str) -> Result<bool> {
        let found = self
            .tokens
            .find_one(doc! { "_id": jti })
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(found.is_some())
    }

    async fn cleanup_expired_tokens(&self, now_unix: i64) -> Result<u64> {
        let res = self
            .tokens
            .delete_many(doc! { "exp": { "$lt": now_unix } })
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(res.deleted_count)
    }

    async fn get_meta(&self, key: &str) -> Result<Option<String>> {
        let found = self
            .meta
            .find_one(doc! { "_id": key })
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        match found {
            None => Ok(None),
            Some(d) => Ok(d.get_str("value").ok().map(String::from)),
        }
    }

    async fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.meta
            .update_one(
                doc! { "_id": key },
                doc! { "$set": { "value": value } },
            )
            .upsert(true)
            .await
            .map_err(|e| Error::Db(e.to_string()))?;
        Ok(())
    }
}

/// Regex-escape minimal pour les 12 chars spéciaux PCRE (Mongo utilise un moteur
/// PCRE-ish). Nécessaire pour éviter qu'un nom user contienne des metachars.
fn regex_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if ".^$*+?()[]{}|\\".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}
