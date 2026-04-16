use axum::{
    extract::{State, Query},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use serde_json::json;

use crate::AppState;
use crate::auth::create_jwt;

pub mod dto {
    use super::*;
    
    #[derive(Deserialize)]
    pub struct LoginReq {
        pub email: String,
        pub password: String,
    }

    #[derive(Deserialize)]
    pub struct SetupQuery {
        pub token: String,
    }
}

pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<dto::LoginReq>,
) -> impl IntoResponse {
    let user = match state.db.find_user_by_email(&payload.email) {
        Ok(Some(u)) => u,
        _ => return (StatusCode::UNAUTHORIZED, Json(json!({"error": "invalid credentials"}))).into_response(),
    };

    use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
    
    let is_valid = match PasswordHash::new(&user.password_hash) {
        Ok(parsed_hash) => {
            Argon2::default().verify_password(payload.password.as_bytes(), &parsed_hash).is_ok()
        }
        Err(_) => false,
    };

    if !is_valid {
        return (StatusCode::UNAUTHORIZED, Json(json!({"error": "invalid credentials"}))).into_response();
    }

    let _ = state.db.update_last_login(&user.id);

    match create_jwt(&user, &state.jwt_secret) {
        Ok(token) => (
            StatusCode::OK,
            Json(json!({"token": token, "role": user.role})),
        ).into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": "failed to create token"})),
        ).into_response(),
    }
}

pub async fn setup_get_handler(
    State(state): State<AppState>,
    Query(query): Query<dto::SetupQuery>,
) -> impl IntoResponse {
    match state.db.count_users() {
        Ok(0) => {}
        _ => return (StatusCode::NOT_FOUND, Json(json!({"error": "setup already completed"}))).into_response(),
    }

    match state.db.get_config_value("setup.token") {
        Ok(Some(token)) if token == query.token => {
            (StatusCode::OK, Json(json!({"message": "ready"}))).into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"error": "setup token not found"}))).into_response(),
        _ => (StatusCode::UNAUTHORIZED, Json(json!({"error": "invalid or expired token"}))).into_response(),
    }
}

pub async fn setup_post_handler(
    State(state): State<AppState>,
    Query(query): Query<dto::SetupQuery>,
    Json(payload): Json<dto::LoginReq>,
) -> impl IntoResponse {
    match state.db.count_users() {
        Ok(0) => {}
        _ => return (StatusCode::NOT_FOUND, Json(json!({"error": "setup already completed"}))).into_response(),
    }

    match state.db.get_config_value("setup.token") {
        Ok(Some(token)) if token == query.token => {}
        _ => return (StatusCode::UNAUTHORIZED, Json(json!({"error": "invalid or expired token"}))).into_response(),
    }

    if !payload.email.contains('@') || payload.password.len() < 8 {
        return (StatusCode::BAD_REQUEST, Json(json!({"error": "invalid email or password too short (min 8)"}))).into_response();
    }

    match state.db.create_user(&payload.email, &payload.password, "admin") {
        Ok(_) => {
            let _ = state.db.set_config_value("setup.token", "");
            (StatusCode::OK, Json(json!({"message": "admin created"}))).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response(),
    }
}
