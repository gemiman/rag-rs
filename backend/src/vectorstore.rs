use anyhow::Result;
use redis::aio::ConnectionManager;

/// Redis key 前缀
const CHUNK_PREFIX: &str = "chunk:";
const DOC_CHUNKS_PREFIX: &str = "doc_chunks:";
const INDEX_NAME: &str = "idx_chunks";

/// 一段知识库片段（chunk）
#[derive(Debug, Clone)]
pub struct Chunk {
    /// Redis key 后缀（如 "3:0" 表示文档 3 的第 0 块）
    pub id: String,
    pub text: String,
    pub document_id: i64,
    pub filename: String,
    pub chunk_index: i64,
    pub vector: Vec<f32>,
}

/// 检索结果
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub score: f32,
    pub text: String,
    pub document_id: i64,
    pub filename: String,
    pub chunk_index: i64,
}

/// f32 向量 → 小端字节（Redis FLOAT32 格式）
pub fn f32_to_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

/// 建 RediSearch 向量索引（幂等，已存在则跳过）
pub async fn ensure_index(conn: &mut ConnectionManager, dims: usize) -> Result<()> {
    // 先查索引是否存在
    let info: redis::RedisResult<redis::Value> =
        redis::cmd("FT.INFO").arg(INDEX_NAME).query_async(conn).await;
    if info.is_ok() {
        return Ok(());
    }

    redis::cmd("FT.CREATE")
        .arg(INDEX_NAME)
        .arg("ON")
        .arg("HASH")
        .arg("PREFIX")
        .arg("1")
        .arg(CHUNK_PREFIX)
        .arg("SCHEMA")
        .arg("text")
        .arg("TEXT")
        .arg("vector")
        .arg("VECTOR")
        .arg("HNSW")
        .arg("6")
        .arg("TYPE")
        .arg("FLOAT32")
        .arg("DIM")
        .arg(dims)
        .arg("DISTANCE_METRIC")
        .arg("COSINE")
        .query_async::<()>(conn)
        .await?;
    Ok(())
}

/// 批量写入 chunk 到 Redis（HSET + 记录到文档的 chunk 集合）
pub async fn add_chunks(conn: &mut ConnectionManager, chunks: &[Chunk]) -> Result<()> {
    for c in chunks {
        let key = format!("{CHUNK_PREFIX}{}", c.id);
        let vec_bytes = f32_to_bytes(&c.vector);
        redis::cmd("HSET")
            .arg(&key)
            .arg("text")
            .arg(&c.text)
            .arg("document_id")
            .arg(c.document_id)
            .arg("filename")
            .arg(&c.filename)
            .arg("chunk_index")
            .arg(c.chunk_index)
            .arg("vector")
            .arg(&vec_bytes)
            .query_async::<()>(conn)
            .await?;

        let set_key = format!("{DOC_CHUNKS_PREFIX}{}", c.document_id);
        redis::cmd("SADD")
            .arg(&set_key)
            .arg(&key)
            .query_async::<()>(conn)
            .await?;
    }
    Ok(())
}

/// 向量相似度检索（返回 top_k 个片段，按相似度从高到低）
pub async fn search(
    conn: &mut ConnectionManager,
    query_vec: &[f32],
    top_k: usize,
) -> Result<Vec<SearchResult>> {
    let vec_bytes = f32_to_bytes(query_vec);
    let query = format!("*=>[KNN {top_k} @vector $q AS score]");

    let result: redis::Value = redis::cmd("FT.SEARCH")
        .arg(INDEX_NAME)
        .arg(&query)
        .arg("PARAMS")
        .arg("2")
        .arg("q")
        .arg(&vec_bytes)
        .arg("SORTBY")
        .arg("score")
        .arg("ASC")
        .arg("DIALECT")
        .arg("2")
        .arg("RETURN")
        .arg("5")
        .arg("text")
        .arg("document_id")
        .arg("filename")
        .arg("chunk_index")
        .arg("score")
        .query_async(conn)
        .await?;

    Ok(parse_search_result(&result))
}

