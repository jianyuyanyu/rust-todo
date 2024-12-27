use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use bcrypt::{hash, verify, DEFAULT_COST};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

use crate::models::Claims;
use crate::AppError;
use std::collections::HashSet;
use std::env;

lazy_static::lazy_static! {
    static ref JWT_SECRET: Vec<u8> = env::var("JWT_SECRET")
        .unwrap_or_else(|_| "ThisISMYSectKeyXHaxx1234".to_string())
        .into_bytes();
}

pub fn hash_password(password: &str) -> Result<String, bcrypt::BcryptError> {
    hash(password.as_bytes(), DEFAULT_COST)
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    verify(password.as_bytes(), hash).unwrap_or(false)
}

pub fn create_token(user_id: i64) -> Result<String, jsonwebtoken::errors::Error> {
    let claims = Claims { sub: user_id };
    let header = Header::default();

    encode(&header, &claims, &EncodingKey::from_secret(&JWT_SECRET))
}

pub struct AuthUser {
    pub user_id: i64,
}

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or_else(|| {
                AppError(
                    StatusCode::UNAUTHORIZED,
                    "Missing authorization header".to_string(),
                )
            })?;

        // Create validation that doesn't check for expiration
        let mut validation = Validation::new(jsonwebtoken::Algorithm::HS256);
        validation.required_spec_claims = HashSet::new();
        validation.validate_exp = false; // Disable expiration time validation
        validation.validate_aud = false;

        // Decode and validate the token
        let token_data = decode::<Claims>(
            auth_header,
            &DecodingKey::from_secret(&JWT_SECRET),
            &validation,
        )
        .map_err(|_| AppError(StatusCode::UNAUTHORIZED, "Invalid token".to_string()))?;

        Ok(AuthUser {
            user_id: token_data.claims.sub,
        })
    }
}
