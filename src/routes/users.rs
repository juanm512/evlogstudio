use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, put, delete},
    Json, Router,
};
use serde::Deserialize;
use std::sync::Arc;

use crate::AppState;
use crate::db::Db;
use crate::auth::middleware::AuthUser;
use crate::auth::require_admin;
use crate::AppError;

#[derive(Deserialize)]
pub struct CreateUserReq {
    email: String,
    password: String,
    role: String,
}

#[derive(Deserialize)]
pub struct UpdateRoleReq {
    role: String,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/users", get(list_users).post(create_user))
        .route("/api/users/:id/role", put(update_role))
        .route("/api/users/:id", delete(delete_user))
}

async fn list_users(
    user: AuthUser,
    State(db): State<Arc<Db>>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    
    let users = db.list_users().map_err(|e| AppError::Internal(e.to_string()))?;
    Ok((StatusCode::OK, Json(users)))
}

async fn create_user(
    user: AuthUser,
    State(db): State<Arc<Db>>,
    Json(payload): Json<CreateUserReq>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    
    let email = payload.email.trim();
    if !email.contains('@') {
        return Err(AppError::BadRequest("invalid email pattern".to_string()));
    }
    if payload.password.len() < 8 {
        return Err(AppError::BadRequest("password must be at least 8 characters".to_string()));
    }
    if payload.role != "admin" && payload.role != "viewer" {
        return Err(AppError::BadRequest("role must be admin or viewer".to_string()));
    }

    if db.find_user_by_email(email).map_err(|e| AppError::Internal(e.to_string()))?.is_some() {
        return Err(AppError::Conflict("email already exists".to_string()));
    }

    let id = db.create_user(email, &payload.password, &payload.role)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Devolver el UserPublicRecord
    let created_user = db.find_user_by_id(&id)
        .map_err(|e| AppError::Internal(e.to_string()))?
        .ok_or_else(|| AppError::Internal("user created but not found".to_string()))?;

    let public_record = crate::db::UserPublicRecord {
        id: created_user.id,
        email: created_user.email,
        role: created_user.role,
        created_at: None, // No lo devolvemos con precisión aqui o deberiamos buscarlo
        last_login: None,
    };

    Ok((StatusCode::CREATED, Json(public_record)))
}

async fn update_role(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
    Json(payload): Json<UpdateRoleReq>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    
    if user.id == id {
        return Err(AppError::Forbidden); // No puede cambiar su propio rol (según spec)
    }

    if payload.role != "admin" && payload.role != "viewer" {
        return Err(AppError::BadRequest("role must be admin or viewer".to_string()));
    }

    let updated = db.update_user_role(&id, &payload.role)
        .map_err(|e| AppError::Internal(e.to_string()))?;
        
    if !updated {
        return Err(AppError::NotFound);
    }

    Ok((StatusCode::OK, Json(serde_json::json!({"message": "updated"}))))
}

async fn delete_user(
    user: AuthUser,
    Path(id): Path<String>,
    State(db): State<Arc<Db>>,
) -> Result<impl IntoResponse, AppError> {
    require_admin(&user)?;
    
    if user.id == id {
        return Err(AppError::Forbidden); // No puede eliminarse a sí mismo
    }

    let deleted = db.delete_user(&id).map_err(|e| AppError::Internal(e.to_string()))?;
    if !deleted {
        return Err(AppError::NotFound);
    }

    Ok((StatusCode::OK, Json(serde_json::json!({"message": "deleted"}))))
}
