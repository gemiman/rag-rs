use anyhow::{Context, Result};

/// 应用配置，从 `.env` 文件 / 环境变量读取
#[derive(Clone, Debug)]
pub struct Config {
    /// 阿里云百炼（DashScope）API Key
    pub api_key: String,
    /// 百炼 OpenAI 兼容接口地址
    pub base_url: String,
    /// 大模型名称
    pub llm_model: String,
    /// Embedding 模型名称
    pub embedding_model: String,
    /// Embedding 向量维度
    pub embedding_dims: usize,
    /// SQLite 数据库连接串
    pub database_url: String,
    /// Redis 连接串
    pub redis_url: String,
    /// 监听地址
    pub host: String,
    /// 监听端口
    pub port: u16,
    /// JWT 签名密钥
    pub jwt_secret: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // 读取 .env 文件（不存在则忽略，允许直接用环境变量）
        dotenvy::dotenv().ok();

        Ok(Config {
            api_key: env("OPENAI_API_KEY")?,
            base_url: env_or(
                "OPENAI_BASE_URL",
                "https://dashscope.aliyuncs.com/compatible-mode/v1",
            ),
            llm_model: env_or("LLM_MODEL", "qwen-plus"),
            embedding_model: env_or("EMBEDDING_MODEL", "text-embedding-v3"),
            embedding_dims: env_or("EMBEDDING_DIMS", "1024")
                .parse()
                .context("EMBEDDING_DIMS 必须是数字")?,
            database_url: env_or("DATABASE_URL", "sqlite://data/rag.db"),
            redis_url: env_or("REDIS_URL", "redis://127.0.0.1:6379"),
            host: env_or("HOST", "127.0.0.1"),
            port: env_or("PORT", "8000").parse().context("PORT 必须是数字")?,
            jwt_secret: env("JWT_SECRET")?,
        })
    }
}

fn env(key: &str) -> Result<String> {
    std::env::var(key).with_context(|| format!("缺少环境变量 {key}（请检查 .env 文件）"))
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}
