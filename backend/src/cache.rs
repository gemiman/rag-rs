use anyhow::Result;
use redis::aio::ConnectionManager;

use crate::rag::Rag;

/// FNV-1a 哈希（确定性，用作缓存 key）
pub fn fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for b in s.as_bytes() {
        hash ^= *b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn f32_to_bytes(v: &[f32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn bytes_to_f32(b: &[u8]) -> Vec<f32> {
    b.chunks_exact(4)
        .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
        .collect()
}

/// 批量向量化（带 Redis 缓存）：已缓存的直接命中，未缓存的才批量调用 API
pub async fn embed_texts_cached(
    redis: &mut ConnectionManager,
    rag: &Rag,
    texts: Vec<String>,
) -> Result<Vec<Vec<f32>>> {
    let mut result: Vec<Option<Vec<f32>>> = vec![None; texts.len()];
    let mut to_embed: Vec<(usize, String)> = Vec::new();

    // 先查缓存
    for (i, t) in texts.iter().enumerate() {
        let key = format!("emb:{}", fnv1a(t));
        let cached: Option<Vec<u8>> = redis::cmd("GET").arg(&key).query_async(redis).await?;
        match cached {
            Some(bytes) => result[i] = Some(bytes_to_f32(&bytes)),
            None => to_embed.push((i, t.clone())),
        }
    }

    // 批量调用 API 处理未缓存的
    if !to_embed.is_empty() {
        let embeddings = rag
            .embed_texts(to_embed.iter().map(|(_, t)| t.clone()).collect())
            .await?;
        for ((i, t), vec) in to_embed.into_iter().zip(embeddings) {
            let key = format!("emb:{}", fnv1a(&t));
            let bytes = f32_to_bytes(&vec);
            redis::cmd("SET")
                .arg(&key)
                .arg(&bytes)
                .query_async::<()>(redis)
                .await?;
            result[i] = Some(vec);
        }
    }

    Ok(result.into_iter().map(|v| v.unwrap_or_default()).collect())
}

/// 读取问答结果缓存（命中返回 (answer, citations_json)）
pub async fn get_answer(
    redis: &mut ConnectionManager,
    question: &str,
) -> Result<Option<(String, String)>> {
    let key = format!("ans:{}", fnv1a(question));
    let cached: Option<String> = redis::cmd("GET").arg(&key).query_async(redis).await?;
    Ok(cached.map(|s| match s.split_once('\u{1}') {
        Some((a, c)) => (a.to_string(), c.to_string()),
        None => (s, "[]".to_string()),
    }))
}

/// 写入问答结果缓存（1 小时过期）
pub async fn set_answer(
    redis: &mut ConnectionManager,
    question: &str,
    answer: &str,
    citations_json: &str,
) -> Result<()> {
    let key = format!("ans:{}", fnv1a(question));
    let value = format!("{answer}\u{1}{citations_json}");
    redis::cmd("SET")
        .arg(&key)
        .arg(&value)
        .arg("EX")
        .arg(3600)
        .query_async::<()>(redis)
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fnv1a_empty_string() {
        assert_eq!(fnv1a(""), 0xcbf29ce484222325);
    }

    #[test]
    fn test_fnv1a_known_value() {
        assert_eq!(fnv1a("a"), 0xaf63dc4c8601ec8c);
    }

    #[test]
    fn test_fnv1a_is_deterministic() {
        assert_eq!(fnv1a("hello world"), fnv1a("hello world"));
    }

    #[test]
    fn test_fnv1a_different_inputs_differ() {
        assert_ne!(fnv1a("a"), fnv1a("b"));
    }

    #[test]
    fn test_f32_bytes_roundtrip() {
        let v = vec![1.0f32, -0.5f32, 3.14159f32];
        let bytes = f32_to_bytes(&v);
        let back = bytes_to_f32(&bytes);
        assert_eq!(v, back);
    }
}
