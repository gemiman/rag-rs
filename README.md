# 企业知识库问答系统（RAG）

基于 **Rust + rig** 框架的电商商品知识库 RAG 问答系统。用户通过浏览器进行「知识库管理」和「知识库问答」，问答会**引用知识库片段**并在回答中展示来源。

## 技术栈

| 部件 | 技术 |
|---|---|
| 后端 | Rust + Axum（异步 Web 框架） |
| RAG 框架 | rig（Rust 界的 LangChain） |
| 大模型 | 阿里云百炼（DashScope）qwen-plus |
| Embedding | 百炼 text-embedding-v3 |
| 向量数据库 + 缓存 | Redis Stack（Docker 运行） |
| 关系数据库 | SQLite |
| 认证 | JWT + argon2 |
| API 文档 | utoipa + swagger-ui |
| 前端 | Vue 3 + Element Plus + Vite + Pinia |

## 环境要求

- **Rust**（1.91+，本机已装 1.98）
- **Node.js**（18+，本机已装 24）
- **Docker**（用于运行 Redis Stack，本机已装）

## 启动步骤

### 1. 启动 Redis Stack（向量库 + 缓存）

```bash
docker compose up -d
```

### 2. 配置后端（.env）

后端目录 `backend/.env` 已经配置好了 API Key。如果换了 Key，编辑 `backend/.env`：

```
OPENAI_API_KEY=你的百炼APIKey
OPENAI_BASE_URL=https://dashscope.aliyuncs.com/compatible-mode/v1
LLM_MODEL=qwen-plus
EMBEDDING_MODEL=text-embedding-v3
ADMIN_PASSWORD=管理员初始密码
```

### 3. 启动后端

```bash
cd backend
cargo run
```

启动后：
- 后端地址：http://127.0.0.1:8000
- API 文档：http://127.0.0.1:8000/swagger-ui
- 首次启动自动创建管理员 `admin`（初始密码由 `.env` 的 `ADMIN_PASSWORD` 决定，默认 `123456`）

### 4. 启动前端

```bash
cd frontend
npm install   # 首次运行需要
npm run dev
```

打开浏览器访问 **http://localhost:5173**

## 使用说明

1. 用 `admin / 123456` 登录（管理员）。
2. 进入「知识库管理」，上传商品资料（txt / md / csv / xlsx / pdf / docx）。
3. 进入「聊天」，提问商品相关问题，回答会引用知识库片段并高亮展示。
4. 普通用户（自行注册）只能问答，看不到知识库管理菜单。

## 功能清单

- ✅ 知识库管理（仅管理员）：上传、解析、切块、向量化入库、列表、删除、片段预览
- ✅ 知识库问答：检索 + 流式输出 + 引用片段展示
- ✅ 多用户多会话管理
- ✅ 会话记录持久化（重新登录可找回历史对话）
- ✅ 用户注册 / 登录 / 修改密码
- ✅ 管理员账号 admin / 123456，角色权限隔离
- ✅ API 文档（Swagger 等价物）
- ✅ 企业级优化：异步文档导入、Redis 缓存、连接池、数据库索引

## 目录结构

```
├── backend/          # Rust 后端
│   └── src/
│       ├── main.rs       # 入口
│       ├── config.rs     # 配置
│       ├── db.rs         # SQLite
│       ├── auth.rs       # JWT + argon2
│       ├── rag.rs        # rig 客户端 + 大模型 + embedding
│       ├── vectorstore.rs # Redis 向量存储
│       ├── ingestion.rs  # 文档解析 + 切块
│       └── routes/       # auth / chat / conversations / kb
├── frontend/         # Vue 3 前端
├── scripts/          # 压测等脚本
│   └── loadtest/     # k6 压测脚本
├── docker-compose.yml # Redis Stack
└── README.md
```

## 压力测试

用 **k6** 模拟 100 人同时使用，分两层测（脚本在 `scripts/loadtest/` 下）。

**第一层：100 人高并发（不调用大模型）**

| 场景 | 并发 | 错误率 | 平均响应 | P95 | 吞吐量 |
|---|---|---|---|---|---|
| ① 注册 + 登录 | 100 | 0% | 3.77s | 7.48s | 13.6 次/秒 |
| ② 会话管理 | 100 | 0% | 1.76s | 5.70s | 31.5 次/秒 |
| ③ 问答（缓存命中） | 100 | 0% | 2.50s | 7.13s | 22.2 次/秒 |

**第二层：真实问答链路（真调用百炼大模型）**

| 场景 | 并发 | 错误率 | 平均响应 | P95 |
|---|---|---|---|---|
| ④ 真实问答（向量化→检索→大模型→流式） | 15 | 0% | 0.75s | 1.59s |

**结论**：系统在 100 并发下**零崩溃、零报错**（SQLite 无锁错误、SSE 流式无断裂、百炼无超时）。主要瓶颈是**登录/注册的密码加密（argon2）**——CPU 密集，100 并发下平均 3.77s、P95 达 7.48s；真实问答链路健康（15 并发平均 0.75s）。

**如何运行压测**（需先启动 Redis + 后端，且知识库已上传文档）：

```bash
cd scripts/loadtest
k6 run auth.js           # 场景① 注册 + 登录
k6 run conversations.js  # 场景② 会话管理
k6 run chat-cache.js     # 场景③ 问答缓存命中
k6 run chat-real.js      # 场景④ 真实问答（会真实调用百炼，注意费用）
```

> 压测会用 `loadtest_` 前缀账号产生测试数据，测完可用 `scripts/loadtest/` 下的备份/清理方式恢复。

## 更新日志

- 2026-09-07：新增 **k6 压力测试**（`scripts/loadtest/` 6 个脚本：5 个 k6 压测脚本 + 1 个清理脚本），完成 100 人并发压测，验证系统零报错；修正功能清单中「接口限流」的不实描述（实际未实现限流）。
- 2026-09：管理员初始密码从「写死在代码里」改为「从 `.env` 的 `ADMIN_PASSWORD` 读取」（安全优化），默认仍是 admin / 123456。
