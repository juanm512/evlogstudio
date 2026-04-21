mod common;

use axum::http::StatusCode;
use serde_json::json;
use crate::common::setup_test_app;

#[tokio::test]
async fn test_setup_and_login_flow() {
    let server = setup_test_app().await;

    // 1. Initial health check should require setup
    let res = server.get("/health").await;
    res.assert_status(StatusCode::OK);
    res.assert_json(&json!({
        "status": "ok",
        "setup_required": true
    }));

    // 2. Setup admin
    let res = server.post("/api/setup")
        .json(&json!({
            "email": "admin@evlog.dev",
            "password": "password123"
        }))
        .await;
    res.assert_status(StatusCode::OK);

    // 3. Setup again should fail 404 (as per spec)
    let res = server.post("/api/setup")
        .json(&json!({
            "email": "admin2@evlog.dev",
            "password": "password123"
        }))
        .await;
    res.assert_status(StatusCode::NOT_FOUND);

    // 4. Login with correct credentials
    let res = server.post("/auth/login")
        .json(&json!({
            "email": "admin@evlog.dev",
            "password": "password123"
        }))
        .await;
    res.assert_status(StatusCode::OK);
    let login_data = res.json::<serde_json::Value>();
    let token = login_data["token"].as_str().expect("Token missing");

    // 5. Login with incorrect password -> 401
    let res = server.post("/auth/login")
        .json(&json!({
            "email": "admin@evlog.dev",
            "password": "wrongpassword"
        }))
        .await;
    res.assert_status(StatusCode::UNAUTHORIZED);

    // 6. Login with non-existent email -> 401
    let res = server.post("/auth/login")
        .json(&json!({
            "email": "nonexistent@evlog.dev",
            "password": "password123"
        }))
        .await;
    res.assert_status(StatusCode::UNAUTHORIZED);

    // 7. Access admin route with token
    let res = server.get("/api/users")
        .add_header("Authorization", format!("Bearer {}", token))
        .await;
    res.assert_status(StatusCode::OK);

    // 8. Access admin route without token -> 401
    let res = server.get("/api/users").await;
    res.assert_status(StatusCode::UNAUTHORIZED);

    // 9. Access admin route with manipulated token -> 401
    let res = server.get("/api/users")
        .add_header("Authorization", format!("Bearer {}manipulated", token))
        .await;
    res.assert_status(StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn test_rbac_admin_vs_viewer() {
    let server = setup_test_app().await;

    // Setup admin
    server.post("/api/setup")
        .json(&json!({
            "email": "admin@evlog.dev",
            "password": "password123"
        }))
        .await;

    // Login as admin
    let res = server.post("/auth/login")
        .json(&json!({
            "email": "admin@evlog.dev",
            "password": "password123"
        }))
        .await;
    let admin_token = res.json::<serde_json::Value>()["token"].as_str().unwrap().to_string();

    // Create a viewer user
    let res = server.post("/api/users")
        .add_header("Authorization", format!("Bearer {}", admin_token))
        .json(&json!({
            "email": "viewer@evlog.dev",
            "password": "password123",
            "role": "viewer"
        }))
        .await;
    res.assert_status(StatusCode::CREATED);

    // Login as viewer
    let res = server.post("/auth/login")
        .json(&json!({
            "email": "viewer@evlog.dev",
            "password": "password123"
        }))
        .await;
    let viewer_token = res.json::<serde_json::Value>()["token"].as_str().unwrap().to_string();

    // Viewer tries to list users (admin only) -> 403
    let res = server.get("/api/users")
        .add_header("Authorization", format!("Bearer {}", viewer_token))
        .await;
    res.assert_status(StatusCode::FORBIDDEN);

    // Admin can list users
    let res = server.get("/api/users")
        .add_header("Authorization", format!("Bearer {}", admin_token))
        .await;
    res.assert_status(StatusCode::OK);
}

#[tokio::test]
async fn test_expired_jwt() {
    let server = setup_test_app().await;

    let secret = "test_secret_32_chars_long_exactly_";
    
    use jsonwebtoken::{encode, Header, EncodingKey};
    use serde::Serialize;

    #[derive(Debug, Serialize)]
    struct Claims {
        sub: String,
        email: String,
        role: String,
        exp: usize,
        iat: usize,
    }

    let now = chrono::Utc::now().timestamp() as usize;
    let expired_claims = Claims {
        sub: "user_123".to_string(),
        email: "admin@evlog.dev".to_string(),
        role: "admin".to_string(),
        exp: now - 3600, 
        iat: now - 7200,
    };

    let token = encode(
        &Header::default(),
        &expired_claims,
        &EncodingKey::from_secret(secret.as_bytes())
    ).unwrap();

    let res = server.get("/api/users")
        .add_header("Authorization", format!("Bearer {}", token))
        .await;
    
    res.assert_status(StatusCode::UNAUTHORIZED);
}
