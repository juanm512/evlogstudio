pub mod schema;

use crate::config::Config;
use duckdb::Connection;
use std::sync::Mutex;
use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRecord {
    pub id: String,
    pub email: String,
    pub password_hash: String,
    pub role: String,
}

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
        let threshold = chrono::Utc::now() - chrono::Duration::days(retention_days);
        let q = format!("DELETE FROM logs WHERE ingested_at < '{}'", threshold.to_rfc3339());
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
        let mut query = "SELECT source, field_path, field_type, seen_count, CAST(last_seen AS VARCHAR) FROM _schema".to_string();

        if let Some(s) = source {
            query.push_str(" WHERE source = ?");
            args.push(Box::new(s.to_string()));
        }

        query.push_str(" ORDER BY source, field_path");

        let mut stmt = conn.prepare(&query).map_err(|e| DbError::Query(e.to_string()))?;
        let sql_args: Vec<&dyn duckdb::ToSql> = args.iter().map(|b| b.as_ref()).collect();

        let row_iter = stmt.query_map(sql_args.as_slice(), |row| {
            let last_seen_str: String = row.get(4)?;
            let last_seen = chrono::DateTime::parse_from_rfc3339(&last_seen_str)
                .map(|d| d.with_timezone(&Utc))
                .ok();

            Ok(crate::ingest::schema::SchemaEntry {
                source: row.get(0)?,
                field_path: row.get(1)?,
                field_type: row.get(2)?,
                seen_count: row.get(3)?,
                last_seen,
            })
        }).map_err(|e| DbError::Query(e.to_string()))?;

        let mut entries = Vec::new();
        for r in row_iter {
            entries.push(r.map_err(|e| DbError::Query(e.to_string()))?);
        }

        Ok(entries)
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
