use utoipa::OpenApi;

use crate::schemas::{
    AuthResponse, ChangePasswordRequest, ChatRequest, ChunkPreview, Citation,
    ConversationResponse, CreateConversationRequest, DocumentResponse, LoginRequest,
    MessageResponse, RegisterRequest, RenameConversationRequest, UserInfo,
};

/// API 文档（相当于 Java 里的 Swagger）
#[derive(OpenApi)]
#[openapi(
    info(
        title = "企业知识库问答系统 API",
        version = "1.0.0",
        description = "基于 Rust + rig 的电商商品知识库 RAG 问答系统"
    ),
    components(schemas(
        RegisterRequest,
        LoginRequest,
        ChangePasswordRequest,
        UserInfo,
        AuthResponse,
        CreateConversationRequest,
        RenameConversationRequest,
        ConversationResponse,
        Citation,
        MessageResponse,
        ChatRequest,
        DocumentResponse,
        ChunkPreview
    ))
)]
pub struct ApiDoc;
