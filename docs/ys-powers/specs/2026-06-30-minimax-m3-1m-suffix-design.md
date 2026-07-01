# Spec: MiniMax-M3 model id 改 `[1m]` 后缀 + 废弃 context_window 自动注入

> 上游 intent：[docs/ys-powers/intent/minimax-m3-1m-suffix.md](../intent/minimax-m3-1m-suffix.md)
> ADR：[docs/adr/0001-minimax-m3-1m-suffix-overrides-claude-md-rule.md](../../adr/0001-minimax-m3-1m-suffix-overrides-claude-md-rule.md)
> 工作分支：`feat/minimax-m3-1m-suffix`

## Objective

**做什么**：
1. `MiniMax-M3` model 的 id / opencode_model_id / 4 个 `ANTHROPIC_DEFAULT_*_MODEL` env_overrides 全部改为 `MiniMax-M3[1m]`
2. `MiniMax-M3[1m]` model 的 `env_overrides` 新增 `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`（**唯一新增** env，不加 `DISABLE_COMPACT` / `CLAUDE_CODE_MAX_CONTEXT_TOKENS`——官方文档未推荐）
3. 彻底删除 `instance.context_window_enabled: bool` 字段、`ModelTemplate.context_window: Option<usize>` 字段、`src/shell.rs::build_env` 的 auto-inject block（约 13 行）、`set_context_window_enabled` DAO 方法
4. SQLite schema migration：启动时一次性（幂等）执行两条 SQL——`UPDATE instances SET model_id='MiniMax-M3[1m]' WHERE model_id='MiniMax-M3'` + `ALTER TABLE instances DROP COLUMN context_window_enabled`
5. 更新 `CLAUDE.md` 中"不要依赖 `[1m]` 后缀"硬约束（按场景区分）
6. 同步 `src/opencode_config.rs` / `src/api/templates.rs` 中所有 `MiniMax-M3` 字符串引用

**为什么**：MiniMax 官方更新 Claude Code 集成文档，把推荐 model id 改为 `MiniMax-M3[1m]`，并显式推荐 `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`。同时用户原话确认：「之前的逻辑废弃掉，instance.context_window_enabled 和相关的功能全部废弃，后续根据模型 id 来自动化配置」。

**目标用户**：在 terminal 用 `cl-mini` 等 cl-* alias 调用 Claude Code 的开发者。

**成功的样子**：

| 验收项 | 期望 |
|---|---|
| `cl-mini --version` 能跑通 | claude 用正确的 `MiniMax-M3[1m]` model id + `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000` 连到 provider |
| 调用结束后父 shell 干净 | subshell 隔离仍生效（不依赖 `context_window_enabled`） |
| 旧 DB 中 `model_id="MiniMax-M3"` 行 | 启动后自动 rename 为 `MiniMax-M3[1m]`，实例继续工作 |
| SQLite schema | `context_window_enabled` 列已被 DROP |
| 重复启动 | migration 幂等，第二次启动无变化（不变更 schema/数据） |
| kimi template / M2.7-highspeed | 不动 |
| CLAUDE.md | "不要依赖 `[1m]` 后缀" 改写为按场景区分 |

## Tech Stack

- Rust 2024 edition（项目既定）
- rusqlite 0.x bundled（自带 SQLite ≥ 3.35，支持 `ALTER TABLE ... DROP COLUMN`）
- 不引入新依赖

## Commands

```bash
# 构建
make build

# 测试
cargo test --lib                        # 全 lib 测试
cargo test --lib shell::tests           # shell 模块
cargo test --lib dao::tests             # DAO（含 migration）
make test                               # cargo + npm 全套

# Lint / Format
make fmt                                # cargo fmt
make lint                               # cargo clippy + eslint
cargo fmt --check                       # 仅检查

# 手动端到端验证
cargo run --release
# 然后：cat ~/.cc-switch-tui/aliases.zsh | grep -E "MiniMax-M3|CLAUDE_CODE_AUTO_COMPACT_WINDOW"
sqlite3 ~/.cc-switch-tui/db.sqlite ".schema instances"
sqlite3 ~/.cc-switch-tui/db.sqlite "SELECT model_id FROM instances"
```

