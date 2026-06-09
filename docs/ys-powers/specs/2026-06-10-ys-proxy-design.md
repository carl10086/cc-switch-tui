# ys-proxy & Trace Viewer 设计文档

## 1. Objective

在 cc-switch-tui 中集成 API 代理与 Trace 查看功能。用户通过 `ys-proxy cl-kimi` 启动 Claude Code 时，所有 API 请求经过本地代理，完整记录 prompt、messages、tool calls、streaming responses、token usage 等信息。用户可在现有 Web 界面中查看、搜索、对比历史 Trace。

**核心价值：**
- 透明化 Claude Code 的 API 调用过程
- 辅助调试 prompt 变化、context window 使用情况
- 完全本地运行，数据不出本机

---

## 2. Commands / Interfaces

### 2.1 CLI Wrapper

新增 shell wrapper `ys-proxy`，用法：

```bash
ys-proxy cl-kimi          # 启动 Claude Code，通过代理转发
ys-proxy cl-mini -- -p "hello"   # 透传参数给 claude
```

Wrapper 逻辑（shell function 生成于 `~/.cc-switch-tui/aliases.zsh`）：

```zsh
function ys-proxy {
  local alias_name=$1
  shift
  local proxy_url="http://localhost:7480/ys-proxy/${alias_name}"
  export ANTHROPIC_BASE_URL=$proxy_url
  # 调用对应的 cl-* alias
  ${alias_name} "$@"
}
```

**实际实现方式**：修改 `src/shell.rs` 中的 `format_function`，为每个 instance 生成两个 function：
- 原 `cl-kimi` — 直接调用 `claude`
- `ys-proxy` — 通用 wrapper，接受 alias 名，设置 `ANTHROPIC_BASE_URL` 后调用该 alias

### 2.2 HTTP API

#### Proxy 路由（非 REST，透传转发）

| Method | Path | 说明 |
|--------|------|------|
| ANY | `/ys-proxy/{alias}/**` | 代理入口，捕获所有子路径，转发到对应 Provider |

处理逻辑：
1. 提取 `alias`
2. 查 SQLite 得该 instance 的 `upstream_url` + `api_key`
3. 将 `**` 部分拼接到 `upstream_url` 后
4. 转发请求（保留 method、headers、body）
5. 响应返回客户端，同时记录 trace

#### Trace 查询 API

| Method | Path | 说明 |
|--------|------|------|
| GET | `/api/traces/sessions` | 列出所有 sessions（分页、按日期过滤） |
| GET | `/api/traces/sessions/:id` | 获取单个 session 元数据 |
| GET | `/api/traces/sessions/:id/records` | 获取 session 的所有 records |
| GET | `/api/traces/sessions/:id/records?limit=N&offset=M` | 分页获取 records |
| DELETE | `/api/traces/sessions/:id` | 删除 session 及其 records |
| GET | `/api/traces/sessions/:id/export/jsonl` | 导出为 JSONL |

### 2.3 Web 前端路由

在现有 React SPA 中新增：

| 路径 | 页面 |
|------|------|
| `/traces` | Trace Dashboard — session 列表 |
| `/traces/:id` | Trace Viewer — 单 session 详情 |

导航栏新增 "Traces" tab。

---

## 3. Project Structure

### 3.1 Rust 后端（新增/修改）

```
src/
├── proxy/
│   ├── mod.rs              # 路由注册: /ys-proxy/{alias}
│   ├── handler.rs          # 主 handler: 提取 alias、查配置、调用 upstream
│   ├── upstream.rs         # reqwest 客户端, 发送请求并返回 Response + Stream
│   ├── sse.rs              # SSE 流解析器: 边转发边提取 event（参考 claude-tap SSEReassembler）
│   ├── parser.rs           # Anthropic API JSON 解析 (messages, usage, tool_calls)
│   └── filter.rs           # Header 过滤/脱敏逻辑
├── trace/
│   ├── mod.rs
│   ├── models.rs           # TraceSession, TraceRecord, TraceEvent 结构体
│   └── store.rs            # SQLite DAO（独立 db 文件: .cc-switch-tui/traces.sqlite）
├── api/
│   ├── mod.rs              # 新增 traces 路由注册
│   └── traces.rs           # /api/traces/* handler
├── shell.rs                # 修改: generate ys-proxy wrapper function
└── main.rs                 # 修改: 初始化 TraceStore
```

### 3.2 前端（新增/修改）

```
web/src/
├── api/
│   └── traces.ts           # Trace API 客户端 (tanstack-query hooks)
├── pages/
│   └── traces/
│       ├── Dashboard.tsx   # session 列表页
│       ├── Viewer.tsx      # 单 session 详情页
│       ├── SessionCard.tsx # session 卡片组件
│       ├── RecordList.tsx  # record 列表组件
│       └── MessageView.tsx # 消息渲染组件
├── App.tsx                 # 修改: 新增 /traces /traces/:id 路由 + 导航
└── components/
    └── TraceNav.tsx        # 导航栏新增 Traces 链接
```

