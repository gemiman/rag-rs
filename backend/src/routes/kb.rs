use axum::extract::{Multipart, Path, State};
use axum::routing::{delete, get};
use axum::{Json, Router};
use serde_json::json;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::ingestion::{parse_document, split_text};
use crate::models::{self, Document};
use crate::routes::auth::require_admin;
use crate::schemas::{ChunkPreview, DocumentResponse};
use crate::state::AppState;
use crate::vectorstore::{self, Chunk};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/documents", get(list).post(upload))
        .route("/documents/{id}", delete(delete_document))
        .route("/documents/{id}/chunks", get(preview_chunks))
}

/// 文档列表（仅管理员）
async fn list(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<Vec<DocumentResponse>>, AppError> {
    require_admin(&user)?;
    let rows = sqlx::query_as::<_, Document>("SELECT * FROM documents ORDER BY id DESC")
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(rows.into_iter().map(to_response).collect()))
}

/// 上传文档（仅管理员），异步解析入库
async fn upload(
    State(state): State<AppState>,
    user: AuthUser,
    mut multipart: Multipart,
) -> Result<Json<DocumentResponse>, AppError> {
    require_admin(&user)?;

    let mut filename = None;
    let mut bytes = Vec::new();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("上传解析失败: {e}")))?
    {
        if let Some(name) = field.file_name().map(|s| s.to_string()) {
            filename = Some(name);
        }
        bytes = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(format!("读取文件失败: {e}")))?
            .to_vec();
    }

    let filename = filename.ok_or_else(|| AppError::BadRequest("未选择文件".into()))?;
    let size = bytes.len() as i64;
    let file_type = std::path::Path::new(&filename)
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();

    let now = models::now();
    let doc_id = sqlx::query_scalar::<_, i64>(
        "INSERT INTO documents (filename, file_type, size, chunk_count, status, created_at) VALUES (?, ?, ?, 0, 'processing', ?) RETURNING id",
    )
    .bind(&filename)
    .bind(&file_type)
    .bind(size)
    .bind(&now)
    .fetch_one(&state.pool)
    .await?;

    // 后台异步导入
    let state_clone = state.clone();
    let fname = filename.clone();
    tokio::spawn(async move {
        ingest_document(state_clone, doc_id, fname, bytes).await;
    });

    Ok(Json(DocumentResponse {
        id: doc_id,
        filename,
        file_type,
        size,
        chunk_count: 0,
        status: "processing".into(),
        created_at: now,
    }))
}

/// 删除文档及其向量（仅管理员）
async fn delete_document(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_admin(&user)?;

    let mut redis = state.redis.clone();
    vectorstore::delete_document(&mut redis, id).await?;
    sqlx::query("DELETE FROM documents WHERE id = ?")
        .bind(id)
        .execute(&state.pool)
        .await?;

    Ok(Json(json!({ "message": "文档已删除" })))
}

/// 预览文档切分后的片段（仅管理员）
async fn preview_chunks(
    State(state): State<AppState>,
    user: AuthUser,
    Path(id): Path<i64>,
) -> Result<Json<Vec<ChunkPreview>>, AppError> {
    require_admin(&user)?;

    let mut redis = state.redis.clone();
    let chunks = vectorstore::get_chunk_texts(&mut redis, id).await?;
    Ok(Json(
        chunks
            .into_iter()
            .enumerate()
            .map(|(i, (_, text))| ChunkPreview {
                index: i,
                content: text,
            })
            .collect(),
    ))
}

/// 后台导入流程：解析 → 切块 → 向量化 → 入 Redis → 更新状态
async fn ingest_document(state: AppState, doc_id: i64, filename: String, bytes: Vec<u8>) {
    let result = do_ingest(&state, doc_id, &filename, &bytes).await;

    if let Err(e) = result {
        let _ = sqlx::query("UPDATE documents SET status = 'error' WHERE id = ?")
            .bind(doc_id)
            .execute(&state.pool)
            .await;
        tracing::error!("文档 {doc_id}（{filename}）导入失败: {e:#}");
    }
}

async fn do_ingest(state: &AppState, doc_id: i64, filename: &str, bytes: &[u8]) -> anyhow::Result<()> {
    let text = parse_document(filename, bytes)?;
    let chunk_texts = split_text(&text, 500, 50);
    if chunk_texts.is_empty() {
        anyhow::bail!("文档内容为空，无法切分");
    }

    // 批量向量化（带缓存，重复文本直接命中，省 API 调用）
    let mut redis = state.redis.clone();
    let vectors = crate::cache::embed_texts_cached(&mut redis, &state.rag, chunk_texts.clone()).await?;

    // 组装 chunk
    let chunks: Vec<Chunk> = chunk_texts
        .into_iter()
        .zip(vectors)
        .enumerate()
        .map(|(i, (text, vector))| Chunk {
            id: format!("{doc_id}:{i}"),
            text,
            document_id: doc_id,
            filename: filename.to_string(),
            chunk_index: i as i64,
            vector,
        })
        .collect();

    let chunk_count = chunks.len();
    vectorstore::add_chunks(&mut redis, &chunks).await?;

    sqlx::query("UPDATE documents SET status = 'done', chunk_count = ? WHERE id = ?")
        .bind(chunk_count as i64)
        .bind(doc_id)
        .execute(&state.pool)
        .await?;

    tracing::info!("文档 {doc_id}（{filename}）导入完成，共 {chunk_count} 个片段");
    Ok(())
}

fn to_response(d: Document) -> DocumentResponse {
    DocumentResponse {
        id: d.id,
        filename: d.filename,
        file_type: d.file_type,
        size: d.size,
        chunk_count: d.chunk_count,
        status: d.status,
        created_at: d.created_at,
    }
}