## Project Structure

本次改动触及的文件（实测 grep 量化）：

```
# 后端 Rust（73 处 context_window 引用 + 43 处 MiniMax-M3 引用）
src/templates.rs                              # MiniMax-M3 → MiniMax-M3[1m]，新增 AUTO_COMPACT_WINDOW env
src/opencode_config.rs                        # opencode_models vec 同步 + MiniMax-M3 字面量
src/domain/instance.rs                        # 删除 context_window_enabled 字段
src/domain/template.rs                        # 删除 ModelTemplate.context_window: Option<u64> 字段
src/shell.rs::build_env                       # 删除 auto-inject block（约 13 行）
src/shell.rs::tests                           # 19 处 fixture/instance 字面量同步
src/dao/sqlite_impl.rs                        # migration SQL + SELECT/INSERT/UPDATE 不再读写 context_window_enabled
src/dao/memory_impl.rs                        # 删除 context_window_enabled 字段写入
src/dao/mod.rs                                # 删除 Dao trait 的 set_context_window_enabled 方法签名
src/api/instances.rs                          # 4 个 DTO struct（CreateInstanceRequest / PatchInstanceRequest / InstanceSummary / InstanceDetail）删 context_window_enabled
src/api/templates.rs                          # 删除 ModelTemplate DTO 的 context_window: Option<u64> 字段
src/api/config.rs                             # line 95 构造 ProviderInstance 时删字段
src/api/instances.rs handlers                 # 创建/更新 handler 不再处理 context_window_enabled 参数

# 前端（26 处 contextWindow 引用，6 个文件）—— **见 ASK-1**
web/src/api/types.ts                          # 删除 Instance.contextWindowEnabled 字段；ModelTemplate.contextWindow 字段处理
web/src/api/hooks.ts                          # 4 处引用
web/src/components/InstanceForm.tsx           # 9 处引用（表单字段 + model context_window 显示）
web/src/components/InstancesTable.tsx         # 1 处引用（badge）
web/src/components/ModelSelect.tsx            # model context_window 显示（与 M3[1m] 自动配置兼容）
web/src/routes/InstanceDetailPage.tsx         # 9 处引用
web/src/lib/validate.ts                       # Zod schema 删除 contextWindowEnabled

# 测试
tests/dao_test.rs                             # 2+ 处 instance/model 字面量
tests/fixture calls across tests/             # 删 context_window_enabled / context_window 字面量

# 文档
CONTEXT.md                                     # 不变（无新术语）
docs/codebase/ARCHITECTURE.md                  # 检查是否需要同步（shell integration 层 / Provider 数据模型）
CLAUDE.md                                      # 改写"[1m] 后缀"硬约束描述

新增：
docs/ys-powers/intent/minimax-m3-1m-suffix.md
docs/ys-powers/specs/2026-06-30-minimax-m3-1m-suffix-design.md
docs/adr/0001-minimax-m3-1m-suffix-overrides-claude-md-rule.md
```

不动：
- `src/proxy/*`（proxy 与 provider 配置无关）
- `src/shell.rs::format_function`（subshell 隔离已就绪）
- `src/port.rs` / `src/main.rs`（与本次意图无关）
- kimi 模板 / oc-* / ys-proxy wrapper / sentinel / `__cc_switch_print_env`

## Code Style

### 1. `templates.rs::minimax_template()` 目标形态