---

## 4. Code Style

### 4.1 Rust

- 沿用现有 cc-switch-tui 风格：
  - 错误处理：使用 `thiserror` + `AppError`
  - 异步：tokio + axum 标准模式
  - 数据库：rusqlite（bundled feature）
  - HTTP 客户端：新增 `reqwest`（stream feature）
- 模块组织：proxy 和 trace 作为独立模块，与 api 平行
- 命名：
  - 代理相关：`ProxyHandler`, `UpstreamClient`, `SseParser`
  - Trace 相关：`TraceStore`, `TraceSession`, `TraceRecord`

### 4.2 TypeScript / React

- 沿用现有前端风格：
  - Tailwind CSS + shadcn/ui 组件
  - tanstack-query 做数据获取
  - react-router-dom 做路由
- 组件命名：PascalCase，文件与组件同名
- API 类型：与 Rust 结构体对齐

### 4.3 参考规范

- SSE 解析逻辑直接参考 `claude-tap/claude_tap/sse.py` 中的 `SSEReassembler`
- SQLite schema 参考 `claude-tap/claude_tap/trace_store.py` v3 schema（简化版，暂不做 blob 拆分）
- Header 脱敏规则复制 claude-tap 的 `SENSITIVE_HEADER_KEYS`

---

## 5. Data Flow & Architecture

```
终端执行: ys-proxy cl-kimi
    │
    ▼
ANTHROPIC_BASE_URL=http://localhost:7480/ys-proxy/cl-kimi
    │
    ▼
Claude Code → POST /ys-proxy/cl-kimi/v1/messages
              Header: Authorization: Bearer <key from instance>
              Body: {messages, model, ...}
    │
    ▼
axum Router /ys-proxy/{alias}/**
    │
    ├──► 提取 alias = "cl-kimi"
    ├──► 查 instance config（upstream_url, api_key）
    ├──► 创建/复用 TraceSession
    │
    ├──► 向上游发送请求（reqwest::Client）
    │         │
    │         ▼
    │    上游 Provider 返回 SSE Stream
    │         │
    │         ▼
    ├──► StreamSplitter（tokio::sync::broadcast 或自定义 Stream）
    │         │
    │         ├──► 分支 1: 逐 chunk 写入 axum Response（给 Claude Code）
    │         │
    │         └──► 分支 2: SseParser 逐 event 解析
    │                    │
    │                    ├──► 提取 message_start / content_block_delta / message_delta
    │                    ├──► 累加 snapshot（类似 SSEReassembler）
    │                    └──► 每个完整 event 写入 TraceRecord
    │
    └──► 请求结束后 finalize session
              │
              ▼
    前端 GET /api/traces/sessions/{id}/records
              │
              ▼
    React Viewer 渲染对话流、token usage、diff
```

**关键技术决策：**

1. **Stream 分流**：使用 `tokio::sync::broadcast` 或 futures::Stream::map + 双 consumer，确保：
   - Claude Code 接收到的流不被缓冲（实时性）
   - Parser 在另一个 task 中解析，不阻塞转发

2. **Session 生命周期**：
   - `message_start` event 时创建 session
   - 每个 request/response 对产生一个 `TraceRecord`
   - 连接关闭或 `message_stop` 时 finalize session

3. **SQLite 写入**：使用单 writer task + channel，避免多线程竞争。WAL 模式已开启。

---

## 6. SQLite Schema

独立数据库文件：`.cc-switch-tui/traces.sqlite`

```sql
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE sessions (
    id TEXT PRIMARY KEY,              -- UUID
    started_at TEXT NOT NULL,         -- ISO 8601
    updated_at TEXT NOT NULL,
    date_key TEXT NOT NULL,           -- YYYY-MM-DD，用于按天分组
    alias TEXT NOT NULL,              -- 如 "cl-kimi"
    provider TEXT NOT NULL,           -- 如 "minimax"
    model TEXT NOT NULL,              -- 如 "MiniMax-M2.7"
    status TEXT NOT NULL DEFAULT 'active',  -- active | complete | error | empty
    record_count INTEGER NOT NULL DEFAULT 0,
    summary_json TEXT                 -- 缓存的 dashboard 摘要
);

CREATE TABLE records (
    session_id TEXT NOT NULL,
    record_index INTEGER NOT NULL,
    turn INTEGER,                     -- 对话轮次
    timestamp TEXT,                   -- ISO 8601
    direction TEXT NOT NULL,          -- 'request' | 'response'
    payload_json TEXT NOT NULL,       -- 完整 JSON payload
    PRIMARY KEY (session_id, record_index),
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE INDEX idx_sessions_updated_at ON sessions(updated_at);
CREATE INDEX idx_sessions_date_key ON sessions(date_key);
CREATE INDEX idx_records_session_id ON records(session_id);
```

