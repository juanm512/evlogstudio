pub const CREATE_LOGS: &str = r#"
CREATE TABLE IF NOT EXISTS logs (
  id           TEXT NOT NULL,
  timestamp    TIMESTAMPTZ NOT NULL,
  source       TEXT NOT NULL,
  service      TEXT,
  environment  TEXT,
  method       TEXT,
  path         TEXT,
  status       INTEGER,
  duration_ms  INTEGER,
  request_id   TEXT,
  error        TEXT,
  level        TEXT,
  message      TEXT,
  fields       JSON,
  ingested_at  TIMESTAMPTZ DEFAULT now()
);
"#;

pub const MIGRATE_LOGS_ADD_COLUMNS: &str = r#"
ALTER TABLE logs ADD COLUMN IF NOT EXISTS service TEXT;
ALTER TABLE logs ADD COLUMN IF NOT EXISTS environment TEXT;
ALTER TABLE logs ADD COLUMN IF NOT EXISTS method TEXT;
ALTER TABLE logs ADD COLUMN IF NOT EXISTS path TEXT;
ALTER TABLE logs ADD COLUMN IF NOT EXISTS status INTEGER;
ALTER TABLE logs ADD COLUMN IF NOT EXISTS duration_ms INTEGER;
ALTER TABLE logs ADD COLUMN IF NOT EXISTS request_id TEXT;
ALTER TABLE logs ADD COLUMN IF NOT EXISTS error TEXT;
"#;

pub const CREATE_IDX_LOGS_SERVICE: &str =
    "CREATE INDEX IF NOT EXISTS idx_logs_service ON logs (service)";
pub const CREATE_IDX_LOGS_ENV: &str =
    "CREATE INDEX IF NOT EXISTS idx_logs_env ON logs (environment)";
pub const CREATE_IDX_LOGS_STATUS: &str =
    "CREATE INDEX IF NOT EXISTS idx_logs_status ON logs (status)";
pub const CREATE_IDX_LOGS_REQUEST_ID: &str =
    "CREATE INDEX IF NOT EXISTS idx_logs_request_id ON logs (request_id)";
pub const CREATE_IDX_LOGS_DURATION: &str =
    "CREATE INDEX IF NOT EXISTS idx_logs_duration ON logs (duration_ms)";

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

/// Agrega columna `retention TEXT` a sources si no existe.
pub const MIGRATE_SOURCES_ADD_RETENTION_COL: &str =
    "ALTER TABLE sources ADD COLUMN IF NOT EXISTS retention TEXT";

/// Puebla `retention` desde `retention_days` para filas existentes.
pub const MIGRATE_SOURCES_POPULATE_RETENTION: &str = r#"
UPDATE sources SET retention = CAST(retention_days AS VARCHAR) || 'd'
WHERE retention IS NULL AND retention_days IS NOT NULL;
UPDATE sources SET retention = '30d' WHERE retention IS NULL;
"#;

/// Agrega campos de sampling por source.
pub const MIGRATE_SOURCES_ADD_SAMPLING: &str = r#"
ALTER TABLE sources ADD COLUMN IF NOT EXISTS sampling_enabled BOOLEAN DEFAULT false;
ALTER TABLE sources ADD COLUMN IF NOT EXISTS sampling_debug_rate INTEGER DEFAULT 10;
ALTER TABLE sources ADD COLUMN IF NOT EXISTS sampling_info_rate INTEGER DEFAULT 100;
ALTER TABLE sources ADD COLUMN IF NOT EXISTS sampling_warn_rate INTEGER DEFAULT 100;
"#;

/// Puebla defaults de sampling para filas existentes que queden NULL.
pub const MIGRATE_SOURCES_POPULATE_SAMPLING: &str = r#"
UPDATE sources SET sampling_enabled = false WHERE sampling_enabled IS NULL;
UPDATE sources SET sampling_debug_rate = 10 WHERE sampling_debug_rate IS NULL;
UPDATE sources SET sampling_info_rate = 100 WHERE sampling_info_rate IS NULL;
UPDATE sources SET sampling_warn_rate = 100 WHERE sampling_warn_rate IS NULL;
"#;

/// Renombra la clave de config de días (entero) al nuevo formato de string.
pub const MIGRATE_CONFIG_RETENTION_KEY: &str =
    "UPDATE config SET key = 'retention.default', value = value || 'd' \
     WHERE key = 'retention.default_days' \
     AND value NOT LIKE '%d' AND value NOT LIKE '%h' AND value NOT LIKE '%m'";

pub const CREATE_DASHBOARDS: &str = r#"
CREATE TABLE IF NOT EXISTS dashboards (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  description TEXT,
  created_by  TEXT NOT NULL,
  created_at  TIMESTAMPTZ DEFAULT now(),
  updated_at  TIMESTAMPTZ DEFAULT now()
)
"#;

pub const CREATE_WIDGETS: &str = r#"
CREATE TABLE IF NOT EXISTS widgets (
  id            TEXT PRIMARY KEY,
  dashboard_id  TEXT NOT NULL,
  title         TEXT NOT NULL,
  "type"        TEXT NOT NULL,
  width         TEXT NOT NULL DEFAULT 'half',
  position      INTEGER NOT NULL DEFAULT 0,
  config        TEXT NOT NULL,
  created_at    TIMESTAMPTZ DEFAULT now()
)
"#;
