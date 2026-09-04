use redis::aio::ConnectionManager;
use sqlx::SqlitePool;

use crate::config::Config;
use crate::rag::Rag;

/// 全局共享状态，注入到所有 handler
#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub pool: SqlitePool,
    pub redis: ConnectionManager,
    pub rag: Rag,
}