**注**：v1 版本不做 claude-tap v4 的 blob 拆分（`record_blobs` 表），所有 payload 直接内联在 `payload_json` 中。当单条记录超过阈值（如 100KB）或性能成为问题时，再引入 blob 拆分。

---

## 7. API 详细设计

### 7.1 Proxy 路由

```rust
// src/proxy/mod.rs
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/ys-proxy/{alias}/*path", any(proxy_handler))
}
```

**路径处理**：
- 请求路径：`/ys-proxy/cl-kimi/v1/messages`
- 提取：`alias = "cl-kimi"`, `sub_path = "/v1/messages"`
- 上游 URL：`{instance.upstream_url}{sub_path}`
- 如 upstream_url = `https://api.minimax.io/anthropic`，则转发到 `https://api.minimax.io/anthropic/v1/messages`

**允许的路径前缀**（参考 claude-tap）：
```
/v1/messages
/v1/complete
/v1/models
```

其他路径返回 404，不转发。

**Header 处理**：
- 移除 hop-by-hop headers（connection, keep-alive, transfer-encoding 等）
- 脱敏敏感 header（authorization 值替换为 `***` 后存入 trace）
- 注入 `Authorization: Bearer {instance.api_key}` 到上游请求

### 7.2 Trace API

```
GET /api/traces/sessions
  Query: ?date=2026-06-10&limit=20&offset=0
  Response: {
    "sessions": [
      {
        "id": "uuid",
        "started_at": "2026-06-10T12:00:00Z",
        "updated_at": "...",
        "date_key": "2026-06-10",
        "alias": "cl-kimi",
        "provider": "minimax",
        "model": "MiniMax-M2.7",
        "status": "complete",
        "record_count": 5,
        "summary": {
          "total_tokens": 1234,
          "prompt_tokens": 1000,
          "completion_tokens": 234,
          "duration_ms": 5000
        }
      }
    ],
    "total": 100
  }

GET /api/traces/sessions/:id/records
  Response: {
    "records": [
      {
        "record_index": 1,
        "turn": 1,
        "timestamp": "...",
        "direction": "request",
        "payload": { /* 原始 JSON */ }
      },
      {
        "record_index": 2,
        "turn": 1,
        "timestamp": "...",
        "direction": "response",
        "payload": { /* 累加后的 snapshot */ }
      }
    ]
  }
```

---

## 8. Frontend Design

### 8.1 Dashboard (`/traces`)

布局参考 claude-tap dashboard.html：
- 左侧：日期筛选栏（今天、昨天、更早）
- 右侧：session 卡片列表
- 每个卡片显示：alias、model、时间、record_count、token 用量、状态
- 点击卡片进入 Viewer

### 8.2 Viewer (`/traces/:id`)

布局参考 claude-tap viewer.html：
- 顶部：session 元数据（alias、model、时间、token 统计）
- 中部：Record 列表（request/response 成对展示）
- 每个 record 可展开查看：
  - Request：messages、system prompt、tools、model
  - Response：assistant message、tool_calls、usage
- 相邻 request 之间的 diff（高亮变化的 message）

### 8.3 组件清单

| 组件 | 来源 | 说明 |
|------|------|------|
| `SessionCard` | 新写 | 列表中的 session 卡片 |
| `RecordList` | 新写 | record 时间线 |
| `MessageView` | 新写 | 渲染 Anthropic message 结构 |
| `DiffView` | 新写 | 两个 request 的 diff |
| `TokenBadge` | 新写 | token 用量小徽章 |
| `StatusBadge` | 新写 | active/complete/error 状态 |

---

## 9. Testing Strategy

### 9.1 单元测试（Rust）

- `proxy::sse::SseParser`：
  - 测试 SSE 字节流解析（单 event、多 event、跨 chunk 边界）
  - 测试 snapshot 累加（message_start → content_block_delta → message_delta）
- `proxy::filter`：
  - 测试 header 脱敏规则
- `trace::store`：
  - 测试 session CRUD、record 追加、分页查询

### 9.2 集成测试（Rust）

- 启动 axum test server，模拟上游 API（用 wiremock 或 mockito）
- 发送 `/ys-proxy/test-alias/v1/messages` 请求
- 验证：
  - 下游收到正确 SSE 流
  - SQLite 中记录正确数量的 records
  - payload 结构符合预期

### 9.3 前端测试（Vitest）

- `MessageView`：正确渲染 text/thinking/tool_use block
- `SessionCard`：正确显示元数据
- API hooks：mock fetch，验证请求路径

### 9.4 端到端测试（手动）

