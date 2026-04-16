pub mod schema;

use crate::config::Config;
use duckdb::Connection;
use std::sync::Mutex;

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
}
