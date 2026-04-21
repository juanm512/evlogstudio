pub mod middleware;

use crate::db::UserRecord;
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey, errors::ErrorKind};
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
#[allow(dead_code)]
pub enum AuthError {
    #[error("Invalid Token")]
    InvalidToken,
    #[error("Token Expired")]
    Expired,
    #[error("Auth Error: {0}")]
    Other(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

pub fn create_jwt(user: &UserRecord, secret: &str) -> Result<String, AuthError> {
    let now = chrono::Utc::now();
    let iat = now.timestamp() as usize;
    let exp = (now + chrono::Duration::days(7)).timestamp() as usize;

    let claims = Claims {
        sub: user.id.clone(),
        email: user.email.clone(),
        role: user.role.clone(),
        exp,
        iat,
    };

    encode(
        &Header::new(jsonwebtoken::Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes())
    ).map_err(|e| AuthError::Other(e.to_string()))
}

#[allow(dead_code)]
pub fn verify_jwt(token: &str, secret: &str) -> Result<Claims, AuthError> {
    let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.validate_exp = true;
    validation.required_spec_claims.insert("exp".to_string());
    validation.required_spec_claims.insert("sub".to_string());

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation
    ).map_err(|e| {
        match e.kind() {
            ErrorKind::ExpiredSignature => AuthError::Expired,
            ErrorKind::InvalidToken | ErrorKind::InvalidSignature | ErrorKind::InvalidAlgorithm | ErrorKind::MissingRequiredClaim(_) => AuthError::InvalidToken,
            _ => AuthError::Other(e.to_string())
        }
    })?;

    Ok(token_data.claims)
}

pub fn require_admin(user: &middleware::AuthUser) -> Result<(), crate::AppError> {
    if user.role != "admin" {
        Err(crate::AppError::Forbidden)
    } else {
        Ok(())
    }
}
