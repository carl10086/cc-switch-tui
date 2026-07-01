# Intent: MiniMax-M3 model id 改用 `[1m]` 后缀 + 废弃 context_window_enabled 自动注入

## TL;DR

MiniMax 官方更新了 Claude Code 集成文档（[platform.minimaxi.com/docs/token-plan/claude-code](https://platform.minimaxi.com/docs/token-plan/claude-code)），把推荐 model id 从 `MiniMax-M3` 改成 `MiniMax-M3[1m]`，并显式推荐 `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`。

本次意图：
1. 把 `MiniMax-M3` 的 model id / opencode_model_id / 4 个 `ANTHROPIC_DEFAULT_*_MODEL` env_overrides 全部改成 `MiniMax-M3[1m]`
2. 在 MiniMax-M3 的 env_overrides 里**硬编码** `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`
3. **废弃** `instance.context_window_enabled: bool` 字段、`ModelTemplate.context_window: Option<usize>` 字段、以及 `src/shell.rs::build_env` 里的自动注入 block（约 13 行）—— 后续按 model id 自动化配置
4. **保留** `MiniMax-M2.7-highspeed` 模型不动
5. **同步更新** CLAUDE.md 中关于「[1m] 后缀」的硬约束描述

---

## Outcome

- `aliases.zsh` 中 `cl-mini` 函数体显式 export `ANTHROPIC_MODEL="MiniMax-M3[1m]"` 和 `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`
- 用户在 TUI 创建 M3 instance 时不再有 `context_window_enabled` 这个 toggle（彻底删字段，不留 UI 残留）
- 旧 DB 里 `context_window_enabled` 列被 drop；旧 `model_id="MiniMax-M3"` 的实例处理策略在 spec 阶段确定（迁移 / invalid / 兼容）
- `MiniMax-M2.7-highspeed` 实例继续工作（不强制升级）
- kimi / ys-proxy / oc-* / subshell 隔离全部不动

---

## 范围内 (In Scope)

### 代码改动

1. **`src/templates.rs`**：
   - `MiniMax-M3` model 的 `id` / `name` 改为 `MiniMax-M3[1m]` / `"MiniMax M3 [1m]"`
   - `MiniMax-M3[1m]` model 的 4 个 `ANTHROPIC_DEFAULT_*_MODEL` env_overrides 值改为 `MiniMax-M3[1m]`
   - `MiniMax-M3[1m]` model 的 `env_overrides` **新增** `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`
   - `MiniMax-M3[1m]` model 的 `opencode_model_id` 改为 `MiniMax-M3[1m]`
   - `opencode_models` vec 中的 `"MiniMax-M3"` 改为 `"MiniMax-M3[1m]"`
   - `MiniMax-M2.7-highspeed` model：**不动**

2. **`src/domain/instance.rs`**：
   - 删除 `ProviderInstance.context_window_enabled: bool` 字段
   - 所有构造 `ProviderInstance` 的位置（DAO / API handler / tests）同步去掉这个字段

3. **`src/domain/template.rs`（或 `ModelTemplate` 定义处）**：
   - 删除 `ModelTemplate.context_window: Option<usize>` 字段
   - 所有构造 `ModelTemplate` 的位置（`templates.rs` / tests）同步去掉这个字段

4. **`src/shell.rs::build_env`**：
   - 删除约 13 行 auto-inject block（包括 `if let Some(model) = ... if let Some(window) = ...`）
   - 上下文窗口 env vars 完全由 model 模板的 `env_overrides` 字面量提供

5. **`src/dao/sqlite_impl.rs`**：
   - SQLite schema migration：drop `context_window_enabled` 列
   - DAO 读写 `ProviderInstance` 时不再涉及该字段
   - 旧 DB 文件升级路径在 spec 阶段确定

6. **tests/ 下所有测试**：
   - 所有 `fixture()` 调用中 `context_window_enabled: false/true` 字面量删除
   - 所有 `ProviderInstance` 字面量构造点同步删除
   - 所有 `ModelTemplate` 字面量构造点同步删除 `context_window`
   - 直接断言 `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000` 出现在生成 aliases.zsh 中的测试**新增**

### 文档改动

7. **`CLAUDE.md`**：
   - 改写「不要依赖 `model` 字段的 `[1m]` 后缀」硬约束
   - 新表述：VS Code 扩展场景下 env var 可能被重置，仍不建议依赖 `[1m]` 后缀；Claude Code 终端场景下 cc-switch-tui 通过 env 变量注入保证 `[1m]` 后缀生效
   - 移除「`DISABLE_COMPACT=1` + `CLAUDE_CODE_MAX_CONTEXT_TOKENS` 配合机制」相关描述（该机制随字段一起废弃）

8. **`docs/codebase/ARCHITECTURE.md`**（如涉及）：
   - 检查 shell integration 层 / 数据模型层描述是否需要同步

---

## 范围外 (Out of Scope)

- `MiniMax-M2.7-highspeed` 模型（保留不动，不升级）
- kimi 模板（不动）
- `ys-proxy` wrapper / sentinel `CC_SWITCH_PROXY_URL` / `__cc_switch_print_env` helper
- oc-* aliases（不动）
- subshell 隔离（`feat/cl-subshell-wrap` 分支已合并，保持）
- `~/.claude.json` onboarding 配置（用户配置层，非 cc-switch-tui scope）
- 国际 endpoint `https://api.minimax.io/anthropic`（另一条独立 intent）
- `kv_cache_enabled` 字段（与本次无关，仍保留）

---

## 关键设计决策与原因

### 决策 1：跟随官方新文档，使用 `MiniMax-M3[1m]` 后缀

**原因**：官方文档明确推荐此 ID 形式（含 `[1m]` 后缀表示 1M 上下文窗口）。Claude Code 终端场景下通过 `ANTHROPIC_MODEL` env 变量注入后，后缀会原样发送到 MiniMax 后端。

**代价**：违反 `CLAUDE.md` 历史约束——该约束原本针对 VS Code 扩展场景（env var 被重置）。本 intent 改写 CLAUDE.md 区分两种场景。

### 决策 2：硬编码 `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000` 到 M3 env_overrides

**原因**：官方文档明确推荐此 env 变量值。把它放在 model 模板的 `env_overrides` 里，模型 id 决定 env 配置，无需额外 toggle。

**对比之前**：旧逻辑用 `instance.context_window_enabled: bool` 让用户手动决定是否注入。但 M3[1m] 的 1M 上下文窗口是 model 本身固有的属性，不该由用户 toggle。

### 决策 3：彻底废弃 `context_window_enabled` 与 `context_window` 字段

**原因**（用户原话）：「之前的逻辑废弃掉，instance.context_window_enabled 和相关的功能全部废弃，后续根据模型 id 来自动化配置」。

**含义**：
- 不是「保留字段但 deprecated」——是彻底删除（数据库 schema 也 drop column）
- 不留 UI toggle、不留向后兼容代码
- 后续如果其他 model 需要类似配置，全部走 env_overrides 字面量

### 决策 4：保留 `MiniMax-M2.7-highspeed` 不动

**原因**：官方文档不再推荐 M2.7，但用户当前可能有依赖 M2.7 的实例。删除会破坏现有用户工作流。本次 intent 不强制升级，留给后续独立 intent 决定 M2.7 的命运。

### 决策 5：`DISABLE_COMPACT` 与 `CLAUDE_CODE_MAX_CONTEXT_TOKENS` 不再注入

**原因**：官方文档**只**推荐 `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`，不推荐另外两个。旧逻辑同时注入 3 个变量是 over-engineering——用户明确「按 model id 自动化配置」。

**含义**：M3[1m] 的 env_overrides 只新增 `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`，不新增其他两个。如果 spec 阶段发现 `DISABLE_COMPACT=1` 或 `CLAUDE_CODE_MAX_CONTEXT_TOKENS` 是必需的，再补。

---

## 不做 (Non-Goals)

- 不修复其他已知 bug
- 不动 TUI 界面其他部分
- 不动 ys-proxy / oc-* / sentinel 架构
- 不替用户改 `.zshrc` 或 `~/.claude.json`
- 不新增对国际 endpoint 的支持
- 不做 kimi 模板的同步变更

---

## 后续会问但本 intent 不阻塞

| # | 问题 | 决定方式 |
|---|---|---|
| Q1 | 旧 DB 中 `model_id="MiniMax-M3"` 的实例如何处理？ | spec 阶段决定（迁移 / 标记 invalid / 自动 rename） |
| Q2 | SQLite drop column 的迁移路径（DROP COLUMN vs 新建表）？ | spec 阶段决定 |
| Q3 | `MiniMax-M3[1m]` 是否还需要 `DISABLE_COMPACT=1` / `CLAUDE_CODE_MAX_CONTEXT_TOKENS=1000000`？ | spec 阶段验证；当前 intent 仅承诺 `CLAUDE_CODE_AUTO_COMPACT_WINDOW` |
| Q4 | `MiniMax-M2.7-highspeed` 是否也要 deprecate？ | 独立 intent |
| Q5 | 国际 endpoint `api.minimax.io` 何时支持？ | 独立 intent |

---

## 验收方法 (Definition of Done)

任一 `MiniMax-M3[1m]` instance（即 `cl-mini` / `cl-XXX` 任意 alias，model_id 为 `MiniMax-M3[1m]`）：

```sh
# 准备干净环境
$ unset ANTHROPIC_AUTH_TOKEN ANTHROPIC_BASE_URL ANTHROPIC_DEFAULT_HAIKU_MODEL \
        ANTHROPIC_DEFAULT_OPUS_MODEL ANTHROPIC_DEFAULT_SONNET_MODEL \
        ANTHROPIC_MODEL API_TIMEOUT_MS CC_SWITCH_PROXY_URL \
        CC_SWITCH_ALIAS CLAUDE_CODE_MAX_CONTEXT_TOKENS CLAUDE_CODE_AUTO_COMPACT_WINDOW \
        DISABLE_COMPACT CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC \
        CMUX_PRESERVE_CLAUDE_AUTH_SELECTION_ENV

# 验收 1：aliases.zsh 中 cl-mini 含新 ID 与 compact var
$ grep 'ANTHROPIC_MODEL' ~/.cc-switch-tui/aliases.zsh
# 期望: ANTHROPIC_MODEL="MiniMax-M3[1m]"

$ grep 'CLAUDE_CODE_AUTO_COMPACT_WINDOW' ~/.cc-switch-tui/aliases.zsh
# 期望: CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000

# 验收 2：claude 能跑通
$ cl-mini --version
# 期望: 能连到 https://api.minimaxi.com/anthropic

# 验收 3：DB schema 中 context_window_enabled 列已 drop
$ sqlite3 ~/.cc-switch-tui/db.sqlite ".schema provider_instance"
# 期望: 不含 context_window_enabled

# 验收 4：构建 & 测试干净
$ cargo test --lib
# 期望: 全部 pass

$ cargo clippy --lib -- -D warnings
# 期望: 0 warning
```

---

## 上下文引用

- 官方文档：[platform.minimaxi.com/docs/token-plan/claude-code](https://platform.minimaxi.com/docs/token-plan/claude-code)
- 模板文件：`src/templates.rs::minimax_template()` (line 10)
- 自动注入逻辑：`src/shell.rs::build_env` (line 88-103 附近)
- Domain 定义：`src/domain/instance.rs` / `src/domain/template.rs` (待确认路径)
- DAO 层：`src/dao/sqlite_impl.rs`
- 历史 CLAUDE.md 硬约束：「不要依赖 `model` 字段的 `[1m]` 后缀（VS Code 扩展会重置）」

---

## 后续阶段

- `/spec`：细化 SQLite migration 路径、Q1-Q3 等开放问题
- `/plan`：拆解为可执行任务（含垂直切片顺序）
- `/build`：按任务实现 + 测试
- `/ys-review`：五维度代码审查
- `/ship`：交付前检查与 go/no-go