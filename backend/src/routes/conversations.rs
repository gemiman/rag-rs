use axum::extract::{Path, State};
use axum::routing::{get, put};
use axum::{Json, Router};
use serde_json::json;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models::{self, Conversation, Message};
use crate::schemas::{
    Citation, ConversationResponse, CreateConversationRequest, MessageResponse,
    RenameConversationRequest,
};
use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list).post(create))
        .route("/{id}", put(rename).delete(delete_conversation))
        .route("/{id}/messages", get(messages))
}

/// 校验会话是否属于当前用户
async fn ensure_owner(
    state: &AppState,
    conversation_id: i64,
    user_id: i64,
) -> Result<(), AppError> {
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM conversations WHERE id = ? AND user_id = ?",
    )
    .bind(conversation_id)
    .bind(user_id)
    .fetch_one(&state.pool)
    .await?;
    if count == 0 {
        return Err(AppError::NotFound("会话不存在".into()));
    }
    Ok(())
}

/// 当前用户的会话列表
async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<ConversationResponse>>, AppError> {
    let rows = sqlx::query_as::<_, Conversation>(
        "SELECT * FROM conversations WHERE user_id = ? ORDER BY updated_at DESC",
    )
    .bind(user.id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(
        rows.into_iter()
            .map(|c| ConversationResponse {
                id: c.id,
                title: c.title,
                created_at: c.created_at,
                updated_at: c.updated_at,
            })
            .collect(),
    ))
}

/// 新建会话
async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<CreateConversationRequest>,
) -> Result<Json<ConversationResponse>, AppError> {
    let title = req.title.unwrap_or_else(|| "新对话".to_string());
    let now = models::now();
    let id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO conversations (user_id, title, created_at, updated_at) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(user.id)
    .bind(&title)
    .bind(&now)
    .bind(&now)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(ConversationResponse {
        id,
        title,
        created_at: now.clone(),
        updated_at: now,
    }))
}

/// 重命名会话
async fn rename(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
    Json(req): Json<RenameConversationRequest>,
) -> Result<Json<ConversationResponse>, AppError> {
    ensure_owner(&state, id, user.id).await?;

    let now = models::now();
    sqlx::query("UPDATE conversations SET title = ?, updated_at = ? WHERE id = ?")
        .bind(&req.title)
        .bind(&now)
        .bind(id)
        .execute(&state.pool)
        .await?;

    let c = sqlx::query_as::<_, Conversation>("SELECT * FROM conversations WHERE id = ?")
        .bind(id)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(ConversationResponse {
        id: c.id,
        title: c.title,
        created_at: c.created_at,
        updated_at: c.updated_at,
    }))
}

/// 删除会话（连同消息）
async fn delete_conversation(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    ensure_owner(&state, id, user.id).await?;

    sqlx::query("DELETE FROM messages WHERE conversation_id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;
    sqlx::query("DELETE FROM conversations WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;

    Ok(Json(json!({ "message": "会话已删除" })))
}

/// 会话的历史消息
async fn messages(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Vec<MessageResponse>>, AppError> {
    ensure_owner(&state, id, user.id).await?;

    let rows = sqlx::query_as::<_, Message>(
        "SELECT * FROM messages WHERE conversation_id = ? ORDER BY id ASC",
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(rows.into_iter().map(to_response).collect()))
}

/// 把 DB 消息转成响应（解析 citations JSON）
fn to_response(m: Message) -> MessageResponse {
    let citations = m
        .citations
        .as_deref()
        .and_then(|s| serde_json::from_str::<Vec<Citation>>(s).ok())
        .unwrap_or_default();
    MessageResponse {
        id: m.id,
        role: m.role,
        content: m.content,
        citations,
        created_at: m.created_at,
    }
}
