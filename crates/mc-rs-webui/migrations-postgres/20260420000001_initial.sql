-- Schéma initial mc-rs-webui Postgres.

CREATE TABLE IF NOT EXISTS users (
    id BYTEA PRIMARY KEY NOT NULL,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('admin','moderator')),
    created_at BIGINT NOT NULL,
    last_login_at BIGINT
);

CREATE TABLE IF NOT EXISTS audit_log (
    id BIGSERIAL PRIMARY KEY,
    ts BIGINT NOT NULL,
    user_id BYTEA,
    username_snapshot TEXT,
    action TEXT NOT NULL,
    detail TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_audit_ts ON audit_log(ts DESC);
CREATE INDEX IF NOT EXISTS idx_audit_user ON audit_log(user_id, ts DESC);

CREATE TABLE IF NOT EXISTS tokens_blacklist (
    jti TEXT PRIMARY KEY,
    exp BIGINT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_tokens_exp ON tokens_blacklist(exp);

CREATE TABLE IF NOT EXISTS meta (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
