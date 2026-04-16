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

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<db::Db>,
    pub jwt_secret: String,
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

    let app_state = AppState {
        db: shared_db.clone(),
        jwt_secret,
    };

    // Imprimir banner si corresponde (primer arranque)
    ensure_initial_setup(&shared_db);

    info!("evlogagent listening on {}:{}", cfg.host, cfg.port);
    info!("storage: {}", cfg.storage_mode);
    if cfg.storage_mode == "local" {
        info!("database ready at {}", cfg.data_path);
    }

    // Arrancar tareas de fondo
    start_retention_job(shared_db.clone());
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

/// Verifica si es necesario el setup inicial y muestra el banner
fn ensure_initial_setup(db: &Arc<db::Db>) {
    let user_count = db.count_users().unwrap_or_else(|e| {
        panic!("Error al contar usuarios: {}", e);
    });

    if user_count == 0 {
        let setup_token = uuid::Uuid::new_v4().to_string();
        db.set_config_value("setup.token", &setup_token).unwrap_or_else(|e| {
            panic!("Error al guardar setup token: {}", e);
        });
        println!("    ╔══════════════════════════════════════════════════╗");
        println!("    ║  Initial setup required                          ║");
        println!("    ║                                                  ║");
        println!("    ║  Complete setup at:                              ║");
        println!("    ║  POST /setup?token={:<30}║", setup_token);
        println!("    ║                                                  ║");
        println!("    ║  This token expires in 24 hours.                 ║");
        println!("    ╚══════════════════════════════════════════════════╝");
    }
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