```rust
fn minimax_template() -> ProviderTemplate {
    let mut default_env = HashMap::new();
    default_env.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        "https://api.minimaxi.com/anthropic".to_string(),
    );
    default_env.insert("API_TIMEOUT_MS".to_string(), "3000000".to_string());
    default_env.insert(
        "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
        "1".to_string(),
    );

    let mut env_overrides_m3 = HashMap::new();
    // 跟随官方文档：MiniMax-M3[1m] + CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000
    // 不再依赖 instance.context_window_enabled toggle；后续 model 改默认值
    // 直接改这里即可（model id 决定 env 配置）。
    env_overrides_m3.insert("ANTHROPIC_MODEL".to_string(), "MiniMax-M3[1m]".to_string());
    env_overrides_m3.insert("ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(), "MiniMax-M3[1m]".to_string());
    env_overrides_m3.insert("ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(), "MiniMax-M3[1m]".to_string());
    env_overrides_m3.insert("ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(), "MiniMax-M3[1m]".to_string());
    env_overrides_m3.insert("CLAUDE_CODE_AUTO_COMPACT_WINDOW".to_string(), "1000000".to_string());

    let mut env_overrides_m27 = HashMap::new();
    env_overrides_m27.insert("ANTHROPIC_MODEL".to_string(), "MiniMax-M2.7-highspeed".to_string());
    // ... 同现状不变

    ProviderTemplate {
        id: "minimax".to_string(),
        name: "MiniMax".to_string(),
        default_env,
        models: vec![
            ModelTemplate {
                id: "MiniMax-M3[1m]".to_string(),
                name: "MiniMax M3 [1m]".to_string(),
                env_overrides: env_overrides_m3,
                opencode_model_id: "MiniMax-M3[1m]".to_string(),
                // context_window 字段已删除
            },
            ModelTemplate {
                id: "MiniMax-M2.7-highspeed".to_string(),
                name: "MiniMax M2.7 Highspeed".to_string(),
                env_overrides: env_overrides_m27,
                opencode_model_id: "MiniMax-M2.7-highspeed".to_string(),
            },
        ],
        opencode_provider_id: "minimax-cn".to_string(),
        opencode_npm: "@ai-sdk/anthropic".to_string(),
        opencode_base_url: "https://api.minimaxi.com/anthropic/v1".to_string(),
        opencode_env_var: "MINIMAX_API_KEY".to_string(),
        opencode_models: vec![
            "MiniMax-M2.7-highspeed".to_string(),
            "MiniMax-M3[1m]".to_string(),  // 同步
        ],
    }
}
```

### 2. `src/shell.rs::build_env` 删除 auto-inject block

**删除**：

```rust
// 整段删除（约 13 行）
if let Some(model) = template.models.iter().find(|m| m.id == instance.model_id) {
    if let Some(window) = model.context_window {
        env.insert("DISABLE_COMPACT".to_string(), "1".to_string());
        env.insert("CLAUDE_CODE_MAX_CONTEXT_TOKENS".to_string(), window.to_string());
        env.insert("CLAUDE_CODE_AUTO_COMPACT_WINDOW".to_string(), window.to_string());
    }
}
```

**理由**：env 完全由 `default_env` + `env_overrides` 提供，model id 决定 env 内容。

### 3. SQLite migration（`src/dao/sqlite_impl.rs::new`）

在现有 schema setup 后追加：

```rust
// 一次性迁移（幂等）：

// Step 1: 旧 model_id 行 rename 到新 id
let _ = conn.execute(
    "UPDATE instances SET model_id = 'MiniMax-M3[1m]' WHERE model_id = 'MiniMax-M3'",
    [],
);

// Step 2: DROP COLUMN context_window_enabled（SQLite ≥ 3.35 支持）
// 通过 pragma_table_info 判断列是否存在再 DROP，保证幂等
let columns: Vec<String> = ...;  // 已有查询逻辑
if columns.contains(&"context_window_enabled".to_string()) {
    let _ = conn.execute(
        "ALTER TABLE instances DROP COLUMN context_window_enabled",
        [],
    );
}
```

**幂等性**：
- UPDATE 第二次匹配 0 行——no-op
- DROP COLUMN 在列已不存在时 `let _ = ...` 吞掉错误

### 4. 领域模型删除字段

