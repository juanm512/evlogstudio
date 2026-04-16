use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::auth::verify_jwt;
use crate::AppState;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub role: String,
}

#[async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let auth_header = parts.headers.get("Authorization");
        let unauthorized = || {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "unauthorized"})),
            ).into_response()
        };

        if let Some(auth_value) = auth_header {
            if let Ok(auth_str) = auth_value.to_str() {
                if auth_str.starts_with("Bearer ") {
                    let token = &auth_str[7..];
                    match verify_jwt(token, &state.jwt_secret) {
                        Ok(claims) => {
                            return Ok(AuthUser {
                                id: claims.sub,
                                email: claims.email,
                                role: claims.role,
                            });
                        }
                        Err(_) => return Err(unauthorized()),
                    }
                }
            }
        }
        
        Err(unauthorized())
    }
}
