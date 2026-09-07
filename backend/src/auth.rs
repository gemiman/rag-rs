use argon2::password_hash::{phc::PasswordHash, PasswordHasher, PasswordVerifier};
use argon2::Argon2;
use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::state::AppState;

/// 密码散列（argon2，自动生成随机盐）
pub fn hash_password(password: &str) -> Result<String, AppError> {
    Argon2::default()
        .hash_password(password.as_bytes())
        .map(|h| h.to_string())
        .map_err(|e| AppError::Internal(format!("密码加密失败: {e}")))
}

/// 校验密码是否与散列匹配
pub fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|parsed| Argon2::default().verify_password(password.as_bytes(), &parsed).is_ok())
        .unwrap_or(false)
}

/// JWT 载荷
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    /// 用户 id
    pub sub: i64,
    pub username: String,
    pub is_admin: bool,
    /// 过期时间（Unix 秒）
    pub exp: usize,
}

/// 生成 JWT（24 小时有效）
pub fn create_token(user_id: i64, username: &str, is_admin: bool, secret: &str) -> Result<String, AppError> {
    let exp = chrono::Utc::now().timestamp() as usize + 24 * 3600;
    let claims = Claims {
        sub: user_id,
        username: username.to_string(),
        is_admin,
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("生成令牌失败: {e}")))
}

/// 解析 JWT
pub fn decode_token(token: &str, secret: &str) -> Result<Claims, AppError> {
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .map(|d| d.claims)
    .map_err(|_| AppError::Unauthorized("令牌无效或已过期".into()))
}

/// 当前登录用户（从请求头提取并校验 JWT）
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: i64,
    pub username: String,
    pub is_admin: bool,
}

/// 从 Authorization 头提取 Bearer token
fn extract_bearer(parts: &Parts) -> Result<String, AppError> {
    parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AppError::Unauthorized("缺少认证令牌".into()))
}

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let token = extract_bearer(parts)?;
        let claims = decode_token(&token, &state.config.jwt_secret)?;
        Ok(AuthUser {
            id: claims.sub,
            username: claims.username,
            is_admin: claims.is_admin,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_password() {
        let hash = hash_password("secret123").unwrap();
        assert!(verify_password("secret123", &hash));
    }

    #[test]
    fn test_wrong_password_is_rejected() {
        let hash = hash_password("secret123").unwrap();
        assert!(!verify_password("wrong-password", &hash));
    }

    #[test]
    fn test_same_password_hashes_differently_due_to_salt() {
        let h1 = hash_password("same").unwrap();
        let h2 = hash_password("same").unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_jwt_roundtrip() {
        let token = create_token(42, "alice", true, "my-secret").unwrap();
        let claims = decode_token(&token, "my-secret").unwrap();
        assert_eq!(claims.sub, 42);
        assert_eq!(claims.username, "alice");
        assert!(claims.is_admin);
    }

    #[test]
    fn test_jwt_rejects_wrong_secret() {
        let token = create_token(1, "bob", false, "secret-a").unwrap();
        assert!(decode_token(&token, "secret-b").is_err());
    }
}
