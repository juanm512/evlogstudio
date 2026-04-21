use std::sync::Arc;
use std::collections::HashMap;
use std::num::NonZeroU32;

use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

use evlogstudio::{AppState, db, routes, SamplingConfig, config};
use governor::{Quota, RateLimiter};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && args[1] == "--version" {
        println!("evlogstudio {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(Level::INFO.into())
                .from_env_lossy(),
        )
        .init();

    let cfg = config::Config::from_env().unwrap_or_else(|e| {
        panic!("Error al cargar configuracion desde variables de entorno: {}", e);
    });

    let db = db::Db::open(&cfg).unwrap_or_else(|e| {
        panic!("Error al inicializar la base de datos: {}", e);
    });

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

    let source_sampling = Arc::new(tokio::sync::RwLock::new(HashMap::new()));

    let login_limiter = Arc::new(RateLimiter::dashmap(
        Quota::with_period(std::time::Duration::from_secs(60))
            .unwrap()
            .allow_burst(NonZeroU32::new(10).unwrap()),
    ));

    let app_state = AppState {
        db: shared_db.clone(),
        jwt_secret,
        sampling_config: sampling_config.clone(),
        source_sampling: source_sampling.clone(),
        login_limiter,
    };

    info!("evlogstudio listening on {}:{}", cfg.host, cfg.port);
    info!("storage: {}", cfg.storage_mode);

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

fn start_retention_job(db: Arc<db::Db>) {
    const INTERVAL_SECS: u64 = 3600;
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(INTERVAL_SECS)).await;
            match db.delete_old_logs() {
                Ok(deleted) => {
                    let total: usize = deleted.values().sum();
                    if total > 0 {
                        tracing::info!("Retention job: deleted {} log(s)", total);
                    }
                }
                Err(e) => tracing::error!("Retention job: failed — {}", e),
            }
        }
    });
}

fn start_s3_sync_job(db: Arc<db::Db>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            let _ = db.sync_to_s3().await;
        }
    });
}

fn start_sampling_refresh_job(
    db: Arc<db::Db>,
    cache: Arc<tokio::sync::RwLock<SamplingConfig>>,
    source_cache: Arc<tokio::sync::RwLock<HashMap<String, SamplingConfig>>>,
) {
    tokio::spawn(async move {
        loop {
            if let Ok(map) = db.get_all_config() {
                let mut lock = cache.write().await;
                lock.enabled = map.get("sampling.enabled").map(|v| v == "true").unwrap_or(false);
                lock.debug_rate = map.get("sampling.debug_rate").and_then(|v| v.parse().ok()).unwrap_or(10);
                lock.info_rate = map.get("sampling.info_rate").and_then(|v| v.parse().ok()).unwrap_or(100);
                lock.warn_rate = map.get("sampling.warn_rate").and_then(|v| v.parse().ok()).unwrap_or(100);
            }

            if let Ok(sources) = db.list_sources() {
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
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        }
    });
}
