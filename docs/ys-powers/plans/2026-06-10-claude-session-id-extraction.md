# Claude Session ID 提取与 PII 脱敏 — 实施计划

## 依赖图

```
session_extractor.rs (Task 2)
        │
        ▼
proxy/handler.rs (Task 3) ◄──── trace/store.rs (Task 1)
        │                            │
        │                            ▼
        │                     api/traces.rs (Task 4)
        │                            │
        │                            ▼
        │                     web/src/api/traces.ts (Task 4)
        │
        ▼
   端到端验证 (Task 5)
```

**依赖关系：**
- Task 1（数据库层）和 Task 2（提取器）**互相独立**，可并行
- Task 3（Handler 集成）依赖 Task 1 + Task 2
- Task 4（API + 前端）依赖 Task 1
- Task 5（验证）依赖 Task 3 + Task 4

---

## 任务列表（垂直切片）

### Task 1: 数据库层 + 模型层

**目标：** 让 `records` 表和 `TraceRecord` 结构体支持 `claude_session_id`。

**文件变更：**
- `src/trace/models.rs`: `TraceRecord` 新增 `claude_session_id: Option<String>`
- `src/trace/store.rs`:
  - `init_schema`: records 表新增 `claude_session_id TEXT`
  - `append_record`: 新增 `claude_session_id: Option<&str>` 参数
  - `row_to_record`: 读取第 7 列
  - 现有测试更新（`append_record` 调用签名变更）
  - 新增测试 `test_append_record_with_claude_session_id`

**验收标准：**
- `cargo test trace::store::tests` 全部通过
- `cargo build` 无警告

**验证步骤：**
```bash
cargo test trace::store::tests -- --nocapture
```

---

### Task 2: Session Extractor（提取 + 脱敏）

**目标：** 独立模块负责 `claude_session_id` 提取和 PII 脱敏，带完整单元测试。

**文件变更：**
- `src/proxy/session_extractor.rs` (NEW):
  - `extract_claude_session_id(body: &Value) -> Option<String>`
  - `redact_user_id_pii(body: &mut Value) -> bool`
  - `MAX_CLAUDE_SESSION_ID_LEN = 128`
- `src/proxy/mod.rs`: 暴露 `session_extractor` 模块

**测试覆盖（12 个）：**
- 提取测试 8 个（见 spec §5.1）
- 脱敏测试 4 个（见 spec §5.2）

**验收标准：**
- 12 个单元测试全部通过
- 提取失败时返回 `None`，不脱敏失败时返回 `false`，均不 panic

**验证步骤：**
```bash
cargo test proxy::session_extractor::tests -- --nocapture
```

---

### Task 3: Handler 集成

**目标：** 在流式请求转发前提取 `claude_session_id`、脱敏 body、写入 trace。

**文件变更：**
- `src/proxy/handler.rs`:
  - 请求转发前，解析 `body_str` 为 `serde_json::Value`
  - 条件：`method == POST && path.starts_with("/v1/messages")`
  - 调用 `extract_claude_session_id` 获取 `claude_session_id`
  - 调用 `redact_user_id_pii` 脱敏 body
  - 脱敏后的 JSON 作为 `body_str_clone` 传入后台 trace 任务
  - 后台任务中 `append_record` 传入 `claude_session_id`

**关键点：**
- 脱敏在 `body_str_clone` 创建前完成，确保存入 DB 的是脱敏版本
- 提取/脱敏失败不影响转发（静默回退）

**验收标准：**
- `cargo build` 通过
- handler 测试（streaming detection）仍通过

**验证步骤：**
```bash
cargo test proxy::handler::tests -- --nocapture
cargo build
```

---

### Task 4: API + 前端类型同步

**目标：** 后端返回新字段，前端类型声明对齐。

**文件变更：**
- `src/api/traces.rs`: `GetRecordsResponse` / 相关 handler 无需修改（`TraceRecord` 已实现 Serialize）
- `web/src/api/traces.ts`: `TraceRecord` 接口新增 `claude_session_id?: string`

**验收标准：**
- `cargo build` 通过
- `cd web && npm run build` 通过（TypeScript 类型检查无错误）

**验证步骤：**
```bash
cargo build
cd web && npm run build
```

---

### Task 5: 端到端验证

**目标：** 真实请求链路验证提取和脱敏工作正常。

**验证步骤：**
1. 启动 server: `cargo run`
2. 清空旧 trace: 访问 `http://127.0.0.1:7481/traces` → Clear All
3. 用 `ys-proxy cl-mini` 发一条消息
4. 检查 DB:
   ```bash
   sqlite3 .cc-switch-tui/traces.sqlite "SELECT claude_session_id, payload_json FROM records WHERE direction='request' LIMIT 1;"
   ```
5. 确认：
   - `claude_session_id` 有值（UUID 格式）
   - `payload_json` 中的 `metadata.user_id` 内 `device_id` 和 `account_uuid` 已被替换为 `"***"`
   - `session_id` 保留原值

**验收标准：**
- 5 步全部通过

---

## 阶段检查点

| 检查点 | 包含任务 | 完成标准 |
|--------|----------|----------|
| **CP1** | Task 1 + Task 2 | 数据库层就绪 + 提取器带 12 个测试全部通过 |
| **CP2** | Task 3 | Handler 集成完成，核心链路打通，build 通过 |
| **CP3** | Task 4 + Task 5 | API 类型同步 + 端到端验证通过 |

---

## 回滚策略

- `records` 表新增列是 `ALTER TABLE ADD COLUMN`，SQLite 支持轻量添加，不破坏现有数据
- 如果中途需要回滚：删除 `claude_session_id` 列（SQLite 需重建表，可用 `.schema` 备份后恢复）
- 更安全的做法：Task 1 中先用 `IF NOT EXISTS` 添加列，避免重复执行报错
