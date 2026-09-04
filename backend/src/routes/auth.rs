use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::json;

use crate::auth::{create_token, hash_password, verify_password, AuthUser};
use crate::error::AppError;
use crate::models::{self, User};
use crate::schemas::{AuthResponse, ChangePasswordRequest, LoginRequest, RegisterRequest, UserInfo};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/change-password", post(change_password))
        .route("/me", get(me))
}

/// 校验当前用户是否为管理员
pub fn require_admin(user: &AuthUser) -> Result<(), AppError> {
    if user.is_admin {
        Ok(())
    } else {
        Err(AppError::Forbidden("需要管理员权限".into()))
    }
}

/// 注册
async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    if req.username.chars().count() < 3 {
        return Err(AppError::BadRequest("用户名至少 3 个字符".into()));
    }
    if req.password.chars().count() < 6 {
        return Err(AppError::BadRequest("密码至少 6 个字符".into()));
    }

    let exists = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_one(&state.pool)
        .await?;
    if exists > 0 {
        return Err(AppError::Conflict("用户名已存在".into()));
    }

    let hash = hash_password(&req.password)?;
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO users (username, password_hash, is_admin, created_at) VALUES (?, ?, 0, ?) RETURNING id",
    )
    .bind(&req.username)
    .bind(&hash)
    .bind(models::now())
    .fetch_one(&state.pool)
    .await?;

    let token = create_token(id, &req.username, false, &state.config.jwt_secret)?;
    Ok(Json(AuthResponse {
        token,
        user: UserInfo {
            id,
            username: req.username,
            is_admin: false,
        },
    }))
}

/// 登录
async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::Unauthorized("用户名或密码错误".into()))?;

    if !verify_password(&req.password, &user.password_hash) {
        return Err(AppError::Unauthorized("用户名或密码错误".into()));
    }

    let token = create_token(user.id, &user.username, user.is_admin, &state.config.jwt_secret)?;
    Ok(Json(AuthResponse {
        token,
        user: UserInfo {
            id: user.id,
            username: user.username,
            is_admin: user.is_admin,
        },
    }))
}

/// 修改密码
async fn change_password(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<ChangePasswordRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if req.new_password.chars().count() < 6 {
        return Err(AppError::BadRequest("新密码至少 6 个字符".into()));
    }

    let db_user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(user.id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("用户不存在".into()))?;

    if !verify_password(&req.old_password, &db_user.password_hash) {
        return Err(AppError::BadRequest("原密码错误".into()));
    }

    let new_hash = hash_password(&req.new_password)?;
    sqlx::query("UPDATE users SET password_hash = ? WHERE id = ?")
        .bind(&new_hash)
        .bind(user.id)
        .execute(&state.pool)
        .await?;

    Ok(Json(json!({ "message": "密码修改成功" })))
}

/// 获取当前用户信息
async fn me(user: AuthUser) -> Json<UserInfo> {
    Json(UserInfo {
        id: user.id,
        username: user.username,
        is_admin: user.is_admin,
    })
}
