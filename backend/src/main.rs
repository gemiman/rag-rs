mod api_doc;
mod auth;
mod cache;
mod config;
mod db;
mod error;
mod ingestion;
mod models;
mod rag;
mod routes;
mod schemas;
mod state;
mod vectorstore;

use axum::Router;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api_doc::ApiDoc;
use crate::config::Config;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = Config::from_env()?;

    // 数据库
    let pool = db::init_pool(&config.database_url).await?;
    seed_admin(&pool, &config.admin_password).await?;

    // Redis（向量库 + 缓存）
    let redis_client = redis::Client::open(config.redis_url.as_str())?;
    let mut redis = redis_client.get_connection_manager().await?;
    vectorstore::ensure_index(&mut redis, config.embedding_dims).await?;

    // RAG（rig 客户端，对接百炼）
    let rag = rag::Rag::new(&config)?;

    let state = AppState {
        config: config.clone(),
        pool,
        redis,
        rag,
    };

    let app = Router::new()
        .merge(
            SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()),
        )
        .nest("/api", routes::router())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("服务已启动: http://{addr}");
    tracing::info!("API 文档: http://{addr}/swagger-ui");
    axum::serve(listener, app).await?;
    Ok(())
}

/// 首次启动时创建管理员账号（密码从 ADMIN_PASSWORD 环境变量读取）
async fn seed_admin(pool: &sqlx::SqlitePool, admin_password: &str) -> anyhow::Result<()> {
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users WHERE username = 'admin'")
        .fetch_one(pool)
        .await?;

    if count == 0 {
        let hash =
            crate::auth::hash_password(admin_password).map_err(|e| anyhow::anyhow!("密码加密失败: {e:?}"))?;
        sqlx::query(
            "INSERT INTO users (username, password_hash, is_admin, created_at) VALUES ('admin', ?, 1, ?)",
        )
        .bind(&hash)
        .bind(models::now())
        .execute(pool)
        .await?;
        tracing::info!("已创建管理员账号 admin");
    }
    Ok(())
}
