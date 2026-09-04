pub mod auth;
pub mod chat;
pub mod conversations;
pub mod kb;

use axum::Router;

use crate::state::AppState;

/// 汇总所有业务路由
pub fn router() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::router())
        .nest("/conversations", conversations::router())
        .nest("/kb", kb::router())
        .merge(chat::router())
}
