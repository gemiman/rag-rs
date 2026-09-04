use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// ===== 认证 =====

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChangePasswordRequest {
    pub old_password: String,
    pub new_password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserInfo {
    pub id: i64,
    pub username: String,
    pub is_admin: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserInfo,
}

// ===== 会话 =====

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RenameConversationRequest {
    pub title: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConversationResponse {
    pub id: i64,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

// ===== 引用片段 & 消息 =====

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct Citation {
    pub index: usize,
    pub content: String,
    pub source: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MessageResponse {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub citations: Vec<Citation>,
    pub created_at: String,
}

// ===== 聊天 =====

#[derive(Debug, Deserialize, ToSchema)]
pub struct ChatRequest {
    pub conversation_id: i64,
    pub question: String,
}

// ===== 知识库 =====

#[derive(Debug, Serialize, ToSchema)]
pub struct DocumentResponse {
    pub id: i64,
    pub filename: String,
    pub file_type: String,
    pub size: i64,
    pub chunk_count: i64,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChunkPreview {
    pub index: usize,
    pub content: String,
}
