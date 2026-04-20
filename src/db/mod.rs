pub mod schema;

use crate::config::Config;
use duckdb::Connection;
use std::sync::Mutex;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};
use aws_sdk_s3::{
    Client as S3Client,
    config::{Credentials, Region},
    error::SdkError,
    operation::get_object::GetObjectError,
};
use tokio::io::AsyncWriteExt;


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPublicRecord {
    pub id: String,
    pub email: String,
    pub role: String,
    pub created_at: Option<DateTime<Utc>>,
    pub last_login: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceRecord {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub retention: Option<String>,
    pub sampling_enabled: Option<bool>,
    pub sampling_debug_rate: Option<i32>,
    pub sampling_info_rate: Option<i32>,
    pub sampling_warn_rate: Option<i32>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Default)]
pub struct UpdateSourceParams {
    pub retention: Option<String>,
    pub sampling_enabled: Option<bool>,
    pub sampling_debug_rate: Option<i32>,
    pub sampling_info_rate: Option<i32>,
    pub sampling_warn_rate: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestTokenRecord {
    pub id: String,
    pub name: String,
    pub source: String,
    pub created_by: String,
    pub created_at: Option<DateTime<Utc>>,
    pub last_used: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRecord {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub service: Option<String>,
    pub environment: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status: Option<i32>,
    pub duration_ms: Option<i32>,
    pub request_id: Option<String>,
    pub error: Option<String>,
    pub level: Option<String>,
    pub message: Option<String>,
    pub fields: serde_json::Value,
    pub ingested_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VolumePoint {
    pub bucket: DateTime<Utc>,
    pub count: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorRateResult {
    pub total: i64,
    pub errors: i64,
    pub rate: f64,
}

#[derive(thiserror::Error, Debug)]
pub enum DbError {
    #[error("Error al abrir base de datos: {0}")]
    Open(String),
    #[error("Error de migracion: {0}")]
    Migration(String),
    #[error("Error de consulta: {0}")]
    Query(String),
    #[error("Input invalido: {0}")]
    InvalidInput(String),
}

/// Configuración de S3 almacenada en el struct Db cuando storage_mode=s3.
#[derive(Clone, Debug)]
struct S3Cfg {
    bucket: String,
    region: String,
    endpoint: Option<String>,
    access_key_id: String,
    secret_access_key: String,
}

pub struct Db {
    conn: Mutex<Connection>,
    /// Solo Some(_) cuando storage_mode = "s3"
    s3_cfg: Option<S3Cfg>,
}


impl Db {
    pub fn open(config: &Config) -> Result<Self, DbError> {
        match config.storage_mode.as_str() {
            "local" => {
                let conn = Connection::open(&config.data_path)
                    .map_err(|e| DbError::Open(e.to_string()))?;
                let db = Db { conn: Mutex::new(conn), s3_cfg: None };
                db.run_migrations()?;
                db.seed_config()?;
                Ok(db)
            }

            "motherduck" => {
                let token = config.motherduck_token.as_deref()
                    .expect("MOTHERDUCK_TOKEN es requerido para storage_mode=motherduck");
                // MotherDuck lee el token desde la variable de entorno
                std::env::set_var("motherduck_token", token);
                let conn = Connection::open("md:evlogstudio")
                    .map_err(|e| DbError::Open(format!("MotherDuck: {}", e)))?;
                let db = Db { conn: Mutex::new(conn), s3_cfg: None };
                db.run_migrations()?;
                db.seed_config()?;
                Ok(db)
            }

            "s3" => {
                let bucket = config.s3_bucket.clone()
                    .expect("S3_BUCKET es requerido para storage_mode=s3");
                let access_key_id = config.s3_access_key_id.clone()
                    .expect("S3_ACCESS_KEY_ID es requerido para storage_mode=s3");
                let secret_access_key = config.s3_secret_access_key.clone()
                    .expect("S3_SECRET_ACCESS_KEY es requerido para storage_mode=s3");
                let region = config.s3_region.clone()
                    .unwrap_or_else(|| "us-east-1".to_string());

                let conn = Connection::open("/tmp/evlogstudio_buffer.duckdb")
                    .map_err(|e| DbError::Open(format!("S3 buffer: {}", e)))?;

                let s3_cfg = S3Cfg {
                    bucket,
                    region,
                    endpoint: config.s3_endpoint.clone(),
                    access_key_id,
                    secret_access_key,
                };

                let db = Db { conn: Mutex::new(conn), s3_cfg: Some(s3_cfg) };
                db.run_migrations()?;
                db.seed_config()?;
                Ok(db)
            }

            other => panic!(
                "STORAGE_MODE inválido: '{}'. Valores válidos: local, motherduck, s3",
                other
            ),
        }
    }

    pub fn run_migrations(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Migration(format!("Mutex envenenado: {}", e)))?;

        conn.execute_batch(schema::CREATE_SOURCES)
            .map_err(|e| DbError::Migration(format!("sources: {}", e)))?;

        conn.execute_batch(schema::CREATE_LOGS)
            .map_err(|e| DbError::Migration(format!("logs: {}", e)))?;

        conn.execute_batch(schema::MIGRATE_LOGS_ADD_COLUMNS)
            .map_err(|e| DbError::Migration(format!("logs migrate columns: {}", e)))?;

        conn.execute_batch(schema::CREATE_IDX_LOGS_SERVICE)
            .map_err(|e| DbError::Migration(format!("idx_logs_service: {}", e)))?;
        conn.execute_batch(schema::CREATE_IDX_LOGS_ENV)
            .map_err(|e| DbError::Migration(format!("idx_logs_env: {}", e)))?;
        conn.execute_batch(schema::CREATE_IDX_LOGS_STATUS)
            .map_err(|e| DbError::Migration(format!("idx_logs_status: {}", e)))?;
        conn.execute_batch(schema::CREATE_IDX_LOGS_REQUEST_ID)
            .map_err(|e| DbError::Migration(format!("idx_logs_request_id: {}", e)))?;
        conn.execute_batch(schema::CREATE_IDX_LOGS_DURATION)
            .map_err(|e| DbError::Migration(format!("idx_logs_duration: {}", e)))?;

        conn.execute_batch(schema::CREATE_SCHEMA_INFERENCE)
            .map_err(|e| DbError::Migration(format!("_schema: {}", e)))?;

        conn.execute_batch(schema::CREATE_INGEST_TOKENS)
            .map_err(|e| DbError::Migration(format!("ingest_tokens: {}", e)))?;

        conn.execute_batch(schema::CREATE_USERS)
            .map_err(|e| DbError::Migration(format!("users: {}", e)))?;

        conn.execute_batch(schema::CREATE_CONFIG)
            .map_err(|e| DbError::Migration(format!("config: {}", e)))?;

        conn.execute_batch(schema::MIGRATE_SOURCES_ADD_RETENTION_COL)
            .map_err(|e| DbError::Migration(format!("sources add retention col: {}", e)))?;

        conn.execute_batch(schema::MIGRATE_SOURCES_POPULATE_RETENTION)
            .map_err(|e| DbError::Migration(format!("sources populate retention: {}", e)))?;

        conn.execute_batch(schema::MIGRATE_CONFIG_RETENTION_KEY)
            .map_err(|e| DbError::Migration(format!("config retention key migration: {}", e)))?;

        conn.execute_batch(schema::MIGRATE_SOURCES_ADD_SAMPLING)
            .map_err(|e| DbError::Migration(format!("sources add sampling cols: {}", e)))?;

        conn.execute_batch(schema::MIGRATE_SOURCES_POPULATE_SAMPLING)
            .map_err(|e| DbError::Migration(format!("sources populate sampling: {}", e)))?;

        conn.execute_batch(schema::CREATE_DASHBOARDS)
            .map_err(|e| DbError::Migration(format!("dashboards: {}", e)))?;

        conn.execute_batch(schema::CREATE_WIDGETS)
            .map_err(|e| DbError::Migration(format!("widgets: {}", e)))?;

        Ok(())
    }

    pub fn seed_config(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;

        conn.execute(
            "INSERT INTO config (key, value) VALUES ('retention.default', '30d') ON CONFLICT (key) DO NOTHING",
            []
        ).map_err(|e| DbError::Query(format!("seed retention.default: {}", e)))?;

        conn.execute(
            "INSERT INTO config (key, value) VALUES ('sampling.enabled', 'false') ON CONFLICT (key) DO NOTHING",
            []
        ).map_err(|e| DbError::Query(format!("seed sampling.enabled: {}", e)))?;

        conn.execute(
            "INSERT INTO config (key, value) VALUES ('sampling.debug_rate', '10') ON CONFLICT (key) DO NOTHING",
            []
        ).map_err(|e| DbError::Query(format!("seed sampling.debug_rate: {}", e)))?;

        conn.execute(
            "INSERT INTO config (key, value) VALUES ('sampling.info_rate', '100') ON CONFLICT (key) DO NOTHING",
            []
        ).map_err(|e| DbError::Query(format!("seed sampling.info_rate: {}", e)))?;

        conn.execute(
            "INSERT INTO config (key, value) VALUES ('sampling.warn_rate', '100') ON CONFLICT (key) DO NOTHING",
            []
        ).map_err(|e| DbError::Query(format!("seed sampling.warn_rate: {}", e)))?;

        Ok(())
    }

    pub fn insert_logs(&self, logs: &[crate::ingest::normalize::NormalizedLog]) -> Result<usize, DbError> {
        if logs.is_empty() {
            return Ok(0);
        }

        let mut conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let tx = conn.transaction().map_err(|e| DbError::Query(format!("Error starting transaction: {}", e)))?;

        let mut count = 0;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO logs (id, timestamp, source, service, environment, method, path, status, duration_ms, request_id, error, level, message, fields, ingested_at) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
            ).map_err(|e| DbError::Query(format!("Prepare statement error: {}", e)))?;

            for log in logs {
                let fields_json = log.fields.to_string();
                stmt.execute(duckdb::params![
                    &log.id,
                    &log.timestamp,
                    &log.source,
                    &log.service,
                    &log.environment,
                    &log.method,
                    &log.path,
                    &log.status,
                    &log.duration_ms,
                    &log.request_id,
                    &log.error,
                    &log.level,
                    &log.message,
                    &fields_json,
                    &log.ingested_at
                ]).map_err(|e| DbError::Query(format!("Execute statement error: {}", e)))?;
                
                count += 1;
            }
        }
        tx.commit().map_err(|e| DbError::Query(format!("Commit transaction error: {}", e)))?;

        Ok(count)
    }

    pub fn count_logs(&self) -> Result<usize, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare("SELECT count(*) FROM logs").map_err(|e| DbError::Query(e.to_string()))?;
        let count: i64 = stmt.query_row([], |row| row.get(0)).map_err(|e| DbError::Query(e.to_string()))?;
        Ok(count as usize)
    }

