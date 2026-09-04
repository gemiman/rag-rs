use serde::Serialize;
use sqlx::FromRow;

/// 用户
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    pub password_hash: String,
    pub is_admin: bool,
    pub created_at: String,
}

/// 会话
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Conversation {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

/// 消息
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Message {
    pub id: i64,
    pub conversation_id: i64,
    /// "user" 或 "assistant"
    pub role: String,
    pub content: String,
    /// 引用片段（JSON 字符串，可能为空）
    pub citations: Option<String>,
    pub created_at: String,
}

/// 知识库文档
#[derive(Debug, Clone, FromRow, Serialize)]
pub struct Document {
    pub id: i64,
    pub filename: String,
    pub file_type: String,
    pub size: i64,
    pub chunk_count: i64,
    /// "processing" / "done" / "error"
    pub status: String,
    pub created_at: String,
}

/// 当前时间（RFC3339 字符串，可直接排序）
pub fn now() -> String {
    chrono::Utc::now().to_rfc3339()
}
