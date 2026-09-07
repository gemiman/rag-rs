---
name: rebuild-app
description: 重新构建「RAG 知识库问答系统」的生产版本（后端 release 二进制 + 前端 dist 静态文件）。当用户想构建、打包、生成生产版本时使用。
---

# 重新构建 RAG 知识库问答系统

把系统重新构建成**生产版本**。注意：这是 Web 应用，没有桌面安装包（.exe），产物是「后端 release 二进制 + 前端静态文件」。按下面步骤：

## 1. 构建后端（release）

```bash
cd backend && cargo build --release
```

- **后台运行**（release 编译耗时较长，可能几分钟，务必后台，不要前台阻塞卡住）。

## 2. 构建前端（静态文件）

```bash
cd frontend && npm run build
```

- 生成静态文件到 `frontend/dist/`。

## 3. 确认产物

- 后端二进制：`backend/target/release/rag-backend.exe`
- 前端静态文件：`frontend/dist/`

确认这两个产物已生成，并把完整路径和文件大小告诉用户。

## 出错了怎么办

把关键错误整理成大白话告诉用户，并给排查建议（例如前端报错看是不是没 `npm install`，后端报错看编译错误信息）。
