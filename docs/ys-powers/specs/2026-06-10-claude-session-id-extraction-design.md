# Claude Session ID 提取与 PII 脱敏设计文档

## 1. Objective

在 ys-proxy 转发 `POST /v1/messages` 请求时，从请求体的 `metadata.user_id` 中提取 `session_id`（即 Claude Code 的真实会话 ID），并在存储前对 PII 字段脱敏。

**核心价值：**
- 让 Viewer 能按真实 Claude Code 会话分组，而非按 HTTP 请求维度
- 支持后台按 `claude_session_id` 做 token 消耗分析
- 脱敏 `device_id` / `account_uuid`，保护用户隐私

**范围（本次不做）：**
- project / cwd 信息提取（需 Registration API，另起 spec）
- 非 `POST /v1/messages` 路径的提取
- 非流式请求的 trace 记录（当前 MVP 只支持流式）

---

## 2. Commands / Interfaces

### 2.1 数据模型变更

`records` 表新增列：

```sql
ALTER TABLE records ADD COLUMN claude_session_id TEXT;
```

`TraceRecord` 结构体新增字段：

```rust
pub struct TraceRecord {
    pub session_id: String,
    pub record_index: i64,
    pub turn: Option<i64>,
    pub timestamp: Option<String>,
    pub direction: String,
    pub payload_json: String,
    pub claude_session_id: Option<String>,  // NEW
}
```

### 2.2 提取条件

仅当同时满足以下条件时提取：

| 条件 | 值 |
|------|-----|
| HTTP method | `POST` |
| Path prefix | `/v1/messages` |

### 2.3 PII 脱敏规则

存储到 DB 前，`payload_json` 中的 `metadata.user_id` 必须脱敏：

| 字段 | 行为 | 原因 |
|------|------|------|
| `device_id` | 替换为 `"***"` | 设备指纹，PII |
| `account_uuid` | 替换为 `"***"` | 账户标识，PII |
| `session_id` | **保留原值** | 分组键，无 PII 风险 |

脱敏失败时回退到原始 body（不阻塞存储）。

---

## 3. Project Structure

### 新增/修改文件

```
src/
├── proxy/
│   ├── session_extractor.rs    # NEW: extract_claude_session_id + redact_user_id_pii
│   ├── handler.rs              # MOD: 调用 extractor，写入 claude_session_id
│   └── mod.rs                  # MOD: 暴露 session_extractor 模块
├── trace/
│   ├── models.rs               # MOD: TraceRecord 新增 claude_session_id
│   └── store.rs                # MOD: schema + append_record 新增参数
├── api/
│   └── traces.rs               # MOD: 返回新字段到前端
└── web/src/api/traces.ts       # MOD: TraceRecord 接口新增字段
```

---

## 4. Code Style

- 提取逻辑完全独立为 `session_extractor.rs`，不耦合在 handler 中
- 使用 `serde_json::Value` 路径提取，而非字符串扫描（可靠，不受 truncate 影响）
- 所有解析失败静默返回 `None`，不 panic、不阻断请求转发
- 脱敏函数接收 `&mut serde_json::Value`，就地修改

---

## 5. Testing Strategy

### 5.1 单元测试：`session_extractor.rs`

8 个测试覆盖正常和异常路径（与 claude-tap 对齐）：

| 测试 | 覆盖点 |
|------|--------|
| `test_extract_success` | 正常 JSON 提取 session_id |
| `test_extract_missing_metadata` | 无 metadata 字段 |
| `test_extract_missing_user_id` | metadata 无 user_id |
| `test_extract_user_id_not_string` | user_id 是数字/对象/null |
| `test_extract_malformed_json` | user_id 字符串不是合法 JSON |
| `test_extract_no_session_id` | JSON 内无 session_id |
| `test_extract_session_id_too_long` | session_id > 128 chars |
| `test_extract_old_format_flat_slug` | 老版本 user_id 是纯字符串（非 JSON） |

### 5.2 单元测试：`redact_user_id_pii`

4 个测试：

| 测试 | 覆盖点 |
|------|--------|
| `test_redact_strips_device_and_account` | 正常脱敏，保留 session_id |
| `test_redact_no_user_id` | 无 user_id 时 body 不变 |
| `test_redact_flat_slug` | 老格式不处理 |
| `test_redact_partial_fields` | 部分字段缺失 |

### 5.3 集成测试

在 `store.rs` 测试中验证：
- `append_record` 能正确写入含 `claude_session_id` 的记录
- `get_records` 能正确读出该字段

---

## 6. Boundaries

### 必须做的
- `claude_session_id` 提取（`metadata.user_id.session_id`）
- PII 脱敏（device_id / account_uuid → `***`）
- `records` 表 schema 迁移（新增列）
- 前端 `TraceRecord` 类型同步更新

### 需要问清楚再做
- project / cwd 提取（需改 `ys-proxy` shell 脚本 + Registration API）
- 从 `system` prompt 推断 project（准确度不确定，需评估）

### 永远不做的
- 不解密、不反序列化 user_id 以外的任何加密字段
- 不把 `device_id` / `account_uuid` 的原始值存入任何表或日志
- 不在脱敏前把原始 body 写入 DB（脱敏必须在 append_record 前完成）
