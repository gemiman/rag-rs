use std::convert::Infallible;

use axum::extract::State;
use axum::response::sse::{Event, Sse};
use axum::routing::post;
use axum::{Json, Router};
use futures::Stream;
use rig_core::completion::Message;
use rig_core::streaming::StreamedAssistantContent;
use serde_json::json;

use crate::auth::AuthUser;
use crate::error::AppError;
use crate::models;
use crate::schemas::{ChatRequest, Citation};
use crate::state::AppState;
use crate::vectorstore::{self, SearchResult};

const SYSTEM_PROMPT: &str = "你是一个电商商品知识库问答助手。请严格基于提供的知识库片段回答用户问题，\
并在回答中用 [0]、[1] 等编号标注引用了哪些片段。如果知识库中没有相关信息，请如实说明无法回答，不要编造。\
回答要准确、简洁、友好。";

/// 已准备好、待流式输出的内容
enum Prepared {
    Cached { answer: String },
    Fresh { messages: Vec<Message> },
}

pub fn router() -> Router<AppState> {
    Router::new().route("/chat", post(chat))
}

/// 聊天：检索知识库 → 流式生成回答 → 返回引用片段（带问答结果缓存）
async fn chat(
    State(state): State<AppState>,
    user: AuthUser,
    Json(req): Json<ChatRequest>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, AppError> {
    if req.question.trim().is_empty() {
        return Err(AppError::BadRequest("问题不能为空".into()));
    }

    // 1. 校验会话归属
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM conversations WHERE id = ? AND user_id = ?",
    )
    .bind(req.conversation_id)
    .bind(user.id)
    .fetch_one(&state.pool)
    .await?;
    if count == 0 {
        return Err(AppError::NotFound("会话不存在".into()));
    }

    // 2. 取最近 8 条历史（转为正序）
    let history: Vec<(String, String)> = sqlx::query_as::<_, (String, String)>(
        "SELECT role, content FROM messages WHERE conversation_id = ? ORDER BY id DESC LIMIT 8",
    )
    .bind(req.conversation_id)
    .fetch_all(&state.pool)
    .await?
    .into_iter()
    .rev()
    .collect();

    // 3. 保存用户消息
    let now = models::now();
    sqlx::query(
        "INSERT INTO messages (conversation_id, role, content, citations, created_at) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(req.conversation_id)
    .bind("user")
    .bind(&req.question)
    .bind(Option::<String>::None)
    .bind(&now)
    .execute(&state.pool)
    .await?;

    // 4. 检查问答结果缓存
    let mut redis = state.redis.clone();
    let cached = crate::cache::get_answer(&mut redis, &req.question).await?;

    // 5. 未命中时检索 + 构建消息
    let (citations, prepared) = if let Some((answer, cit_json)) = cached {
        let citations: Vec<Citation> = serde_json::from_str(&cit_json).unwrap_or_default();
        (citations, Prepared::Cached { answer })
    } else {
        let query_vec = state.rag.embed_one(&req.question).await?;
        let results = vectorstore::search(&mut redis, &query_vec, 5).await?;

        let citations: Vec<Citation> = results
            .iter()
            .enumerate()
            .map(|(i, r)| Citation {
                index: i,
                content: r.text.clone(),
                source: r.filename.clone(),
            })
            .collect();

        let context = build_context(&results);
        let mut messages = vec![Message::system(SYSTEM_PROMPT)];
        for (role, content) in history {
            let msg = if role == "user" {
                Message::user(content)
            } else {
                Message::assistant(content)
            };
            messages.push(msg);
        }
        messages.push(Message::user(format!("{context}\n\n用户问题：{}", req.question)));

        (citations, Prepared::Fresh { messages })
    };

    // 6. 流式输出（SSE）
    let conversation_id = req.conversation_id;
    let question = req.question.clone();
    let rag = state.rag.clone();
    let pool = state.pool.clone();
    let mut cache_conn = state.redis.clone();

    let stream = async_stream::stream! {
        let mut full_answer = String::new();

        match prepared {
            Prepared::Cached { answer } => {
                full_answer = answer.clone();
                yield Ok(Event::default().data(json!({ "type": "delta", "text": answer }).to_string()));
            }
            Prepared::Fresh { messages } => match rag.stream(messages).await {
                Ok(mut s) => {
                    use futures::StreamExt;
                    while let Some(item) = s.next().await {
                        if let Ok(StreamedAssistantContent::Text(t)) = item {
                            let text = t.text().to_string();
                            full_answer.push_str(&text);
                            yield Ok(Event::default().data(json!({ "type": "delta", "text": text }).to_string()));
                        }
                    }
                }
                Err(e) => {
                    yield Ok(Event::default().data(json!({ "type": "error", "message": e.to_string() }).to_string()));
                }
            },
        }

        // 保存助手消息
        let now = models::now();
        let cit_json = serde_json::to_string(&citations).unwrap_or_else(|_| "[]".into());
        let _ = sqlx::query(
            "INSERT INTO messages (conversation_id, role, content, citations, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(conversation_id)
        .bind("assistant")
        .bind(&full_answer)
        .bind(Some(&cit_json))
        .bind(&now)
        .execute(&pool)
        .await;
        let _ = sqlx::query("UPDATE conversations SET updated_at = ? WHERE id = ?")
            .bind(now)
            .bind(conversation_id)
            .execute(&pool)
            .await;

        // 写入结果缓存
        let _ = crate::cache::set_answer(&mut cache_conn, &question, &full_answer, &cit_json).await;

        yield Ok(Event::default().data(json!({ "type": "done", "citations": citations }).to_string()));
    };

    Ok(Sse::new(stream))
}

/// 把检索结果拼成给模型看的上下文
fn build_context(results: &[SearchResult]) -> String {
    let mut ctx = String::from("以下是知识库中相关的商品信息片段：\n");
    for (i, r) in results.iter().enumerate() {
        ctx.push_str(&format!("[{i}]（来源：{}）{}\n", r.filename, r.text));
    }
    ctx
}
