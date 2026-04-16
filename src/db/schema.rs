pub const CREATE_LOGS: &str = r#"
CREATE TABLE IF NOT EXISTS logs (
  id           TEXT NOT NULL,
  timestamp    TIMESTAMPTZ NOT NULL,
  source       TEXT NOT NULL,
  level        TEXT,
  message      TEXT,
  fields       JSON,
  ingested_at  TIMESTAMPTZ DEFAULT now()
);
"#;

pub const CREATE_SCHEMA_INFERENCE: &str = r#"
CREATE TABLE IF NOT EXISTS _schema (
  source       TEXT NOT NULL,
  field_path   TEXT NOT NULL,
  field_type   TEXT NOT NULL,
  seen_count   BIGINT DEFAULT 0,
  last_seen    TIMESTAMPTZ,
  PRIMARY KEY (source, field_path)
);
"#;

pub const CREATE_SOURCES: &str = r#"
CREATE TABLE IF NOT EXISTS sources (
  id             TEXT PRIMARY KEY,
  name           TEXT UNIQUE NOT NULL,
  description    TEXT,
  retention_days INTEGER DEFAULT 30,
  created_at     TIMESTAMPTZ DEFAULT now()
);
"#;

pub const CREATE_INGEST_TOKENS: &str = r#"
CREATE TABLE IF NOT EXISTS ingest_tokens (
  id           TEXT PRIMARY KEY,
  name         TEXT NOT NULL,
  token_hash   TEXT NOT NULL,
  source       TEXT NOT NULL,
  created_by   TEXT NOT NULL,
  created_at   TIMESTAMPTZ DEFAULT now(),
  expires_at   TIMESTAMPTZ,
  revoked_at   TIMESTAMPTZ,
  last_used    TIMESTAMPTZ
);
"#;

pub const CREATE_USERS: &str = r#"
CREATE TABLE IF NOT EXISTS users (
  id            TEXT PRIMARY KEY,
  email         TEXT UNIQUE NOT NULL,
  password_hash TEXT NOT NULL,
  role          TEXT NOT NULL,
  created_at    TIMESTAMPTZ DEFAULT now(),
  last_login    TIMESTAMPTZ
);
"#;

pub const CREATE_CONFIG: &str = r#"
CREATE TABLE IF NOT EXISTS config (
  key        TEXT PRIMARY KEY,
  value      TEXT NOT NULL,
  updated_at TIMESTAMPTZ DEFAULT now()
);
"#;
