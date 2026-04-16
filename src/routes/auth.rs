use axum::{
    extract::{State, Query},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use serde::Deserialize;
use serde_json::json;

use crate::AppState;
use crate::auth::create_jwt;
use crate::AppError;

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
) -> Result<impl IntoResponse, AppError> {
    let user = match state.db.find_user_by_email(&payload.email) {
        Ok(Some(u)) => u,
        _ => return Err(AppError::Unauthorized),
    };

    use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};
    
    let is_valid = match PasswordHash::new(&user.password_hash) {
        Ok(parsed_hash) => {
            Argon2::default().verify_password(payload.password.as_bytes(), &parsed_hash).is_ok()
        }
        Err(_) => false,
    };

    if !is_valid {
        return Err(AppError::Unauthorized);
    }

    let _ = state.db.update_last_login(&user.id);

    match create_jwt(&user, &state.jwt_secret) {
        Ok(token) => Ok((
            StatusCode::OK,
            Json(json!({"token": token, "role": user.role})),
        )),
        Err(_) => Err(AppError::Internal("failed to create token".to_string())),
    }
}

pub async fn setup_get_handler(
    State(state): State<AppState>,
    Query(query): Query<dto::SetupQuery>,
) -> Result<impl IntoResponse, AppError> {
    match state.db.count_users() {
        Ok(0) => {}
        _ => return Err(AppError::NotFound),
    }

    match state.db.get_config_value("setup.token") {
        Ok(Some(token)) if token == query.token => {
            Ok((StatusCode::OK, Json(json!({"message": "ready"}))))
        }
        Ok(None) => Err(AppError::NotFound),
        _ => Err(AppError::Unauthorized),
    }
}

pub async fn setup_post_handler(
    State(state): State<AppState>,
    Query(query): Query<dto::SetupQuery>,
    Json(payload): Json<dto::LoginReq>,
) -> Result<impl IntoResponse, AppError> {
    match state.db.count_users() {
        Ok(0) => {}
        _ => return Err(AppError::NotFound),
    }

    match state.db.get_config_value("setup.token") {
        Ok(Some(token)) if token == query.token => {}
        _ => return Err(AppError::Unauthorized),
    }

    if !payload.email.contains('@') || payload.password.len() < 8 {
        return Err(AppError::BadRequest("invalid email or password too short (min 8)".to_string()));
    }

    match state.db.create_user(&payload.email, &payload.password, "admin") {
        Ok(_) => {
            let _ = state.db.set_config_value("setup.token", "");
            Ok((StatusCode::OK, Json(json!({"message": "admin created"}))))
        }
        Err(e) => Err(AppError::Internal(e.to_string())),
    }
}
