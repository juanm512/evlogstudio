use std::sync::Arc;
use axum_test::TestServer;
use evlogstudio::{AppState, db, routes, SamplingConfig};
use governor::{Quota, RateLimiter};
use std::num::NonZeroU32;
use std::collections::HashMap;

pub async fn setup_test_app() -> TestServer {
    let db = db::Db::open(&evlogstudio::config::Config {
        storage_mode: "memory".to_string(),
        data_path: "".to_string(),
        host: "127.0.0.1".to_string(),
        port: 8080,
        ..Default::default()
    }).expect("Failed to open memory DB");

    let shared_db = Arc::new(db);
    let jwt_secret = "test_secret_32_chars_long_exactly_".to_string();

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
            .allow_burst(NonZeroU32::new(100).unwrap()), // Higher for tests
    ));

    let state = AppState {
        db: shared_db,
        jwt_secret,
        sampling_config,
        source_sampling,
        login_limiter,
    };

    let router = routes::create_router(state);
    TestServer::new(router).expect("Failed to create test server")
}
