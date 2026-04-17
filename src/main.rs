mod config;
mod db;
mod ingest;
mod routes;
mod auth;
pub mod error;
pub use error::AppError;

use std::sync::Arc;

use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct SamplingConfig {
    pub enabled: bool,
    pub debug_rate: u8,
    pub info_rate: u8,
    pub warn_rate: u8,
}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<db::Db>,
    pub jwt_secret: String,
    pub sampling_config: Arc<tokio::sync::RwLock<SamplingConfig>>,
}

impl axum::extract::FromRef<AppState> for Arc<db::Db> {
    fn from_ref(state: &AppState) -> Arc<db::Db> {
        state.db.clone()
    }
}

#[tokio::main]
async fn main() {
    // Inicializar tracing-subscriber con env-filter y default en INFO
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(Level::INFO.into())
                .from_env_lossy(),
        )
        .init();

    // Cargar config y fallar claramente si falla
    let cfg = config::Config::from_env().unwrap_or_else(|e| {
        panic!("Error al cargar configuracion desde variables de entorno: {}", e);
    });

    let db = db::Db::open(&cfg).unwrap_or_else(|e| {
        panic!("Error al inicializar la base de datos: {}", e);
    });

    // En modo s3: restaurar desde el snapshot antes de arrancar
    if cfg.storage_mode == "s3" {
        let rt = tokio::runtime::Handle::current();
        rt.block_on(async {
            db.load_from_s3().await.unwrap_or_else(|e| {
                panic!("Error al cargar datos desde S3: {}", e);
            });
        });
    }

    let shared_db = Arc::new(db);

    let jwt_secret = shared_db.get_or_create_jwt_secret().unwrap_or_else(|e| {
        panic!("Error al inicializar jwt secret: {}", e);
    });

    let sampling_config = Arc::new(tokio::sync::RwLock::new(SamplingConfig {
        enabled: false,
        debug_rate: 10,
        info_rate: 100,
        warn_rate: 100,
    }));

    let app_state = AppState {
        db: shared_db.clone(),
        jwt_secret,
        sampling_config: sampling_config.clone(),
    };

    info!("evlogstudio listening on {}:{}", cfg.host, cfg.port);
    info!("storage: {}", cfg.storage_mode);

    // Arrancar tareas de fondo
    start_retention_job(shared_db.clone());
    start_sampling_refresh_job(shared_db.clone(), sampling_config.clone());
    if cfg.storage_mode == "s3" {
        start_s3_sync_job(shared_db.clone());
    }

    let app = routes::create_router(app_state);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap_or_else(|e| {
        panic!("Error al hacer bind en la direccion {}: {}", addr, e);
    });

    axum::serve(listener, app).await.unwrap_or_else(|e| {
        panic!("Error al iniciar el servidor Axum: {}", e);
    });
}

/// Arranca el job de limpieza de logs antiguos segun retencion
fn start_retention_job(db: Arc<db::Db>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            match db.get_retention_days() {
                Ok(days) => {
                    match db.delete_old_logs(days) {
                        Ok(deleted) => tracing::info!("Retention job: deleted {} old logs", deleted),
                        Err(e) => tracing::error!("Retention job: error deleting logs: {}", e),
                    }
                }
                Err(e) => tracing::error!("Retention job: error reading retention_days: {}", e),
            }
        }
    });
}

/// Arranca el job de sincronización periódica con S3 (cada 60 segundos).
/// Solo debe llamarse cuando storage_mode == "s3".
fn start_s3_sync_job(db: Arc<db::Db>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            match db.sync_to_s3().await {
                Ok(()) => tracing::info!("S3 sync job: ok"),
                Err(e) => tracing::error!("S3 sync job: error: {}", e),
            }
        }
    });
}

/// Actualiza el cache de sampling cada 60s
fn start_sampling_refresh_job(db: Arc<db::Db>, cache: Arc<tokio::sync::RwLock<SamplingConfig>>) {
    tokio::spawn(async move {
        loop {
            match db.get_all_config() {
                Ok(map) => {
                    let mut lock = cache.write().await;
                    lock.enabled = map.get("sampling.enabled").map(|v| v == "true").unwrap_or(false);
                    lock.debug_rate = map.get("sampling.debug_rate").and_then(|v| v.parse().ok()).unwrap_or(10);
                    lock.info_rate = map.get("sampling.info_rate").and_then(|v| v.parse().ok()).unwrap_or(100);
                    lock.warn_rate = map.get("sampling.warn_rate").and_then(|v| v.parse().ok()).unwrap_or(100);
                }
                Err(e) => tracing::error!("Sampling refresh job: error: {}", e),
            }
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    });
}