    pub fn query_logs(&self, params: &LogQueryParams) -> Result<Vec<crate::ingest::normalize::NormalizedLog>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        
        let mut query = "SELECT id, timestamp, source, service, environment, method, path, status, duration_ms, request_id, error, level, message, fields, ingested_at FROM logs WHERE 1=1".to_string();
        let mut args: Vec<Box<dyn duckdb::ToSql>> = Vec::new();

        if let Some(ref s) = params.source {
            query.push_str(" AND source = ?");
            args.push(Box::new(s.clone()));
        }
        if let Some(ref l) = params.level {
            query.push_str(" AND level = ?");
            args.push(Box::new(l.clone()));
        }
        if let Some(ref f) = params.from {
            query.push_str(" AND timestamp >= ?");
            args.push(Box::new(f.to_rfc3339()));
        }
        if let Some(ref t) = params.to {
            query.push_str(" AND timestamp <= ?");
            args.push(Box::new(t.to_rfc3339()));
        }
        if let Some(ref search) = params.search {
            query.push_str(" AND lower(message) LIKE lower(?)");
            args.push(Box::new(format!("%{}%", search)));
        }
        if let Some(ref s) = params.service {
            query.push_str(" AND service = ?");
            args.push(Box::new(s.clone()));
        }
        if let Some(ref e) = params.environment {
            query.push_str(" AND environment = ?");
            args.push(Box::new(e.clone()));
        }
        if let Some(ref m) = params.method {
            query.push_str(" AND method = ?");
            args.push(Box::new(m.clone()));
        }
        if let Some(ref p) = params.path {
            query.push_str(" AND path = ?");
            args.push(Box::new(p.clone()));
        }
        if let Some(s) = params.status {
            query.push_str(" AND status = ?");
            args.push(Box::new(s));
        }
        if let Some(ref r) = params.request_id {
            query.push_str(" AND request_id = ?");
            args.push(Box::new(r.clone()));
        }
        if let Some(ref c) = params.cursor {
            query.push_str(" AND id < ?");
            args.push(Box::new(c.clone()));
        }

        // Fetch limit+1 so the caller can detect whether a next page exists
        let fetch_limit = u64::from(params.limit) + 1;
        query.push_str(" ORDER BY timestamp DESC LIMIT ?");
        args.push(Box::new(fetch_limit as i64));

        let mut stmt = conn.prepare(&query).map_err(|e| DbError::Query(e.to_string()))?;

        let sql_args: Vec<&dyn duckdb::ToSql> = args.iter().map(|b| b.as_ref()).collect();

        // col indices: 0=id,1=ts,2=source,3=service,4=env,5=method,6=path,
        //              7=status,8=duration_ms,9=request_id,10=error,
        //              11=level,12=message,13=fields,14=ingested_at
        let log_iter = stmt.query_map(sql_args.as_slice(), |row| {
            let fields_str: String = row.get(13)?;
            let fields = serde_json::from_str(&fields_str).unwrap_or(serde_json::Value::Null);

            Ok(crate::ingest::normalize::NormalizedLog {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                source: row.get(2)?,
                service: row.get(3).ok().flatten(),
                environment: row.get(4).ok().flatten(),
                method: row.get(5).ok().flatten(),
                path: row.get(6).ok().flatten(),
                status: row.get(7).ok().flatten(),
                duration_ms: row.get(8).ok().flatten(),
                request_id: row.get(9).ok().flatten(),
                error: row.get(10).ok().flatten(),
                level: row.get(11)?,
                message: row.get(12)?,
                fields,
                ingested_at: row.get(14)?,
            })
        }).map_err(|e| DbError::Query(e.to_string()))?;

        let mut logs = Vec::new();
        for log in log_iter {
            logs.push(log.map_err(|e| DbError::Query(e.to_string()))?);
        }

