use anyhow::Result;
use rig_core::{
    client::{CompletionClient, EmbeddingsClient, ProviderClient},
    completion::{CompletionModel, Message},
    embeddings::EmbeddingModel,
    providers::openai,
    streaming::{StreamedAssistantContent, StreamingCompletionResponse},
};

use crate::config::Config;

/// RAG 组件：封装 rig 客户端（对接阿里云百炼）+ 大模型 + embedding 模型
#[derive(Clone)]
pub struct Rag {
    client: openai::CompletionsClient,
    llm_model: String,
    embedding_model: String,
    embedding_dims: usize,
}

impl Rag {
    /// 构造：从环境变量读取百炼 API Key，切换到 Chat Completions 兼容模式
    pub fn new(config: &Config) -> Result<Self> {
        let client = openai::Client::from_env()?
            .with_system_instructions_as_messages()
            .completions_api();
        Ok(Self {
            client,
            llm_model: config.llm_model.clone(),
            embedding_model: config.embedding_model.clone(),
            embedding_dims: config.embedding_dims,
        })
    }

    /// 批量向量化，返回 `Vec<f32>`（便于存 Redis 的 FLOAT32 向量）
    pub async fn embed_texts(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let model = self
            .client
            .embedding_model_with_ndims(&self.embedding_model, self.embedding_dims);
        let embeddings = model.embed_texts(texts).await?;
        Ok(embeddings
            .into_iter()
            .map(|e| e.vec.into_iter().map(|x| x as f32).collect())
            .collect())
    }

    /// 单个文本向量化
    pub async fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let mut v = self.embed_texts(vec![text.to_string()]).await?;
        Ok(v.pop().unwrap_or_default())
    }

    /// 流式生成回答。
    /// `messages` 必须是「最后一条 = 用户问题」的完整对话（含 system 与历史）。
    pub async fn stream(&self, messages: Vec<Message>) -> Result<StreamingCompletionResponse> {
        let mut messages = messages;
        let prompt = messages
            .pop()
            .ok_or_else(|| anyhow::anyhow!("消息列表不能为空"))?;
        let model = self.client.completion_model(&self.llm_model);
        let request = model
            .completion_request(prompt)
            .messages(messages)
            .temperature(0.3)
            .build();
        Ok(model.stream(request).await?)
    }
}

/// 把流式响应里的文本增量收集成完整答案
pub async fn collect_stream_text(
    mut stream: StreamingCompletionResponse,
) -> Result<String> {
    use futures::StreamExt;

    let mut answer = String::new();
    while let Some(item) = stream.next().await {
        if let StreamedAssistantContent::Text(text) = item? {
            answer.push_str(text.text());
        }
    }
    Ok(answer)
}
