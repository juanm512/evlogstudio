mod config;
mod db;
mod ingest;
mod routes;
mod auth;
pub mod error;
pub use error::AppError;

use std::sync::Arc;
use std::collections::HashMap;

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
    pub source_sampling: Arc<tokio::sync::RwLock<HashMap<String, SamplingConfig>>>,
}

impl axum::extract::FromRef<AppState> for Arc<db::Db> {
    fn from_ref(state: &AppState) -> Arc<db::Db> {
        state.db.clone()
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--version" {
        println!("evlogstudio {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

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

    let source_sampling = Arc::new(tokio::sync::RwLock::new(HashMap::<String, SamplingConfig>::new()));

    let app_state = AppState {
        db: shared_db.clone(),
        jwt_secret,
        sampling_config: sampling_config.clone(),
        source_sampling: source_sampling.clone(),
    };

    info!("evlogstudio listening on {}:{}", cfg.host, cfg.port);
    info!("storage: {}", cfg.storage_mode);

    // Arrancar tareas de fondo
    start_retention_job(shared_db.clone());
    start_sampling_refresh_job(shared_db.clone(), sampling_config.clone(), source_sampling.clone());
    if cfg.storage_mode == "s3" {
        start_s3_sync_job(shared_db.clone());
    }

    let app = routes::create_router(app_state);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap_or_else(|e| {
        panic!("Error al hacer bind en la direccion {}: {}", addr, e);
    });

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap_or_else(|e| {
            panic!("Error al iniciar el servidor Axum: {}", e);
        });

    // Al llegar aquí el servidor paró de aceptar conexiones.
    // Soltar el Arc<Db> para que DuckDB haga checkpoint del WAL antes de salir.
    drop(shared_db);
    info!("Base de datos cerrada correctamente.");
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Error al instalar handler de Ctrl+C");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Error al instalar handler de SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c    => { info!("Ctrl+C recibido — iniciando shutdown graceful…"); },
        _ = terminate => { info!("SIGTERM recibido — iniciando shutdown graceful…"); },
    }
}

/// Arranca el job de limpieza de logs antiguos segun retencion
fn start_retention_job(db: Arc<db::Db>) {
    const INTERVAL_SECS: u64 = 3600;
    tracing::info!(
        "Retention job: scheduled (interval={}s, mode=global — one job handles all sources)",
        INTERVAL_SECS
    );
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(INTERVAL_SECS)).await;
            tracing::info!("Retention job: starting run");
            match db.delete_old_logs() {
                Ok(deleted) => {
                    let total: usize = deleted.values().sum();
                    if total == 0 {
                        tracing::info!("Retention job: finished — nothing to delete");
                    } else {
                        tracing::info!("Retention job: finished — deleted {} log(s) total", total);
                        let mut breakdown: Vec<(&String, &usize)> = deleted.iter()
                            .filter(|(_, c)| **c > 0)
                            .collect();
                        breakdown.sort_by_key(|(s, _)| s.as_str());
                        for (source, count) in breakdown {
                            let label = if source == "_default" { "(default retention)" } else { source.as_str() };
                            tracing::info!("  Retention job: source={} deleted={}", label, count);
                        }
                    }
                }
                Err(e) => tracing::error!("Retention job: run failed — {}", e),
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

/// Actualiza el cache de sampling global y por source cada 60s
fn start_sampling_refresh_job(
    db: Arc<db::Db>,
    cache: Arc<tokio::sync::RwLock<SamplingConfig>>,
    source_cache: Arc<tokio::sync::RwLock<HashMap<String, SamplingConfig>>>,
) {
    tokio::spawn(async move {
        loop {
            // Cache global
            match db.get_all_config() {
                Ok(map) => {
                    let mut lock = cache.write().await;
                    lock.enabled = map.get("sampling.enabled").map(|v| v == "true").unwrap_or(false);
                    lock.debug_rate = map.get("sampling.debug_rate").and_then(|v| v.parse().ok()).unwrap_or(10);
                    lock.info_rate = map.get("sampling.info_rate").and_then(|v| v.parse().ok()).unwrap_or(100);
                    lock.warn_rate = map.get("sampling.warn_rate").and_then(|v| v.parse().ok()).unwrap_or(100);
                }
                Err(e) => tracing::error!("Sampling refresh job: error al leer config global: {}", e),
            }

            // Cache por source — solo sources con sampling_enabled = true entran al map
            match db.list_sources() {
                Ok(sources) => {
                    let mut map = source_cache.write().await;
                    map.clear();
                    for s in sources {
                        if s.sampling_enabled == Some(true) {
                            map.insert(s.name, SamplingConfig {
                                enabled: true,
                                debug_rate: s.sampling_debug_rate.unwrap_or(10) as u8,
                                info_rate:  s.sampling_info_rate.unwrap_or(100) as u8,
                                warn_rate:  s.sampling_warn_rate.unwrap_or(100) as u8,
                            });
                        }
                    }
                }
                Err(e) => tracing::error!("Sampling refresh job: error al leer sources: {}", e),
            }

            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    });
}
