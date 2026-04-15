mod config;
mod routes;

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

    info!(
        "evlogagent listening on {}:{}, storage={}",
        cfg.host, cfg.port, cfg.storage_mode
    );

    let app = routes::create_router();

    let addr = format!("{}:{}", cfg.host, cfg.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap_or_else(|e| {
        panic!("Error al hacer bind en la direccion {}: {}", addr, e);
    });

    axum::serve(listener, app).await.unwrap_or_else(|e| {
        panic!("Error al iniciar el servidor Axum: {}", e);
    });
}