```rust
// ProviderInstance 删除字段
pub struct ProviderInstance {
    pub id: String,
    pub template_id: String,
    pub model_id: String,
    pub api_key: String,
    pub created_at: DateTime<Utc>,
    pub alias: String,
    pub opencode_model_id: String,
    pub kv_cache_enabled: bool,
    // 删除：pub context_window_enabled: bool,
}

// ModelTemplate 删除字段
pub struct ModelTemplate {
    pub id: String,
    pub name: String,
    pub env_overrides: HashMap<String, String>,
    pub opencode_model_id: String,
    // 删除：pub context_window: Option<u64>,
}
```

### 5. DAO trait 删除方法

`src/dao/mod.rs` 的 `Dao` trait 包含：

```rust
fn set_context_window_enabled(&mut self, id: &str, enabled: bool) -> Result<(), AppError>;
```

整条声明从 trait 中删除。两个 impl（`sqlite_impl.rs` / `memory_impl.rs`）的实现同步删除。

### 6. 前端 web/ 改写范围（确认纳入本次 intent）

```typescript
// web/src/api/types.ts
interface Instance {
    // 删除 contextWindowEnabled: boolean;
    // 其他字段保留
}

interface ModelTemplate {
    // 删除 contextWindow?: number; —— 由 ModelSelect 硬编码从 model id 推断
}

// web/src/lib/validate.ts — Zod schema
// 删除 contextWindowEnabled: z.boolean().default(false),

// web/src/components/InstanceForm.tsx
// 删除 form state 字段、checkbox UI、model contextWindow 显示逻辑

// web/src/components/InstancesTable.tsx
// 删除 {i.contextWindowEnabled && (...)} 条件渲染

// web/src/components/ModelSelect.tsx — 硬编码从 model id 推断
// 替换：{m.contextWindow ? ` · ${formatTokens(m.contextWindow)} context` : ''}
// 为：
function inferContextFromModelId(modelId: string): string | null {
    if (modelId.includes('[1m]')) return '1M context';
    if (modelId.includes('[200k]')) return '200K context';
    // 后续若其他 model 加新后缀，按此模式扩展
    return null;
}
// 显示：{inferContextFromModelId(m.id) ? ` · ${inferContextFromModelId(m.id)}` : ''}

// web/src/routes/InstanceDetailPage.tsx
// 删除 form draft 字段、checkbox UI
```

**后端 + 前端同步改动总览**：

| 层 | 文件数 | 引用数 |
|---|---|---|
| Rust 后端 src/ | 10 个 | ~47 处 |
| Rust 后端 tests/ | 多个 fixture | ~26 处 |
| TS 前端 web/src/ | 6 个 | 26 处 |
| 文档 | CLAUDE.md + ARCHITECTURE.md | 2 段 |
| **合计** | **~18 文件** | **~101 处** |

### 7. CLAUDE.md 改写片段

**当前**（硬约束）：
> 不要依赖 `model` 字段的 `[1m]` 后缀（VS Code 扩展会重置）；用 env 变量 `ANTHROPIC_BASE_URL` / `ANTHROPIC_AUTH_TOKEN` / `ANTHROPIC_MODEL` 组合覆盖。

**改写为**（按场景）：
> `model` 字段 `[1m]` 后缀**仅在 Claude Code 终端场景下可靠**：cc-switch-tui 通过 `ANTHROPIC_MODEL` env 变量在 cl-* 函数体内注入，shell 进程不被重置。**VS Code 扩展场景下 env var 可能被 extension 重置**，需自行验证。当前 follow 官方文档使用 `MiniMax-M3[1m]`（含后缀）。

并删除原"DISABLE_COMPACT=1 + CLAUDE_CODE_MAX_CONTEXT_TOKENS 配合机制"段落（机制随字段废弃）。

## Testing Strategy

测试框架：Rust 内置 `#[test]` + `#[cfg(test)]` 模块（沿用项目现有模式）。

### 1. 单元测试（`src/dao/sqlite_impl.rs::tests`）

**新增**：

