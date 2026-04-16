mod config;
mod db;
mod ingest;
mod routes;
use std::sync::Arc;

use tracing::{info, Level};
use tracing_subscriber::EnvFilter;

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

    let shared_db = Arc::new(db);

    info!(
        "evlogagent listening on {}:{}, storage={}",
        cfg.host, cfg.port, cfg.storage_mode
    );
    info!("database ready at {}", cfg.data_path);

    let app = routes::create_router(shared_db);

    let addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap_or_else(|e| {
        panic!("Error al hacer bind en la direccion {}: {}", addr, e);
    });

    axum::serve(listener, app).await.unwrap_or_else(|e| {
        panic!("Error al iniciar el servidor Axum: {}", e);
    });
}
