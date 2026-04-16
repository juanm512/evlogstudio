pub mod schema;

use crate::config::Config;
use duckdb::Connection;
use std::sync::Mutex;
use chrono::{DateTime, Utc};

#[derive(thiserror::Error, Debug)]
pub enum DbError {
    #[error("Error al abrir base de datos: {0}")]
    Open(String),
    #[error("Error de migracion: {0}")]
    Migration(String),
    #[error("Error de consulta: {0}")]
    Query(String),
}

pub struct Db {
    conn: Mutex<Connection>,
}

impl Db {
    pub fn open(config: &Config) -> Result<Self, DbError> {
        if config.storage_mode != "local" {
            return Err(DbError::Open("Solo se soporta storage_mode = 'local'".to_string()));
        }

        let conn = Connection::open(&config.data_path)
            .map_err(|e| DbError::Open(e.to_string()))?;

        let db = Db {
            conn: Mutex::new(conn),
        };

        db.run_migrations()?;
        db.seed_config()?;

        Ok(db)
    }

    pub fn run_migrations(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Migration(format!("Mutex envenenado: {}", e)))?;

        conn.execute_batch(schema::CREATE_LOGS)
            .map_err(|e| DbError::Migration(format!("logs: {}", e)))?;

        conn.execute_batch(schema::CREATE_SCHEMA_INFERENCE)
            .map_err(|e| DbError::Migration(format!("_schema: {}", e)))?;

        conn.execute_batch(schema::CREATE_SOURCES)
            .map_err(|e| DbError::Migration(format!("sources: {}", e)))?;

        conn.execute_batch(schema::CREATE_INGEST_TOKENS)
            .map_err(|e| DbError::Migration(format!("ingest_tokens: {}", e)))?;

        conn.execute_batch(schema::CREATE_USERS)
            .map_err(|e| DbError::Migration(format!("users: {}", e)))?;

        conn.execute_batch(schema::CREATE_CONFIG)
            .map_err(|e| DbError::Migration(format!("config: {}", e)))?;

        Ok(())
    }

    pub fn seed_config(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;

        conn.execute(
            "INSERT INTO config (key, value) VALUES ('retention.default_days', '30') ON CONFLICT (key) DO NOTHING",
            []
        ).map_err(|e| DbError::Query(format!("seed retention.default_days: {}", e)))?;

        conn.execute(
            "INSERT INTO config (key, value) VALUES ('sampling.enabled', 'false') ON CONFLICT (key) DO NOTHING",
            []
        ).map_err(|e| DbError::Query(format!("seed sampling.enabled: {}", e)))?;

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
            let mut stmt = tx.prepare("INSERT INTO logs (id, timestamp, source, level, message, fields, ingested_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
                .map_err(|e| DbError::Query(format!("Prepare statement error: {}", e)))?;

            for log in logs {
                let fields_json = log.fields.to_string(); // fields serializado a JSON
                let ts_str = log.timestamp.to_rfc3339();
                let ing_str = log.ingested_at.to_rfc3339();
                
                stmt.execute(duckdb::params![
                    &log.id,
                    &ts_str,
                    &log.source,
                    &log.level,
                    &log.message,
                    &fields_json,
                    &ing_str
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
        
        let mut query = "SELECT id, CAST(timestamp AS VARCHAR), source, level, message, fields, CAST(ingested_at AS VARCHAR) FROM logs WHERE 1=1".to_string();
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
            query.push_str(" AND message LIKE ?");
            args.push(Box::new(format!("%{}%", search)));
        }
        if let Some(ref c) = params.cursor {
            query.push_str(" AND id < ?");
            args.push(Box::new(c.clone()));
        }

        query.push_str(" ORDER BY timestamp DESC LIMIT ?");
        args.push(Box::new(params.limit));

        let mut stmt = conn.prepare(&query).map_err(|e| DbError::Query(e.to_string()))?;

        let sql_args: Vec<&dyn duckdb::ToSql> = args.iter().map(|b| b.as_ref()).collect();

        let log_iter = stmt.query_map(sql_args.as_slice(), |row| {
            let fields_str: String = row.get(5)?;
            let fields = serde_json::from_str(&fields_str).unwrap_or(serde_json::Value::Null);

            let ts_str: String = row.get(1)?;
            let timestamp = chrono::DateTime::parse_from_rfc3339(&ts_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_default();
            
            let ing_str: String = row.get(6)?;
            let ingested_at = chrono::DateTime::parse_from_rfc3339(&ing_str)
                .map(|d| d.with_timezone(&Utc))
                .unwrap_or_default();

            Ok(crate::ingest::normalize::NormalizedLog {
                id: row.get(0)?,
                timestamp,
                source: row.get(2)?,
                level: row.get(3)?,
                message: row.get(4)?,
                fields,
                ingested_at,
            })
        }).map_err(|e| DbError::Query(e.to_string()))?;

        let mut logs = Vec::new();
        for log in log_iter {
            logs.push(log.map_err(|e| DbError::Query(e.to_string()))?);
        }

        Ok(logs)
    }

    pub fn delete_old_logs(&self, retention_days: i64) -> Result<usize, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let q = format!("DELETE FROM logs WHERE ingested_at < now() - INTERVAL {} DAY", retention_days);
        let mut stmt = conn.prepare(&q).map_err(|e| DbError::Query(e.to_string()))?;
        let count = stmt.execute([]).map_err(|e| DbError::Query(e.to_string()))?;
        Ok(count)
    }

    pub fn get_retention_days(&self) -> Result<i64, DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::Query(format!("Mutex envenenado: {}", e)))?;
        let mut stmt = conn.prepare("SELECT value FROM config WHERE key = 'retention.default_days'")
            .map_err(|e| DbError::Query(e.to_string()))?;
        let value: String = stmt.query_row([], |row| row.get(0)).map_err(|e| DbError::Query(e.to_string()))?;
        value.parse::<i64>().map_err(|e| DbError::Query(format!("Invalid retention days format: {}", e)))
    }
}

#[derive(Debug, Clone)]
pub struct LogQueryParams {
    pub source: Option<String>,
    pub level: Option<String>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub search: Option<String>,
    pub limit: u32,
    pub cursor: Option<String>,
}