        Ok(logs)
    }

    /// Parsea strings tipo "30d", "24h", "60m" a minutos totales.
    pub fn parse_retention(retention: &str) -> Result<i64, DbError> {
        let re = regex::Regex::new(r"^(\d+)(d|h|m)$")
            .map_err(|e| DbError::InvalidInput(e.to_string()))?;

        let caps = re.captures(retention)
            .ok_or_else(|| DbError::InvalidInput(
                format!("Formato de retención inválido: '{}'. Usar: '30d', '24h', '60m'", retention)
            ))?;

        let num: i64 = caps[1].parse()
            .map_err(|_| DbError::InvalidInput("Número inválido".to_string()))?;

        let minutes = match &caps[2] {
            "m" => num,
            "h" => num * 60,
            "d" => num * 24 * 60,
            _ => unreachable!()
        };

        Ok(minutes)
    }

    /// Elimina logs expirados respetando retención por source.
    /// Retorna mapa source → filas eliminadas.
    pub fn delete_old_logs(&self) -> Result<std::collections::HashMap<String, usize>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;

        // 1. Default retention
        let default_str = {
            let mut stmt = conn.prepare("SELECT value FROM config WHERE key = 'retention.default'")
                .map_err(|e| DbError::Query(e.to_string()))?;
            stmt.query_row([], |row| row.get::<_, String>(0))
                .unwrap_or_else(|_| "30d".to_string())
        };
        let default_minutes = Self::parse_retention(&default_str)?;

        // 2. Sources con retención específica
        let sources: Vec<(String, Option<String>)> = {
            let mut stmt = conn.prepare("SELECT name, retention FROM sources")
                .map_err(|e| DbError::Query(e.to_string()))?;
            let iter = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
            }).map_err(|e| DbError::Query(e.to_string()))?;
            let mut v = Vec::new();
            for r in iter {
                v.push(r.map_err(|e| DbError::Query(e.to_string()))?);
            }
            v
        };

        let mut result: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        let mut sources_with_retention: Vec<String> = Vec::new();

        // 3. Retención por source
        for (name, ret_opt) in &sources {
            if let Some(ret) = ret_opt {
                let minutes = match Self::parse_retention(ret) {
                    Ok(m) => m,
                    Err(_) => continue,
                };
                let threshold = chrono::Utc::now() - chrono::Duration::minutes(minutes);
                let q = format!(
                    "DELETE FROM logs WHERE source = '{}' AND ingested_at < '{}'",
                    name, threshold.to_rfc3339()
                );
                let mut stmt = conn.prepare(&q).map_err(|e| DbError::Query(e.to_string()))?;
                let count = stmt.execute([]).map_err(|e| DbError::Query(e.to_string()))?;
                result.insert(name.clone(), count);
                sources_with_retention.push(name.clone());
            }
        }

        // 4. Retención default para el resto
        let threshold = chrono::Utc::now() - chrono::Duration::minutes(default_minutes);
        let q = if sources_with_retention.is_empty() {
            format!("DELETE FROM logs WHERE ingested_at < '{}'", threshold.to_rfc3339())
        } else {
            let quoted: Vec<String> = sources_with_retention.iter()
                .map(|s| format!("'{}'", s))
                .collect();
            format!(
                "DELETE FROM logs WHERE ingested_at < '{}' AND source NOT IN ({})",
                threshold.to_rfc3339(),
                quoted.join(", ")
            )
        };
        let mut stmt = conn.prepare(&q).map_err(|e| DbError::Query(e.to_string()))?;
        let count = stmt.execute([]).map_err(|e| DbError::Query(e.to_string()))?;
        result.insert("_default".to_string(), count);

        Ok(result)
    }

    pub fn upsert_schema(&self, entries: &[crate::ingest::schema::SchemaEntry]) -> Result<(), DbError> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let tx = conn.transaction().map_err(|e| DbError::Query(format!("Error starting transaction: {}", e)))?;

        {
            let mut stmt = tx.prepare(
                "INSERT INTO _schema (source, field_path, field_type, seen_count, last_seen) \
                 VALUES (?, ?, ?, 1, now()) \
                 ON CONFLICT (source, field_path) DO UPDATE SET \
                 seen_count = _schema.seen_count + 1, \
                 last_seen = now()"
            ).map_err(|e| DbError::Query(format!("Prepare statement error: {}", e)))?;

            for entry in entries {
                stmt.execute(duckdb::params![
                    &entry.source,
                    &entry.field_path,
                    &entry.field_type
                ]).map_err(|e| DbError::Query(format!("Execute statement error: {}", e)))?;
            }
        }

        tx.commit().map_err(|e| DbError::Query(format!("Commit transaction error: {}", e)))?;
        Ok(())
    }

    pub fn get_schema(&self, source: Option<&str>) -> Result<Vec<crate::ingest::schema::SchemaEntry>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;

        let mut args: Vec<Box<dyn duckdb::ToSql>> = Vec::new();
        let mut query = "SELECT source, field_path, field_type, seen_count, last_seen FROM _schema".to_string();

        if let Some(s) = source {
            query.push_str(" WHERE source = ?");
            args.push(Box::new(s.to_string()));
        }

        query.push_str(" ORDER BY source, field_path");

        let mut stmt = conn.prepare(&query).map_err(|e| DbError::Query(e.to_string()))?;
        let sql_args: Vec<&dyn duckdb::ToSql> = args.iter().map(|b| b.as_ref()).collect();

        let row_iter = stmt.query_map(sql_args.as_slice(), |row| {
            Ok(crate::ingest::schema::SchemaEntry {
                source: row.get(0)?,
                field_path: row.get(1)?,
                field_type: row.get(2)?,
                seen_count: row.get(3)?,
                last_seen: row.get(4).ok(),
            })
        }).map_err(|e| DbError::Query(e.to_string()))?;

        let mut entries = Vec::new();
        for r in row_iter {
            entries.push(r.map_err(|e| DbError::Query(e.to_string()))?);
        }

        Ok(entries)
    }

    pub fn list_schema_sources(&self) -> Result<Vec<String>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare("SELECT DISTINCT source FROM _schema ORDER BY source")
            .map_err(|e| DbError::Query(e.to_string()))?;

        let row_iter = stmt.query_map([], |row| {
            row.get::<_, String>(0)
        }).map_err(|e| DbError::Query(e.to_string()))?;

        let mut sources = Vec::new();
        for r in row_iter {
            sources.push(r.map_err(|e| DbError::Query(e.to_string()))?);
        }
        Ok(sources)
    }

    pub fn get_config_value(&self, key: &str) -> Result<Option<String>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare("SELECT value FROM config WHERE key = ?")
            .map_err(|e| DbError::Query(e.to_string()))?;
        
        let mut rows = stmt.query([key]).map_err(|e| DbError::Query(e.to_string()))?;
        if let Some(row) = rows.next().map_err(|e| DbError::Query(e.to_string()))? {
            let value: String = row.get(0).map_err(|e| DbError::Query(e.to_string()))?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    pub fn set_config_value(&self, key: &str, value: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare(
            "INSERT INTO config (key, value, updated_at) VALUES (?, ?, now()) \
             ON CONFLICT (key) DO UPDATE SET value = ?, updated_at = now()"
        ).map_err(|e| DbError::Query(e.to_string()))?;
        
        stmt.execute([key, value, value]).map_err(|e| DbError::Query(e.to_string()))?;
        Ok(())
    }

    pub fn get_all_config(&self) -> Result<std::collections::HashMap<String, String>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare("SELECT key, value FROM config ORDER BY key")
            .map_err(|e| DbError::Query(e.to_string()))?;
        
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        }).map_err(|e| DbError::Query(e.to_string()))?;

        let mut map = std::collections::HashMap::new();
        for row in rows {
            let (k, v) = row.map_err(|e| DbError::Query(e.to_string()))?;
            map.insert(k, v);
        }
        Ok(map)
    }

    pub fn delete_all_logs(&self) -> Result<i64, DbError> {
        let mut conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let tx = conn.transaction().map_err(|e| DbError::Query(e.to_string()))?;

        let count: i64 = tx.query_row("SELECT COUNT(*) FROM logs", [], |row| row.get(0))
            .map_err(|e| DbError::Query(e.to_string()))?;

        tx.execute("DELETE FROM logs", [])
            .map_err(|e| DbError::Query(e.to_string()))?;

        tx.commit().map_err(|e| DbError::Query(e.to_string()))?;
        Ok(count)
    }

    pub fn get_or_create_jwt_secret(&self) -> Result<String, DbError> {
        if let Some(secret) = self.get_config_value("jwt.secret")? {
            return Ok(secret);
        }

        use rand::RngCore;
        let mut bytes = [0u8; 64];
        rand::thread_rng().fill_bytes(&mut bytes);
        let secret: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
        
        self.set_config_value("jwt.secret", &secret)?;
        Ok(secret)
    }

    pub fn count_users(&self) -> Result<i64, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM users")
            .map_err(|e| DbError::Query(e.to_string()))?;
        let count: i64 = stmt.query_row([], |row| row.get(0)).map_err(|e| DbError::Query(e.to_string()))?;
        Ok(count)
    }

    pub fn create_user(&self, email: &str, password: &str, role: &str) -> Result<String, DbError> {
        use argon2::{
            password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
            Argon2
        };
        
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| DbError::Query(format!("Password hash error: {}", e)))?
            .to_string();

        let id = uuid::Uuid::new_v4().to_string();

        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare(
            "INSERT INTO users (id, email, password_hash, role, created_at) VALUES (?, ?, ?, ?, now())"
        ).map_err(|e| DbError::Query(e.to_string()))?;
        
        stmt.execute(duckdb::params![&id, email, &password_hash, role])
            .map_err(|e| DbError::Query(e.to_string()))?;
            
        Ok(id)
    }

    pub fn find_user_by_email(&self, email: &str) -> Result<Option<UserRecord>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare("SELECT id, email, password_hash, role FROM users WHERE email = ?")
            .map_err(|e| DbError::Query(e.to_string()))?;
            
        let mut rows = stmt.query([email]).map_err(|e| DbError::Query(e.to_string()))?;
        if let Some(row) = rows.next().map_err(|e| DbError::Query(e.to_string()))? {
            Ok(Some(UserRecord {
                id: row.get(0).map_err(|e| DbError::Query(e.to_string()))?,
                email: row.get(1).map_err(|e| DbError::Query(e.to_string()))?,
                password_hash: row.get(2).map_err(|e| DbError::Query(e.to_string()))?,
                role: row.get(3).map_err(|e| DbError::Query(e.to_string()))?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_last_login(&self, user_id: &str) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare("UPDATE users SET last_login = now() WHERE id = ?")
            .map_err(|e| DbError::Query(e.to_string()))?;
        stmt.execute([user_id]).map_err(|e| DbError::Query(e.to_string()))?;
        Ok(())
    }

    pub fn list_users(&self) -> Result<Vec<UserPublicRecord>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare("SELECT id, email, role, created_at, last_login FROM users ORDER BY created_at ASC")
            .map_err(|e| DbError::Query(e.to_string()))?;

        let row_iter = stmt.query_map([], |row| {
            Ok(UserPublicRecord {
                id: row.get(0)?,
                email: row.get(1)?,
                role: row.get(2)?,
                created_at: row.get(3).ok(),
                last_login: row.get(4).ok(),
            })
        }).map_err(|e| DbError::Query(e.to_string()))?;

        let mut users = Vec::new();
        for u in row_iter {
            users.push(u.map_err(|e| DbError::Query(e.to_string()))?);
        }
        Ok(users)
    }

    pub fn find_user_by_id(&self, id: &str) -> Result<Option<UserRecord>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare("SELECT id, email, password_hash, role FROM users WHERE id = ?")
            .map_err(|e| DbError::Query(e.to_string()))?;
            
        let mut rows = stmt.query([id]).map_err(|e| DbError::Query(e.to_string()))?;
        if let Some(row) = rows.next().map_err(|e| DbError::Query(e.to_string()))? {
            Ok(Some(UserRecord {
                id: row.get(0).map_err(|e| DbError::Query(e.to_string()))?,
                email: row.get(1).map_err(|e| DbError::Query(e.to_string()))?,
                password_hash: row.get(2).map_err(|e| DbError::Query(e.to_string()))?,
                role: row.get(3).map_err(|e| DbError::Query(e.to_string()))?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_user_role(&self, id: &str, role: &str) -> Result<bool, DbError> {
        if role != "admin" && role != "viewer" {
            return Err(DbError::Query("El rol debe ser admin o viewer".to_string()));
        }
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare("UPDATE users SET role = ? WHERE id = ?")
            .map_err(|e| DbError::Query(e.to_string()))?;
        let count = stmt.execute([role, id]).map_err(|e| DbError::Query(e.to_string()))?;
        Ok(count > 0)
    }

    pub fn delete_user(&self, id: &str) -> Result<bool, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare("DELETE FROM users WHERE id = ?")
            .map_err(|e| DbError::Query(e.to_string()))?;
        let count = stmt.execute([id]).map_err(|e| DbError::Query(e.to_string()))?;
        Ok(count > 0)
    }

    pub fn create_source(&self, name: &str, description: Option<&str>, retention: Option<String>) -> Result<String, DbError> {
        if name.is_empty() {
            return Err(DbError::Query("El nombre del source no puede estar vacio".to_string()));
        }
        let retention_val = retention.unwrap_or_else(|| "30d".to_string());
        Self::parse_retention(&retention_val)?;
        let id = uuid::Uuid::new_v4().to_string();
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare("INSERT INTO sources (id, name, description, retention_days, retention, created_at) VALUES (?, ?, ?, 30, ?, now())")
            .map_err(|e| DbError::Query(e.to_string()))?;
        stmt.execute(duckdb::params![&id, name, description, &retention_val])
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(id)
    }

    pub fn list_sources(&self) -> Result<Vec<SourceRecord>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, retention, \
             sampling_enabled, sampling_debug_rate, sampling_info_rate, sampling_warn_rate, \
             created_at FROM sources ORDER BY created_at DESC"
        ).map_err(|e| DbError::Query(e.to_string()))?;

        let row_iter = stmt.query_map([], |row| {
            Ok(SourceRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                retention: row.get(3)?,
                sampling_enabled: row.get(4).ok().flatten(),
                sampling_debug_rate: row.get(5).ok().flatten(),
                sampling_info_rate: row.get(6).ok().flatten(),
                sampling_warn_rate: row.get(7).ok().flatten(),
                created_at: row.get(8)?,
            })
        }).map_err(|e| DbError::Query(e.to_string()))?;

        let mut sources = Vec::new();
        for s in row_iter {
            sources.push(s.map_err(|e| DbError::Query(e.to_string()))?);
        }
        Ok(sources)
    }

    pub fn update_source(&self, id: &str, params: &UpdateSourceParams) -> Result<bool, DbError> {
        if let Some(ref r) = params.retention {
            Self::parse_retention(r)?;
        }
        for rate in [params.sampling_debug_rate, params.sampling_info_rate, params.sampling_warn_rate].iter().flatten() {
            if *rate < 0 || *rate > 100 {
                return Err(DbError::InvalidInput("sampling rate must be 0–100".to_string()));
            }
        }

        let mut sets: Vec<String> = Vec::new();
        let mut args: Vec<Box<dyn duckdb::ToSql>> = Vec::new();

        if let Some(ref r) = params.retention {
            sets.push("retention = ?".to_string());
            args.push(Box::new(r.clone()));
        }
        if let Some(se) = params.sampling_enabled {
            sets.push("sampling_enabled = ?".to_string());
            args.push(Box::new(se));
        }
        if let Some(r) = params.sampling_debug_rate {
            sets.push("sampling_debug_rate = ?".to_string());
            args.push(Box::new(r));
        }
        if let Some(r) = params.sampling_info_rate {
            sets.push("sampling_info_rate = ?".to_string());
            args.push(Box::new(r));
        }
        if let Some(r) = params.sampling_warn_rate {
            sets.push("sampling_warn_rate = ?".to_string());
            args.push(Box::new(r));
        }

        if sets.is_empty() {
            return Ok(true);
        }

        args.push(Box::new(id.to_string()));
        let q = format!("UPDATE sources SET {} WHERE id = ?", sets.join(", "));
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare(&q).map_err(|e| DbError::Query(e.to_string()))?;
        let sql_args: Vec<&dyn duckdb::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let count = stmt.execute(sql_args.as_slice()).map_err(|e| DbError::Query(e.to_string()))?;
        Ok(count > 0)
    }

    pub fn delete_source_logs(&self, id: &str) -> Result<i64, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let name: String = {
            let mut stmt = conn.prepare("SELECT name FROM sources WHERE id = ?")
                .map_err(|e| DbError::Query(e.to_string()))?;
            let mut rows = stmt.query([id]).map_err(|e| DbError::Query(e.to_string()))?;
            match rows.next().map_err(|e| DbError::Query(e.to_string()))? {
                Some(row) => row.get(0).map_err(|e| DbError::Query(e.to_string()))?,
                None => return Err(DbError::Query("Source not found".to_string())),
            }
        };
        let count: i64 = {
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM logs WHERE source = ?")
                .map_err(|e| DbError::Query(e.to_string()))?;
            stmt.query_row([&name], |row| row.get(0)).map_err(|e| DbError::Query(e.to_string()))?
        };
        let mut stmt = conn.prepare("DELETE FROM logs WHERE source = ?")
            .map_err(|e| DbError::Query(e.to_string()))?;
        stmt.execute([&name]).map_err(|e| DbError::Query(e.to_string()))?;
        Ok(count)
    }

    pub fn delete_source(&self, id: &str) -> Result<bool, DbError> {
        let mut conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let tx = conn.transaction().map_err(|e| DbError::Query(format!("Transaction error: {}", e)))?;

        // 1. Obtener nombre del source para limpiar tablas relacionadas
        let name_res: Result<String, _> = tx.query_row("SELECT name FROM sources WHERE id = ?", [id], |row| row.get(0));
        
        let name = match name_res {
            Ok(n) => n,
            Err(duckdb::Error::QueryReturnedNoRows) => return Ok(false),
            Err(e) => return Err(DbError::Query(e.to_string())),
        };

        // 2. Limpiar logs
        tx.execute("DELETE FROM logs WHERE source = ?", [&name])
            .map_err(|e| DbError::Query(format!("logs cleanup: {}", e)))?;

        // 3. Limpiar tokens
        tx.execute("DELETE FROM ingest_tokens WHERE source = ?", [&name])
            .map_err(|e| DbError::Query(format!("tokens cleanup: {}", e)))?;

        // 4. Limpiar esquema
        tx.execute("DELETE FROM _schema WHERE source = ?", [&name])
            .map_err(|e| DbError::Query(format!("schema cleanup: {}", e)))?;

        // 5. Eliminar source
        let count = tx.execute("DELETE FROM sources WHERE id = ?", [id])
            .map_err(|e| DbError::Query(format!("source deletion: {}", e)))?;

        tx.commit().map_err(|e| DbError::Query(format!("commit deletion: {}", e)))?;
        Ok(count > 0)
    }

    pub fn create_ingest_token(&self, name: &str, source_name: &str, created_by: &str) -> Result<(String, String), DbError> {
        use rand::RngCore;
        use sha2::{Sha256, Digest};
        
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        let token_real = format!("tok_{}", hex::encode(bytes));
        
        let mut hasher = Sha256::new();
        hasher.update(token_real.as_bytes());
        let token_hash = hex::encode(hasher.finalize());
        
        let id = uuid::Uuid::new_v4().to_string();
        
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare(
            "INSERT INTO ingest_tokens (id, name, token_hash, source, created_by, created_at) VALUES (?, ?, ?, ?, ?, now())"
        ).map_err(|e| DbError::Query(e.to_string()))?;
            
        stmt.execute(duckdb::params![&id, name, &token_hash, source_name, created_by])
            .map_err(|e| DbError::Query(e.to_string()))?;
            
        Ok((id, token_real))
    }

    pub fn list_ingest_tokens(&self, source_id: Option<&str>) -> Result<Vec<IngestTokenRecord>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        
        let mut query = "SELECT id, name, source, created_by, created_at, last_used, revoked_at FROM ingest_tokens".to_string();
        let mut args: Vec<Box<dyn duckdb::ToSql>> = Vec::new();
        
        if let Some(s) = source_id {
            query.push_str(" WHERE source = (SELECT name FROM sources WHERE id = ?)");
            args.push(Box::new(s.to_string()));
        }
        query.push_str(" ORDER BY created_at DESC");
        
        let mut stmt = conn.prepare(&query).map_err(|e| DbError::Query(e.to_string()))?;
        let sql_args: Vec<&dyn duckdb::ToSql> = args.iter().map(|b| b.as_ref()).collect();
            
        let row_iter = stmt.query_map(sql_args.as_slice(), |row| {
            Ok(IngestTokenRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                source: row.get(2)?,
                created_by: row.get(3)?,
                created_at: row.get(4)?,
                last_used: row.get(5)?,
                revoked_at: row.get(6)?,
            })
        }).map_err(|e| DbError::Query(e.to_string()))?;

        let mut tokens = Vec::new();
        for t in row_iter {
            tokens.push(t.map_err(|e| DbError::Query(e.to_string()))?);
        }
        Ok(tokens)
    }

    pub fn revoke_ingest_token(&self, id: &str) -> Result<bool, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        
        // Usamos RETURNING id para confirmar que se actualizó una fila
        let mut stmt = conn.prepare("UPDATE ingest_tokens SET revoked_at = now() WHERE id = ? AND revoked_at IS NULL RETURNING id").map_err(|e| DbError::Query(e.to_string()))?;
        let mut rows = stmt.query([id]).map_err(|e| DbError::Query(e.to_string()))?;
        
        let success = rows.next().map_err(|e| DbError::Query(e.to_string()))?.is_some();
        if !success {
            // Log para debug (esto saldrá en los logs de la app)
            tracing::warn!("Fallo al revocar token: id {} no encontrado o ya revocado", id);
        }
        Ok(success)
    }

    pub fn verify_ingest_token(&self, raw_token: &str) -> Result<Option<String>, DbError> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(raw_token.as_bytes());
        let token_hash = hex::encode(hasher.finalize());
        
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        
        let mut stmt = conn.prepare("SELECT source FROM ingest_tokens WHERE token_hash = ? AND revoked_at IS NULL")
            .map_err(|e| DbError::Query(e.to_string()))?;
            
        let mut rows = stmt.query([&token_hash]).map_err(|e| DbError::Query(e.to_string()))?;
        if let Some(row) = rows.next().map_err(|e| DbError::Query(e.to_string()))? {
            let source: String = row.get(0).map_err(|e| DbError::Query(e.to_string()))?;
            let mut upd_stmt = conn.prepare("UPDATE ingest_tokens SET last_used = now() WHERE token_hash = ?").map_err(|e| DbError::Query(e.to_string()))?;
            let _ = upd_stmt.execute([&token_hash]);
            Ok(Some(source))
        } else {
            Ok(None)
        }
    }

    pub fn analytics_volume(
        &self,
        source: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        interval: &str,
    ) -> Result<Vec<VolumePoint>, DbError> {
        match interval {
            "minute" | "hour" | "day" | "week" => {}
            _ => return Err(DbError::InvalidInput("invalid interval".to_string())),
        }

        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;

        // interval has been validated above – safe to interpolate
        let mut query = format!(
            "SELECT CAST(DATE_TRUNC('{}', CAST(timestamp AS TIMESTAMP)) AS VARCHAR) AS bucket, COUNT(*) AS count \
             FROM logs WHERE 1=1",
            interval
        );
        let mut args: Vec<Box<dyn duckdb::ToSql>> = Vec::new();

        if let Some(s) = source {
            query.push_str(" AND source = ?");
            args.push(Box::new(s.to_string()));
        }
        if let Some(f) = from {
            query.push_str(" AND CAST(timestamp AS TIMESTAMP) >= CAST(? AS TIMESTAMP)");
            args.push(Box::new(f.to_rfc3339()));
        }
        if let Some(t) = to {
            query.push_str(" AND CAST(timestamp AS TIMESTAMP) <= CAST(? AS TIMESTAMP)");
            args.push(Box::new(t.to_rfc3339()));
        }
        query.push_str(" GROUP BY bucket ORDER BY bucket ASC");

        let mut stmt = conn.prepare(&query).map_err(|e| DbError::Query(e.to_string()))?;
        let sql_args: Vec<&dyn duckdb::ToSql> = args.iter().map(|b| b.as_ref()).collect();

        let row_iter = stmt.query_map(sql_args.as_slice(), |row| {
            let bucket_str: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((bucket_str, count))
        }).map_err(|e| DbError::Query(e.to_string()))?;

        let mut points = Vec::new();
        for r in row_iter {
            let (bucket_str, count) = r.map_err(|e| DbError::Query(e.to_string()))?;
            // DuckDB may return timestamps in various formats; try RFC3339 first then a plain format
            let bucket = chrono::DateTime::parse_from_rfc3339(&bucket_str)
                .map(|d| d.with_timezone(&Utc))
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(&bucket_str, "%Y-%m-%d %H:%M:%S")
                        .map(|ndt| ndt.and_utc())
                })
                .or_else(|_| {
                    chrono::NaiveDateTime::parse_from_str(&bucket_str, "%Y-%m-%dT%H:%M:%S")
                        .map(|ndt| ndt.and_utc())
                })
                .map_err(|e| DbError::Query(format!("bucket parse error: {}", e)))?;
            points.push(VolumePoint { bucket, count });
        }
        Ok(points)
    }

    pub fn analytics_error_rate(
        &self,
        source: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
    ) -> Result<ErrorRateResult, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;

        // Build shared WHERE clause (after WHERE 1=1)
        let mut where_suffix = String::new();
        let mut args: Vec<Box<dyn duckdb::ToSql>> = Vec::new();

        if let Some(s) = source {
            where_suffix.push_str(" AND source = ?");
            args.push(Box::new(s.to_string()));
        }
        if let Some(f) = from {
            where_suffix.push_str(" AND CAST(timestamp AS TIMESTAMP) >= CAST(? AS TIMESTAMP)");
            args.push(Box::new(f.to_rfc3339()));
        }
        if let Some(t) = to {
            where_suffix.push_str(" AND CAST(timestamp AS TIMESTAMP) <= CAST(? AS TIMESTAMP)");
            args.push(Box::new(t.to_rfc3339()));
        }

        // Total count
        let total_query = format!("SELECT COUNT(*) FROM logs WHERE 1=1{}", where_suffix);
        let sql_args: Vec<&dyn duckdb::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let mut stmt = conn.prepare(&total_query).map_err(|e| DbError::Query(e.to_string()))?;
        let total: i64 = stmt.query_row(sql_args.as_slice(), |row| row.get(0))
            .map_err(|e| DbError::Query(e.to_string()))?;

        // Error count (level IN ('error','fatal'))
        let errors_query = format!(
            "SELECT COUNT(*) FROM logs WHERE 1=1{} AND level IN ('error','fatal')",
            where_suffix
        );
        let sql_args2: Vec<&dyn duckdb::ToSql> = args.iter().map(|b| b.as_ref()).collect();
        let mut stmt2 = conn.prepare(&errors_query).map_err(|e| DbError::Query(e.to_string()))?;
        let errors: i64 = stmt2.query_row(sql_args2.as_slice(), |row| row.get(0))
            .map_err(|e| DbError::Query(e.to_string()))?;

        let raw_rate = if total == 0 { 0.0 } else { errors as f64 / total as f64 };
        let rate = (raw_rate * 10_000.0).round() / 10_000.0;

        Ok(ErrorRateResult { total, errors, rate })
    }

    /// Returns logs that arrived after a given cutoff, ordered by ingested_at ASC.
    /// Used by the polling endpoint: "give me what arrived since the last log I saw".
    pub fn poll_logs(
        &self,
        source: Option<&str>,
        level: Option<&str>,
        search: Option<&str>,
        since_id: Option<&str>,
        since_timestamp: Option<DateTime<Utc>>,
        limit: u32,
    ) -> Result<Vec<LogRecord>, DbError> {
        let conn = self.conn.lock()
            .map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;

        // Build the WHERE clause for the ingested_at cutoff.
        //
        // When since_id is given we use a SQL subquery so DuckDB compares TIMESTAMPTZ
        // natively, avoiding any Rust-side timestamp parsing issues.
        // The COALESCE handles the case where the ID is not found: it falls through to
        // the bound fallback timestamp.
        let mut args: Vec<Box<dyn duckdb::ToSql>> = Vec::new();

        let cutoff_clause: String;
        if let Some(id) = since_id {
            // Fallback: since_timestamp or now()-30s (as RFC3339 which DuckDB accepts)
            let fallback = since_timestamp
                .unwrap_or_else(|| Utc::now() - chrono::Duration::seconds(30))
                .to_rfc3339();
            // COALESCE: use the referenced log's ingested_at; if not found use fallback
            cutoff_clause = "ingested_at > COALESCE(\
                (SELECT ingested_at FROM logs WHERE id = ?), \
                CAST(? AS TIMESTAMPTZ)\
            )".to_string();
            args.push(Box::new(id.to_string()));
            args.push(Box::new(fallback));
        } else {
            let cutoff = since_timestamp
                .unwrap_or_else(|| Utc::now() - chrono::Duration::seconds(30))
                .to_rfc3339();
            cutoff_clause = "ingested_at > CAST(? AS TIMESTAMPTZ)".to_string();
            args.push(Box::new(cutoff));
        }

        // --- Build full query ---
        let mut query = format!(
            "SELECT id, timestamp, source, service, environment, method, path, \
             status, duration_ms, request_id, error, level, message, fields, ingested_at \
             FROM logs WHERE {}",
            cutoff_clause
        );

        if let Some(s) = source {
            query.push_str(" AND source = ?");
            args.push(Box::new(s.to_string()));
        }
        if let Some(l) = level {
            query.push_str(" AND level = ?");
            args.push(Box::new(l.to_string()));
        }
        if let Some(s) = search {
            query.push_str(" AND lower(message) LIKE lower(?)");
            args.push(Box::new(format!("%{}%", s)));
        }

        query.push_str(" ORDER BY ingested_at ASC LIMIT ?");
        args.push(Box::new(i64::from(limit)));

        let mut stmt = conn.prepare(&query).map_err(|e| DbError::Query(e.to_string()))?;
        let sql_args: Vec<&dyn duckdb::ToSql> = args.iter().map(|b| b.as_ref()).collect();

        let row_iter = stmt.query_map(sql_args.as_slice(), |row| {
            // col indices: 0=id,1=ts,2=source,3=service,4=env,5=method,6=path,
            //              7=status,8=duration_ms,9=request_id,10=error,
            //              11=level,12=message,13=fields,14=ingested_at
            Ok(LogRecord {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                source: row.get(2)?,
                service: row.get(3).ok().flatten(),
                environment: row.get(4).ok().flatten(),
                method: row.get(5).ok().flatten(),
                path: row.get(6).ok().flatten(),
                status: row.get(7).ok().flatten(),
                duration_ms: row.get(8).ok().flatten(),
                request_id: row.get(9).ok().flatten(),
                error: row.get(10).ok().flatten(),
                level: row.get(11).ok(),
                message: row.get(12).ok(),
                fields: {
                    let s: String = row.get(13)?;
                    serde_json::from_str(&s).unwrap_or_default()
                },
                ingested_at: row.get(14)?,
            })
        }).map_err(|e| DbError::Query(e.to_string()))?;

        let mut records = Vec::new();
        for r in row_iter {
            records.push(r.map_err(|e| DbError::Query(e.to_string()))?);
        }
        Ok(records)
    }

    pub fn query_raw_flexible(&self, sql: &str) -> Result<Vec<serde_json::Value>, DbError> {
        // Recover from a previously poisoned mutex instead of propagating the error.
        // A poisoned mutex means a previous thread panicked while holding the guard;
        // the Connection itself is still usable in DuckDB's case.
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());

        // STEP 1 — get column names via DESCRIBE so we never call column_names()
        // on an un-executed statement (which panics internally via .expect()).
        let describe_sql = format!("DESCRIBE {}", sql.trim_end_matches(';'));
        let col_names: Vec<String> = {
            let mut d = conn.prepare(&describe_sql)
                .map_err(|e| DbError::Query(format!("DESCRIBE: {}", e)))?;
            let iter = d.query_map([], |row| row.get::<_, String>(0))
                .map_err(|e| DbError::Query(e.to_string()))?;
            let mut names = Vec::new();
            for r in iter {
                names.push(r.map_err(|e| DbError::Query(e.to_string()))?);
            }
            names
        };


        // STEP 2 — execute the real query with query_map (no column_names() call)
        let mut stmt = conn.prepare(sql).map_err(|e| DbError::Query(e.to_string()))?;
        let iter = stmt.query_map([], |row| {
            let mut map = serde_json::Map::new();
            for (i, col_name) in col_names.iter().enumerate() {
                let val: serde_json::Value =
                    if let Ok(Some(v)) = row.get::<_, Option<f64>>(i) { serde_json::json!(v) }
                    else if let Ok(Some(v)) = row.get::<_, Option<i64>>(i) { serde_json::json!(v) }
                    else if let Ok(Some(v)) = row.get::<_, Option<String>>(i) { serde_json::json!(v) }
                    else { serde_json::Value::Null };
                map.insert(col_name.clone(), val);
            }
            Ok(serde_json::Value::Object(map))
        }).map_err(|e| DbError::Query(e.to_string()))?;

        let mut results = Vec::new();
        for r in iter {
            results.push(r.map_err(|e| DbError::Query(e.to_string()))?);
        }
        Ok(results)
    }

    pub fn query_raw(&self, sql: &str) -> Result<Vec<crate::ingest::normalize::NormalizedLog>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare(sql).map_err(|e| DbError::Query(e.to_string()))?;
        // Expected col order (from buildAdvancedSql):
        // 0=id,1=ts,2=source,3=service,4=env,5=method,6=path,
        // 7=status,8=duration_ms,9=request_id,10=error,
        // 11=level,12=message,13=fields,14=ingested_at
        let log_iter = stmt.query_map([], |row| {
            let fields_str: String = row.get(13)?;
            let fields = serde_json::from_str(&fields_str).unwrap_or(serde_json::Value::Null);
            Ok(crate::ingest::normalize::NormalizedLog {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                source: row.get(2)?,
                service: row.get(3).ok().flatten(),
                environment: row.get(4).ok().flatten(),
                method: row.get(5).ok().flatten(),
                path: row.get(6).ok().flatten(),
                status: row.get(7).ok().flatten(),
                duration_ms: row.get(8).ok().flatten(),
                request_id: row.get(9).ok().flatten(),
                error: row.get(10).ok().flatten(),
                level: row.get(11)?,
                message: row.get(12)?,
                fields,
                ingested_at: row.get(14)?,
            })
        }).map_err(|e| DbError::Query(e.to_string()))?;
        let mut logs = Vec::new();
        for log in log_iter {
            logs.push(log.map_err(|e| DbError::Query(e.to_string()))?);
        }
        Ok(logs)
    }

    // ─── Dashboard helpers ───────────────────────────────────────────────────

    pub fn list_dashboards(&self) -> Result<Vec<Dashboard>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare(
            "SELECT d.id, d.name, d.description, d.created_by, \
             CAST(d.created_at AS VARCHAR), CAST(d.updated_at AS VARCHAR), \
             (SELECT COUNT(*) FROM widgets w WHERE w.dashboard_id = d.id) as widget_count \
             FROM dashboards d ORDER BY d.created_at DESC"
        ).map_err(|e| DbError::Query(e.to_string()))?;

        let iter = stmt.query_map([], |row| {
            Ok(Dashboard {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                created_by: row.get(3)?,
                created_at: row.get::<_, Option<String>>(4)?.unwrap_or_default(),
                updated_at: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                widget_count: row.get(6).ok(),
            })
        }).map_err(|e| DbError::Query(e.to_string()))?;

        let mut out = Vec::new();
        for r in iter {
            out.push(r.map_err(|e| DbError::Query(e.to_string()))?);
        }
        Ok(out)
    }

    pub fn get_dashboard(&self, id: &str) -> Result<Option<Dashboard>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare(
            "SELECT id, name, description, created_by, \
             CAST(created_at AS VARCHAR), CAST(updated_at AS VARCHAR) \
             FROM dashboards WHERE id = ?"
        ).map_err(|e| DbError::Query(e.to_string()))?;

        let mut rows = stmt.query([id]).map_err(|e| DbError::Query(e.to_string()))?;
        if let Some(row) = rows.next().map_err(|e| DbError::Query(e.to_string()))? {
            Ok(Some(Dashboard {
                id: row.get(0).map_err(|e| DbError::Query(e.to_string()))?,
                name: row.get(1).map_err(|e| DbError::Query(e.to_string()))?,
                description: row.get(2).map_err(|e| DbError::Query(e.to_string()))?,
                created_by: row.get(3).map_err(|e| DbError::Query(e.to_string()))?,
                created_at: row.get::<_, Option<String>>(4).map_err(|e| DbError::Query(e.to_string()))?.unwrap_or_default(),
                updated_at: row.get::<_, Option<String>>(5).map_err(|e| DbError::Query(e.to_string()))?.unwrap_or_default(),
                widget_count: None,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn create_dashboard(&self, name: &str, description: Option<&str>, created_by: &str) -> Result<String, DbError> {
        let id = uuid::Uuid::new_v4().to_string();
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare(
            "INSERT INTO dashboards (id, name, description, created_by, created_at, updated_at) \
             VALUES (?, ?, ?, ?, now(), now())"
        ).map_err(|e| DbError::Query(e.to_string()))?;
        stmt.execute(duckdb::params![&id, name, description, created_by])
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(id)
    }

    pub fn update_dashboard(&self, id: &str, name: &str, description: Option<&str>) -> Result<bool, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare(
            "UPDATE dashboards SET name = ?, description = ?, updated_at = now() WHERE id = ?"
        ).map_err(|e| DbError::Query(e.to_string()))?;
        let count = stmt.execute(duckdb::params![name, description, id])
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(count > 0)
    }

    pub fn delete_dashboard(&self, id: &str) -> Result<bool, DbError> {
        let mut conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let tx = conn.transaction().map_err(|e| DbError::Query(e.to_string()))?;
        tx.execute("DELETE FROM widgets WHERE dashboard_id = ?", [id])
            .map_err(|e| DbError::Query(format!("widgets cleanup: {}", e)))?;
        let count = tx.execute("DELETE FROM dashboards WHERE id = ?", [id])
            .map_err(|e| DbError::Query(format!("dashboard delete: {}", e)))?;
        tx.commit().map_err(|e| DbError::Query(e.to_string()))?;
        Ok(count > 0)
    }

    pub fn list_widgets(&self, dashboard_id: &str) -> Result<Vec<Widget>, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare(
            r#"SELECT id, dashboard_id, title, "type", width, position, config
               FROM widgets WHERE dashboard_id = ? ORDER BY position ASC"#
        ).map_err(|e| DbError::Query(e.to_string()))?;

        let iter = stmt.query_map([dashboard_id], |row| {
            Ok(Widget {
                id: row.get(0)?,
                dashboard_id: row.get(1)?,
                title: row.get(2)?,
                widget_type: row.get(3)?,
                width: row.get(4)?,
                position: row.get(5)?,
                config: row.get(6)?,
            })
        }).map_err(|e| DbError::Query(e.to_string()))?;

        let mut out = Vec::new();
        for r in iter {
            out.push(r.map_err(|e| DbError::Query(e.to_string()))?);
        }
        Ok(out)
    }

    pub fn create_widget(
        &self,
        dashboard_id: &str,
        title: &str,
        widget_type: &str,
        width: &str,
        position: i32,
        config: &str,
    ) -> Result<String, DbError> {
        let id = uuid::Uuid::new_v4().to_string();
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare(
            r#"INSERT INTO widgets (id, dashboard_id, title, "type", width, position, config, created_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, now())"#
        ).map_err(|e| DbError::Query(e.to_string()))?;
        stmt.execute(duckdb::params![&id, dashboard_id, title, widget_type, width, position, config])
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(id)
    }

    pub fn update_widget(
        &self,
        id: &str,
        title: &str,
        widget_type: &str,
        width: &str,
        config: &str,
    ) -> Result<bool, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare(
            r#"UPDATE widgets SET title = ?, "type" = ?, width = ?, config = ? WHERE id = ?"#
        ).map_err(|e| DbError::Query(e.to_string()))?;
        let count = stmt.execute(duckdb::params![title, widget_type, width, config, id])
            .map_err(|e| DbError::Query(e.to_string()))?;
        Ok(count > 0)
    }

    pub fn update_widget_positions(&self, positions: &[(String, i32)]) -> Result<(), DbError> {
        if positions.is_empty() {
            return Ok(());
        }
        let mut conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let tx = conn.transaction().map_err(|e| DbError::Query(e.to_string()))?;
        {
            let mut stmt = tx.prepare("UPDATE widgets SET position = ? WHERE id = ?")
                .map_err(|e| DbError::Query(e.to_string()))?;
            for (widget_id, pos) in positions {
                stmt.execute(duckdb::params![pos, widget_id])
                    .map_err(|e| DbError::Query(e.to_string()))?;
            }
        }
        tx.commit().map_err(|e| DbError::Query(e.to_string()))?;
        Ok(())
    }

    pub fn delete_widget(&self, id: &str) -> Result<bool, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare("DELETE FROM widgets WHERE id = ?")
            .map_err(|e| DbError::Query(e.to_string()))?;
        let count = stmt.execute([id]).map_err(|e| DbError::Query(e.to_string()))?;
        Ok(count > 0)
    }

    // ─── S3 sync helpers ────────────────────────────────────────────────────

    /// Construye un `S3Client` a partir de la config guardada en el struct.
    fn build_s3_client(cfg: &S3Cfg) -> S3Client {
        let creds = Credentials::new(
            &cfg.access_key_id,
            &cfg.secret_access_key,
            None,
            None,
            "evlogstudio",
        );
        let region = Region::new(cfg.region.clone());
        let mut builder = aws_sdk_s3::Config::builder()
            .credentials_provider(creds)
            .region(region)
            .behavior_version_latest();
        if let Some(ref ep) = cfg.endpoint {
            builder = builder.endpoint_url(ep.clone());
        }
        S3Client::from_conf(builder.build())
    }

    /// Exporta la tabla `logs` a Parquet y sube el archivo a S3.
    /// No-op silencioso si el modo no es s3.
    pub async fn sync_to_s3(&self) -> Result<(), DbError> {
        let cfg = match self.s3_cfg.as_ref() {
            Some(c) => c,
            None => return Ok(()),
        };

        // 1. Exportar a Parquet local
        let export_path = "/tmp/evlogstudio_export.parquet";
        let row_count: i64 = {
            let conn = self.conn.lock()
                .map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
            conn.execute_batch(&format!(
                "COPY (SELECT * FROM logs) TO '{}' (FORMAT PARQUET)",
                export_path
            )).map_err(|e| DbError::Query(format!("COPY to parquet: {}", e)))?;
            let mut stmt = conn.prepare("SELECT COUNT(*) FROM logs")
                .map_err(|e| DbError::Query(e.to_string()))?;
            stmt.query_row([], |row| row.get(0))
                .map_err(|e| DbError::Query(e.to_string()))?
        };

        // 2. Subir a S3
        let client = Self::build_s3_client(cfg);
        let file = tokio::fs::File::open(export_path).await
            .map_err(|e| DbError::Query(format!("S3 sync: abrir parquet: {}", e)))?;
        let file_len = file.metadata().await
            .map_err(|e| DbError::Query(format!("S3 sync: metadata: {}", e)))?;
        let byte_stream = aws_sdk_s3::primitives::ByteStream::read_from()
            .path(export_path)
            .build()
            .await
            .map_err(|e| DbError::Query(format!("S3 sync: ByteStream: {}", e)))?;
        client
            .put_object()
            .bucket(&cfg.bucket)
            .key("logs/latest.parquet")
            .content_length(file_len.len() as i64)
            .body(byte_stream)
            .send()
            .await
            .map_err(|e| DbError::Query(format!("S3 sync: put_object: {}", e)))?;

        tracing::info!("S3 sync: exportadas {} filas a s3://{}/logs/latest.parquet", row_count, cfg.bucket);
        Ok(())
    }

    /// Descarga `logs/latest.parquet` de S3 e importa a la tabla `logs`.
    /// Si el objeto no existe (primer arranque) retorna Ok sin error.
    /// No-op silencioso si el modo no es s3.
    pub async fn load_from_s3(&self) -> Result<(), DbError> {
        let cfg = match self.s3_cfg.as_ref() {
            Some(c) => c,
            None => return Ok(()),
        };

        let client = Self::build_s3_client(cfg);
        let download_path = "/tmp/evlogstudio_download.parquet";

        // 1. Intentar descargar
        let get_result = client
            .get_object()
            .bucket(&cfg.bucket)
            .key("logs/latest.parquet")
            .send()
            .await;

        let response = match get_result {
            Ok(r) => r,
            Err(SdkError::ServiceError(se)) if matches!(se.err(), GetObjectError::NoSuchKey(_)) => {
                tracing::info!("S3 load: no existe logs/latest.parquet — primer arranque");
                return Ok(());
            }
            Err(e) => {
                return Err(DbError::Query(format!("S3 load: get_object: {}", e)));
            }
        };

        // 2. Guardar el archivo
        let bytes = response.body.collect().await
            .map_err(|e| DbError::Query(format!("S3 load: leer body: {}", e)))?;
        let mut file = tokio::fs::File::create(download_path).await
            .map_err(|e| DbError::Query(format!("S3 load: crear archivo: {}", e)))?;
        file.write_all(&bytes.into_bytes()).await
            .map_err(|e| DbError::Query(format!("S3 load: escribir archivo: {}", e)))?;
        file.flush().await
            .map_err(|e| DbError::Query(format!("S3 load: flush: {}", e)))?;
        drop(file);

        // 3. Importar a la tabla
        {
            let conn = self.conn.lock()
                .map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
            conn.execute_batch(&format!(
                "INSERT INTO logs SELECT * FROM read_parquet('{}')",
                download_path
            )).map_err(|e| DbError::Query(format!("S3 load: insert from parquet: {}", e)))?;
        }

        tracing::info!("S3 load: logs restaurados desde s3://{}/logs/latest.parquet", cfg.bucket);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_retention_days() {
        assert_eq!(Db::parse_retention("30d").unwrap(), 43200);
    }

    #[test]
    fn test_parse_retention_hours() {
        assert_eq!(Db::parse_retention("24h").unwrap(), 1440);
    }

    #[test]
    fn test_parse_retention_minutes() {
        assert_eq!(Db::parse_retention("60m").unwrap(), 60);
    }

    #[test]
    fn test_parse_retention_invalid_alpha() {
        assert!(Db::parse_retention("abc").is_err());
    }

    #[test]
    fn test_parse_retention_invalid_unit() {
        assert!(Db::parse_retention("30x").is_err());
    }

    #[test]
    fn test_parse_retention_empty() {
        assert!(Db::parse_retention("").is_err());
    }
}

// ─── Dashboard / Widget structs ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub created_by: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub widget_count: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Widget {
    pub id: String,
    pub dashboard_id: String,
    pub title: String,
    #[serde(rename = "type")]
    pub widget_type: String,
    pub width: String,
    pub position: i32,
    pub config: String,
}

#[derive(Debug, Clone)]
pub struct LogQueryParams {
    pub source: Option<String>,
    pub level: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub search: Option<String>,
    pub service: Option<String>,
    pub environment: Option<String>,
    pub method: Option<String>,
    pub path: Option<String>,
    pub status: Option<i32>,
    pub request_id: Option<String>,
    pub limit: u32,
    pub cursor: Option<String>,
}
