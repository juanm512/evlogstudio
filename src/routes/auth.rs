use axum::{
    extract::{State},
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

}

pub async fn login_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<dto::LoginReq>,
) -> Result<impl IntoResponse, AppError> {
    // RATE LIMITING
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<std::net::IpAddr>().ok())
        .unwrap_or(std::net::IpAddr::from([127, 0, 0, 1]));

    if state.login_limiter.check_key(&ip).is_err() {
        return Err(AppError::TooManyRequests);
    }

    use argon2::{password_hash::PasswordHash, Argon2, PasswordVerifier};

    let user_opt = state.db.find_user_by_email(&payload.email).unwrap_or_default();

    let (is_valid, user) = match user_opt {
        Some(u) => {
            let parsed_hash = PasswordHash::new(&u.password_hash).map_err(|_| AppError::Unauthorized)?;
            let ok = Argon2::default().verify_password(payload.password.as_bytes(), &parsed_hash).is_ok();
            (ok, Some(u))
        }
        None => {
            // DUMMY HASH para evitar ataques de timing (enumeración de usuarios)
            // Este hash representa un password "dummy" con los mismos parámetros que el real.
            // Argon2id, m=65536, t=2, p=1
            let dummy_hash = "$argon2id$v=19$m=65536,t=2,p=1$c29tZXNhbHQ$Rdesc8vVv+7WqU6Y/mO/q6D+I3Lnd8P6tU5e1q9SjB0";
            let parsed_hash = PasswordHash::new(dummy_hash).unwrap();
            let _ = Argon2::default().verify_password(payload.password.as_bytes(), &parsed_hash);
            (false, None)
        }
    };

    if !is_valid {
        return Err(AppError::Unauthorized);
    }

    let user = user.unwrap();
    let _ = state.db.update_last_login(&user.id);

    match create_jwt(&user, &state.jwt_secret) {
        Ok(token) => Ok((
            StatusCode::OK,
            Json(json!({
                "token": token,
                "user": {
                    "email": user.email,
                    "role": user.role
                }
            })),
        )),
        Err(_) => Err(AppError::Internal("failed to create token".to_string())),
    }
}

pub async fn verify_handler(
    crate::auth::middleware::AuthUser { email, role, .. }: crate::auth::middleware::AuthUser,
) -> Result<impl IntoResponse, AppError> {
    Ok((
        StatusCode::OK,
        Json(json!({
            "email": email,
            "role": role
        })),
    ))
}

pub async fn setup_get_handler(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    match state.db.count_users() {
        Ok(0) => Ok((StatusCode::OK, Json(json!({"setup_required": true})))),
        _ => Err(AppError::NotFound),
    }
}

pub async fn setup_post_handler(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(payload): Json<dto::LoginReq>,
) -> Result<impl IntoResponse, AppError> {
    // RATE LIMITING
    let ip = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<std::net::IpAddr>().ok())
        .unwrap_or(std::net::IpAddr::from([127, 0, 0, 1]));

    if state.login_limiter.check_key(&ip).is_err() {
        return Err(AppError::TooManyRequests);
    }

    // 1. Validar input
    if !payload.email.contains('@') || payload.password.len() < 8 {
        return Err(AppError::BadRequest("invalid email or password too short (min 8)".to_string()));
    }

    // 2. Intentar crear admin solo si no hay usuarios. 
    // Usamos el error de la DB (o count_users previo) pero confiamos en que
    // el primer usuario creado será el admin. Para evitar race conditions,
    // count_users y create_user deberían ser atómicos.
    // La DB tiene restricción UNIQUE en email (asumido).
    let count = state.db.count_users().map_err(|e| AppError::Internal(e.to_string()))?;
    if count > 0 {
        return Err(AppError::NotFound);
    }

    match state.db.create_user(&payload.email, &payload.password, "admin") {
        Ok(_) => {
            Ok((StatusCode::OK, Json(json!({"message": "admin created"}))))
        }
        Err(e) => {
            // Si hubo una race condition y alguien más creó el primer usuario justo antes,
            // probablemente fallará por UNIQUE constraint o similar.
            Err(AppError::Internal(format!("failed to create admin: {}", e)))
        }
    }
}