```bash
# 1. 启动 server
cargo run

# 2. 配置一个 test instance（如指向 mock server）
# 3. 执行 ys-proxy test-alias -- -p "hello"
# 4. 浏览器打开 http://localhost:7480/traces
# 5. 验证 session 出现、records 正确、message 可阅读
```

---

## 10. Boundaries

### 10.1 明确支持（MVP）

- 仅支持 Claude Code（Anthropic Messages API 格式）
- 仅处理 SSE 流式响应（`accept: text/event-stream`）
- 仅支持通过 `ys-proxy` wrapper 启动的会话
- 仅支持 cc-switch-tui 已配置的 Provider（MiniMax、Kimi 等）

### 10.2 暂不支持（后续扩展）

- OpenAI API 格式（Codex CLI 等）
- AWS Bedrock EventStream 格式
- WebSocket 代理
- 非流式请求的记录（可记录但前端不特殊处理）
- 多用户并发（SQLite 单 writer 足够）
- Trace 导出 HTML（v2 再做，先支持 JSONL）

### 10.3 必须遵守的安全约束

- `Authorization` header 的值**绝不**以明文存入 SQLite（存储前脱敏为 `***` 或前 12 位 + `...`）
- `api_key` 只在内存中使用，不写入 trace payload
- 路径白名单：只允许 `/v1/messages` 等已知 API 路径，拒绝 `/etc/passwd` 等扫描请求

### 10.4 开发纪律

- 不修改现有 Instance 管理、Alias 生成、Opencode 等无关代码
- 新增代码与现有代码风格一致
- 前端不引入新的 UI 库（只用现有 shadcn/ui + Tailwind）

---

## 11. Implementation Order（垂直切片）

### Slice 1: 非流式请求走通（端到端骨架）

**目标**：验证 alias wrapper → proxy → 上游 → 记录 → 前端展示 的完整链路。

1. 修改 `shell.rs`：生成 `ys-proxy` wrapper function
2. 实现 `proxy::mod.rs` + `proxy::handler.rs`：基础反向代理（非流式）
3. 实现 `trace::models.rs` + `trace::store.rs`：基础 schema + session/record CRUD
4. 实现 `api::traces.rs`：基础查询 API
5. 前端 `Dashboard.tsx`：简单 session 列表（仅显示 alias + 时间）

**验证**：
```bash
ys-proxy cl-test -- -p "hello"
# 浏览器打开 /traces，能看到一条 session
```

### Slice 2: SSE 流式代理

**目标**：实现 SSE 边转发边解析。

1. 实现 `proxy::sse.rs`：`SseParser`（参考 claude-tap）
2. 实现 `proxy::upstream.rs`：Stream 分流（broadcast channel）
3. 修改 `proxy::handler.rs`：流式响应处理
4. 前端 `Viewer.tsx`：基础 record 列表展示

**验证**：
```bash
ys-proxy cl-kimi -- -p "写一个快速排序"
# 能看到实时 streaming 的 response，且前端能查看每条 message
```

### Slice 3: 深解析 + Trace Viewer

**目标**：解析 messages、tool calls、usage，做 diff 和统计。

1. 实现 `proxy::parser.rs`：Anthropic payload 解析
2. 修改 `trace::store.rs`：summary 自动更新
3. 前端 `MessageView`、`DiffView`、`TokenBadge`

### Slice 4: Dashboard 完善 + 导出

**目标**：session 列表优化、按天筛选、JSONL 导出。

1. 前端 `SessionCard` 完善、日期筛选
2. 实现 `DELETE /api/traces/sessions/:id`
3. 实现 `GET /api/traces/sessions/:id/export/jsonl`

---

## 12. Dependencies

### Rust（新增）

```toml
[dependencies]
reqwest = { version = "0.12", features = ["json", "stream"] }
futures = "0.3"
bytes = "1"
uuid = { version = "1", features = ["v4"] }
```

### 前端（现有）

无需新增依赖，沿用：
- `@tanstack/react-query`
- `react-router-dom`
- `tailwindcss`
- `lucide-react`

如需 diff 展示，可引入 `diff-match-patch`（轻量，可选）。

---

## 13. Risks & Mitigations

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| SSE Stream 分流实现复杂 | 高 | 先用最简单的 `tokio::sync::broadcast`，不行再换 `flume` 或手动 Stream impl |
| 上游 API 格式变化 | 中 | parser 模块隔离，只解析已知字段，未知字段透传 |
| SQLite 写入成为瓶颈 | 低 | WAL 模式 + 单 writer task + 批量写入（非每个 event 都写，可缓存后 batch）|
| 前端 mock 与真实数据不一致 | 中 | Slice 1 就要求端到端走通，不用纯 mock |

---

*文档版本: v1.0*
*日期: 2026-06-10*
*分支: feat/ys-proxy*