/// 删除某个文档的所有 chunk
pub async fn delete_document(conn: &mut ConnectionManager, document_id: i64) -> Result<()> {
    let set_key = format!("{DOC_CHUNKS_PREFIX}{document_id}");
    let keys: Vec<String> = redis::cmd("SMEMBERS")
        .arg(&set_key)
        .query_async(conn)
        .await?;

    if !keys.is_empty() {
        let mut cmd = redis::cmd("DEL");
        for k in &keys {
            cmd.arg(k);
        }
        cmd.query_async::<()>(conn).await?;
    }
    redis::cmd("DEL").arg(&set_key).query_async::<()>(conn).await?;
    Ok(())
}

/// 获取某文档的所有 chunk 文本（按 chunk_index 排序）
pub async fn get_chunk_texts(
    conn: &mut ConnectionManager,
    document_id: i64,
) -> Result<Vec<(i64, String)>> {
    let set_key = format!("{DOC_CHUNKS_PREFIX}{document_id}");
    let keys: Vec<String> = redis::cmd("SMEMBERS").arg(&set_key).query_async(conn).await?;

    let mut out = Vec::new();
    for k in keys {
        let idx: i64 = redis::cmd("HGET")
            .arg(&k)
            .arg("chunk_index")
            .query_async(conn)
            .await
            .unwrap_or(0);
        let text: String = redis::cmd("HGET")
            .arg(&k)
            .arg("text")
            .query_async(conn)
            .await
            .unwrap_or_default();
        out.push((idx, text));
    }
    out.sort_by_key(|(i, _)| *i);
    Ok(out)
}

/// 解析 FT.SEARCH 返回结果：[total, key1, [f1, v1, ...], key2, [...], ...]
fn parse_search_result(v: &redis::Value) -> Vec<SearchResult> {
    let arr = match v {
        redis::Value::Array(items) => items,
        _ => return vec![],
    };

    let mut out = Vec::new();
    let mut i = 1; // 跳过 total
    while i + 1 < arr.len() {
        let fields = match &arr[i + 1] {
            redis::Value::Array(f) => f,
            _ => break,
        };

        let mut r = SearchResult {
            score: 0.0,
            text: String::new(),
            document_id: 0,
            filename: String::new(),
            chunk_index: 0,
        };

        let mut j = 0;
        while j + 1 < fields.len() {
            let name = v_str(&fields[j]);
            let val = &fields[j + 1];
            match name.as_str() {
                "text" => r.text = v_str(val),
                "document_id" => r.document_id = v_i64(val),
                "filename" => r.filename = v_str(val),
                "chunk_index" => r.chunk_index = v_i64(val),
                "score" => r.score = v_f32(val),
                _ => {}
            }
            j += 2;
        }

        out.push(r);
        i += 2;
    }
    out
}

fn v_str(v: &redis::Value) -> String {
    match v {
        redis::Value::BulkString(b) => String::from_utf8_lossy(b).to_string(),
        redis::Value::SimpleString(s) => s.clone(),
        redis::Value::Int(i) => i.to_string(),
        redis::Value::Double(d) => d.to_string(),
        _ => String::new(),
    }
}

fn v_i64(v: &redis::Value) -> i64 {
    match v {
        redis::Value::Int(i) => *i,
        redis::Value::BulkString(b) => String::from_utf8_lossy(b).parse().unwrap_or(0),
        _ => 0,
    }
}

fn v_f32(v: &redis::Value) -> f32 {
    match v {
        redis::Value::Double(d) => *d as f32,
        redis::Value::BulkString(b) => String::from_utf8_lossy(b).parse().unwrap_or(0.0),
        redis::Value::Int(i) => *i as f32,
        _ => 0.0,
    }
}
