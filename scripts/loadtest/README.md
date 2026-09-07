# 压力测试（k6）

用 [k6](https://k6.io/) 模拟 100 人同时使用本系统，分两层测：**100 人高并发压本地接口**（不烧钱）+ **15 并发压真实问答链路**（真调百炼大模型）。

## 文件说明

| 文件 | 作用 |
|---|---|
| `common.js` | 公共工具（登录、随机用户名、唯一问题等），被其它脚本复用 |
| `auth.js` | 场景① 注册 + 登录（100 并发，测 argon2 密码加密瓶颈） |
| `conversations.js` | 场景② 会话管理（100 并发，测 SQLite 并发读写） |
| `chat-cache.js` | 场景③ 问答·缓存命中（100 并发，固定问题命中缓存，测 SSE 回放） |
| `chat-real.js` | 场景④ 真实问答链路（15 并发，唯一问题真走百炼，⚠️ 烧钱） |
| `cleanup.py` | 压测后清理 `loadtest_` 前缀测试数据（自动备份后清理） |
| `result_*.txt` | 每次压测的原始输出报告（可删） |

## 运行前提

1. 后端已启动（`cd backend && cargo run`）
2. Redis 已启动（`docker compose up -d`）
3. 知识库已上传文档并索引完成（否则问答检索为空）

## 怎么跑

```bash
# 安装 k6（一次性）：winget install --id GrafanaLabs.k6

cd scripts/loadtest
k6 run auth.js           # 场景①
k6 run conversations.js  # 场景②
k6 run chat-cache.js     # 场景③
k6 run chat-real.js      # 场景④（真调百炼，注意费用）
```

每个场景约 2 分钟，会从低并发逐步加到目标（10 → 50 → 100），方便观察拐点。

## 压测后清理

压测会用 `loadtest_` 前缀账号产生大量测试数据，跑完执行：

```bash
python scripts/loadtest/cleanup.py
```

会自动备份数据库（生成 `rag.db.bak_时间戳`）后，删除所有 `loadtest_` 测试用户及其会话、消息。
