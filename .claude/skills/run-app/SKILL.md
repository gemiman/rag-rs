---
name: run-app
description: 启动「RAG 知识库问答系统」的开发模式。当用户想运行、启动、打开这个系统，或想看到代码改动的实际效果时使用。
---

# 启动 RAG 知识库问答系统

按顺序启动三个服务。**后端和前端都是长驻进程，务必用后台方式运行，绝不能前台阻塞卡住。**

## 1. 启动 Redis Stack（向量库 + 缓存）

```bash
docker compose up -d
```

- 这是后台容器（带 `-d`），命令本身不会卡住，跑完就返回。
- 如果报「docker 未运行 / Cannot connect」，提醒用户先启动 Docker Desktop。

## 2. 启动后端（Rust）

```bash
cd backend && cargo run
```

- **后台运行**（长驻进程，会一直开着直到关闭）。
- 首次 `cargo run` 编译耗时较长（可能几分钟），务必后台跑并耐心等。
- 启动成功标志：日志出现 `服务已启动: http://127.0.0.1:8000`。
- 前置条件：`backend/.env` 里有 `OPENAI_API_KEY`（百炼 Key），否则会报「缺少环境变量 OPENAI_API_KEY」。

## 3. 启动前端（Vue/Vite）

```bash
cd frontend && npm run dev
```

- **后台运行**（长驻进程）。
- 首次运行前如果 `node_modules` 不存在，先 `npm install`。
- 启动成功标志：日志出现 `Local: http://localhost:5173`。

## 4. 确认

三个服务都起来后，告诉用户打开浏览器访问 **http://localhost:5173**，用 `admin / 123456` 登录。

## 出错了怎么办

把关键错误整理成大白话告诉用户，并给排查建议：

- **Redis 连不上** → 检查 Docker Desktop 是否启动、`docker compose up -d` 是否成功（`docker ps` 看有没有 `rag-redis`）。
- **后端报「缺少环境变量」** → 检查 `backend/.env` 是否存在、有没有填 API Key。
- **前端起不来** → 检查是否 `npm install` 过、端口 5173 是否被占用。