```rust
#[test]
fn test_migration_renames_old_minimax_m3_model_id() {
    // 准备：旧 DB schema（无 context_window_enabled 列）+ 含 model_id="MiniMax-M3" 的行
    // 执行：SqliteDaoImpl::new
    // 断言：load_instances 返回的 row.model_id == "MiniMax-M3[1m]"
}

#[test]
fn test_migration_drops_context_window_enabled_column() {
    // 准备：旧 DB 含 context_window_enabled 列
    // 执行：SqliteDaoImpl::new
    // 断言：pragma_table_info('instances') 不含 context_window_enabled
}

#[test]
fn test_migration_is_idempotent() {
    // 准备：旧 DB
    // 执行：SqliteDaoImpl::new 两次
    // 断言：第二次执行不报错；行数不变；列不变
}
```

**改写**：`test_context_window_column_migration` 与 `test_create_instance_with_context_window_enabled`——删除（前者因字段已 drop，后者因 `context_window_enabled: true` 字段不存在）。

### 2. 单元测试（`src/shell.rs::tests`）

**新增**：

```rust
#[test]
fn test_aliases_contain_minimax_m3_1m_model_id() {
    // 用 fixture 生成 aliases.zsh
    // 断言：content.contains("ANTHROPIC_MODEL=\"MiniMax-M3[1m]\"") 或类似
}

#[test]
fn test_aliases_contain_auto_compact_window_var() {
    // 断言：content.contains("CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000")
}

#[test]
fn test_aliases_do_not_contain_disabled_compact_var() {
    // 断言：!content.contains("DISABLE_COMPACT")（已废弃）
}
```

**改写**：`fixture("minimax", "MiniMax-M3[1m]", ...)` —— 改 model_id 字符串。
`fixture("minimax", "MiniMax-M2.7-highspeed", ...)` —— 不动。

### 3. 集成测试（`tests/aliases_test.rs` 等）

如含 `MiniMax-M3` 字面量引用，更新为 `MiniMax-M3[1m]`。

### 4. 测试覆盖率期望

- DAO migration 测试：3 个新测试覆盖（rename / drop / idempotent）
- shell module 测试：3 个新断言 + 1 个 fixture 字符串更新
- 所有 instance/model 构造点同步更新（约 30+ 处需要扫）
- 全 `cargo test --lib` 必须 100% pass
- `cargo clippy -D warnings` 0 warning
- `cargo fmt --check` 0 diff

## Boundaries

### Always do

- 改 `templates.rs` 后跑 `cargo test --lib shell::tests`
- 改 DAO migration 后跑 `cargo test --lib dao::tests`
- 提交前跑 `make fmt` 与 `make lint`
- migration SQL 写为幂等（pragma_table_info 判断列是否存在；UPDATE WHERE 不报错）
- 函数构造点更新必须批量扫（grep `context_window_enabled` / `context_window:` 应只剩 0 命中）
- 注释保持中文（与现有 `templates.rs` 一致）

### Ask first

- 加任何新 crate 依赖（本任务不需要）
- 改 kimi 模板 / oc-* / ys-proxy / sentinel 任一项
- 改 SQLite schema 的其他部分（不只是 drop context_window_enabled）
- 改 `~/.claude.json` onboarding（out of scope）
- 添加国际 endpoint `https://api.minimax.io/anthropic`（独立 intent）
- 添加 `DISABLE_COMPACT` 或 `CLAUDE_CODE_MAX_CONTEXT_TOKENS` 到 M3 env_overrides（本 spec 明确不加）

### Never do

- 保留 `instance.context_window_enabled` 字段作为 deprecated（用户明确删）
- 保留 `ModelTemplate.context_window` 字段作为 deprecated（用户明确删）
- 手动写 `~/.cc-switch-tui/db.sqlite` 绕过 DAO
- migration 写为非幂等（启动时多次运行必须安全）
- 跳过测试直接提交
- 改 kimi 模板（独立 intent）

## Success Criteria

