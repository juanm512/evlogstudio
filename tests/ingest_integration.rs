mod common;

use axum::http::StatusCode;
use serde_json::json;
use crate::common::setup_test_app;

#[tokio::test]
async fn test_ingest_token_validation() {
    let server = setup_test_app().await;

    // 1. Setup admin and login to create a source
    server.post("/api/setup")
        .json(&json!({
            "email": "admin@evlog.dev",
            "password": "password123"
        }))
        .await;

    let res = server.post("/auth/login")
        .json(&json!({
            "email": "admin@evlog.dev",
            "password": "password123"
        }))
        .await;
    let admin_token = res.json::<serde_json::Value>()["token"].as_str().expect("Ingest token missing").to_string();

    // 2. Create a source and get ingest token
    let res = server.post("/api/sources")
        .add_header("Authorization", format!("Bearer {}", admin_token))
        .json(&json!({
            "name": "backend",
            "retention_days": 7
        }))
        .await;
    res.assert_status(StatusCode::CREATED);
    let source_id = res.json::<serde_json::Value>()["id"].as_str().expect("Source ID missing").to_string();

    // 2.1 Create an ingest token for the source
    let res = server.post(&format!("/api/sources/{}/tokens", source_id))
        .add_header("Authorization", format!("Bearer {}", admin_token))
        .json(&json!({ "name": "default-token" }))
        .await;
    res.assert_status(StatusCode::CREATED);
    let token_data = res.json::<serde_json::Value>();
    let ingest_token = token_data["token"].as_str().expect("Ingest token missing");

    // 3. Ingest with valid token
    let res = server.post("/ingest")
        .add_header("Authorization", format!("Bearer {}", ingest_token))
        .json(&json!({
            "message": "hello world",
            "level": "info"
        }))
        .await;
    res.assert_status(StatusCode::OK);
    res.assert_json(&json!({"inserted": 1}));

    // 4. Ingest with invalid token
    let res = server.post("/ingest")
        .add_header("Authorization", "Bearer invalid_token")
        .json(&json!({
            "message": "hello world"
        }))
        .await;
    res.assert_status(StatusCode::UNAUTHORIZED);

    // 5. Ingest without token
    let res = server.post("/ingest")
        .json(&json!({
            "message": "hello world"
        }))
        .await;
    res.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_ingest_source_isolation() {
    let server = setup_test_app().await;

    // Setup admin
    server.post("/api/setup")
        .json(&json!({
            "email": "admin@evlog.dev",
            "password": "password123"
        }))
        .await;

    let res = server.post("/auth/login")
        .json(&json!({
            "email": "admin@evlog.dev",
            "password": "password123"
        }))
        .await;
    let admin_token = res.json::<serde_json::Value>()["token"].as_str().expect("Ingest token missing").to_string();

    // Create source A
    let res = server.post("/api/sources")
        .add_header("Authorization", format!("Bearer {}", admin_token))
        .json(&json!({ "name": "source-a" }))
        .await;
    let source_a_id = res.json::<serde_json::Value>()["id"].as_str().expect("Source A ID missing").to_string();

    let res = server.post(&format!("/api/sources/{}/tokens", source_a_id))
        .add_header("Authorization", format!("Bearer {}", admin_token))
        .json(&json!({ "name": "token-a" }))
        .await;
    let token_a = res.json::<serde_json::Value>()["token"].as_str().expect("Token A missing").to_string();

    // Create source B
    let res = server.post("/api/sources")
        .add_header("Authorization", format!("Bearer {}", admin_token))
        .json(&json!({ "name": "source-b" }))
        .await;
    let source_b_id = res.json::<serde_json::Value>()["id"].as_str().expect("Source B ID missing").to_string();

    let res = server.post(&format!("/api/sources/{}/tokens", source_b_id))
        .add_header("Authorization", format!("Bearer {}", admin_token))
        .json(&json!({ "name": "token-b" }))
        .await;
    let token_b = res.json::<serde_json::Value>()["token"].as_str().expect("Token B missing").to_string();

    // Ingest for source A with token A
    let res = server.post("/ingest")
        .add_header("Authorization", format!("Bearer {}", token_a))
        .json(&json!({ "msg": "from a" }))
        .await;
    res.assert_status(StatusCode::OK);

    // Ingest for source B with token B
    let res = server.post("/ingest")
        .add_header("Authorization", format!("Bearer {}", token_b))
        .json(&json!({ "msg": "from b" }))
        .await;
    res.assert_status(StatusCode::OK);

    // Verify logs for source A still has 1
    let res = server.get("/api/logs?source=source-a")
        .add_header("Authorization", format!("Bearer {}", admin_token))
        .await;
    let logs_a = res.json::<serde_json::Value>()["logs"].as_array().expect("Logs array missing").len();
    assert_eq!(logs_a, 1);

    // Verify logs for source B now has 1
    let res = server.get("/api/logs?source=source-b")
        .add_header("Authorization", format!("Bearer {}", admin_token))
        .await;
    let logs_b = res.json::<serde_json::Value>()["logs"].as_array().expect("Logs array missing").len();
    assert_eq!(logs_b, 1);
}
