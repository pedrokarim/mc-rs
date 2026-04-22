-- Schéma initial mc-rs-webui : users + audit log + JWT blacklist + meta KV.

CREATE TABLE IF NOT EXISTS users (
    id BLOB PRIMARY KEY NOT NULL,                  -- UUID v4 bytes
    username TEXT UNIQUE NOT NULL COLLATE NOCASE,
    password_hash TEXT NOT NULL,                   -- Argon2id PHC string
    role TEXT NOT NULL CHECK(role IN ('admin','moderator')),
    created_at INTEGER NOT NULL,
    last_login_at INTEGER
);

CREATE TABLE IF NOT EXISTS audit_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    ts INTEGER NOT NULL,
    user_id BLOB,                                  -- NULL si action système / first-boot
    username_snapshot TEXT,                        -- copie au cas où user supprimé
    action TEXT NOT NULL,
    detail TEXT NOT NULL                           -- JSON serialisé
);
CREATE INDEX IF NOT EXISTS idx_audit_ts ON audit_log(ts DESC);
CREATE INDEX IF NOT EXISTS idx_audit_user ON audit_log(user_id, ts DESC);

CREATE TABLE IF NOT EXISTS tokens_blacklist (
    jti TEXT PRIMARY KEY,
    exp INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tokens_exp ON tokens_blacklist(exp);

CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