| # | 标准 | 验证方式 |
|---|---|---|
| 1 | `cargo test --lib` 全绿 | 终端输出 0 failed |
| 2 | `cargo run --release` 生成的 `~/.cc-switch-tui/aliases.zsh` 中 cl-mini 函数体含 `MiniMax-M3[1m]` | `grep "MiniMax-M3\[1m\]" ~/.cc-switch-tui/aliases.zsh` 命中 |
| 3 | aliases.zsh 含 `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000` | `grep "CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000" ~/.cc-switch-tui/aliases.zsh` 命中 |
| 4 | aliases.zsh **不**含 `DISABLE_COMPACT` / `CLAUDE_CODE_MAX_CONTEXT_TOKENS` | grep 无命中 |
| 5 | 旧 DB `model_id="MiniMax-M3"` 行自动 rename | 手动验证：先创建该行 → 启动 → 查询发现已 rename |
| 6 | 旧 DB `context_window_enabled` 列已 DROP | `sqlite3 .schema instances` 不含该列 |
| 7 | migration 幂等：连续启动两次无副作用 | 第二次启动不报错；DB 状态不变 |
| 8 | `cargo clippy --lib -- -D warnings` clean | 终端输出 0 warning |
| 9 | `cargo fmt --check` clean | 终端输出 0 diff |
| 10 | `CLAUDE.md` 已改写为按场景区分 `[1m]` 后缀描述 | `git diff CLAUDE.md` 体现 |
| 11 | `MiniMax-M2.7-highspeed` 不受影响 | `aliases.zsh` 中 `cl-km1` / `cl-km2`（如有）继续工作 |
| 12 | 新增 6 个后端测试全部 pass | `cargo test --lib -- --list` 列出 |
| 13 | `cargo grep context_window_enabled` 在 src/ 下 0 命中 | 终端验证 |
| 14 | `npm test` 全 pass；web/src/ 中 `grep contextWindowEnabled` 0 命中 | `npm test` + 终端 grep |
| 15 | `make build`（cargo build + web build + embed）成功 | `make build` 0 错误 |
| 16 | web UI 上 InstanceForm / InstanceDetailPage 不再显示 "Context Window" checkbox | 手动验证 |

## Open Questions

收口状态：

| # | 问题 | 状态 |
|---|---|---|
| Q1 | 旧 DB 中 `model_id="MiniMax-M3"` 实例如何处理 | ✅ 已决议：一次性迁移脚本（Phase 1 收口） |
| Q2 | SQLite drop column 迁移路径 | ✅ 已决议：`ALTER TABLE ... DROP COLUMN`（rusqlite bundled SQLite ≥ 3.35 支持） |
| Q3 | M3 是否还需要 `DISABLE_COMPACT=1` / `CLAUDE_CODE_MAX_CONTEXT_TOKENS=1000000` | ✅ 已决议：**不加**——只 `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`（官方文档唯一推荐） |

无未决问题。

## Reference

- 上游 intent：[docs/ys-powers/intent/minimax-m3-1m-suffix.md](../intent/minimax-m3-1m-suffix.md)
- ADR：[docs/adr/0001-minimax-m3-1m-suffix-overrides-claude-md-rule.md](../../adr/0001-minimax-m3-1m-suffix-overrides-claude-md-rule.md)
- 官方文档：[platform.minimaxi.com/docs/token-plan/claude-code](https://platform.minimaxi.com/docs/token-plan/claude-code)
- 模板文件：`src/templates.rs::minimax_template()`
- 自动注入待删逻辑：`src/shell.rs::build_env` line 88-103 附近
- DAO 迁移模式：`src/dao/sqlite_impl.rs::new` line 26-68（现有 ALTER TABLE ADD COLUMN 模式可参照）
- Domain 定义：`src/domain/instance.rs` / `src/domain/template.rs`（待确认路径）
- 历史 CLAUDE.md 约束：原文"不要依赖 `model` 字段的 `[1m]` 后缀（VS Code 扩展会重置）"