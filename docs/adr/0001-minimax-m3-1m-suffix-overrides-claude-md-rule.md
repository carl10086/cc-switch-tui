# MiniMax-M3 model id 使用 `[1m]` 后缀，打破 CLAUDE.md 旧"不要依赖 `[1m]` 后缀"约束

MiniMax 官方在 2026 年更新 Claude Code 集成文档，把推荐 model id 改为 `MiniMax-M3[1m]`（含 `[1m]` 后缀表示 1M 上下文窗口），并显式推荐 `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`。本次决议：**跟随官方文档**，把 `MiniMax-M3` → `MiniMax-M3[1m]`，同时改写 CLAUDE.md 中"不要依赖 `[1m]` 后缀"的硬约束，按场景区分（VS Code 扩展场景仍不依赖；Claude Code 终端场景下通过 env 变量注入保证后缀生效）。

## Status

accepted (2026-06-30)

## Context

CLAUDE.md 历史约束写道：

> 不要依赖 `model` 字段的 `[1m]` 后缀（VS Code 扩展会重置）；用 env 变量 `ANTHROPIC_BASE_URL` / `ANTHROPIC_AUTH_TOKEN` / `ANTHROPIC_MODEL` 组合覆盖。

该约束的根因是 **VS Code 扩展场景**下，extension 会重置进程 env，导致 `[1m]` 后缀丢失。

但 cc-switch-tui 的 `cl-*` alias 是为 **Claude Code 终端**（CLI）设计的：用户在 `~/.zshrc` 中 source `~/.cc-switch-tui/aliases.zsh`，cl-* 作为 zsh function 注入 env。终端场景下 env var 不会被重置，`[1m]` 后缀可以稳定生效。

MiniMax 官方新文档明确推荐 `MiniMax-M3[1m]` 作为 model id——这是 MiniMax 后端能正确识别 1M 上下文窗口的请求字段值。

## Decision

- `MiniMax-M3` model id → `MiniMax-M3[1m]`
- `MiniMax-M3[1m]` model 的 4 个 `ANTHROPIC_DEFAULT_*_MODEL` env_overrides 值改为 `MiniMax-M3[1m]`
- `MiniMax-M3[1m]` model 的 `opencode_model_id` 改为 `MiniMax-M3[1m]`
- `CLAUDE.md` 中"不要依赖 `[1m]` 后缀"硬约束改写为按场景区分：
  > VS Code 扩展场景下 env var 可能被重置，仍不建议依赖 `[1m]` 后缀；
  > Claude Code 终端场景下 cc-switch-tui 通过 env 变量注入保证 `[1m]` 后缀生效。

## Considered Options

### Option A（采纳）：跟随官方文档，`MiniMax-M3` → `MiniMax-M3[1m]`

**优点**：
- 与 MiniMax 官方推荐一致
- Claude Code 终端场景下后端能正确识别 1M 上下文窗口
- 干净——无遗留 alias 机制

**缺点**：
- VS Code 扩展场景下后缀可能丢失
- 违反 CLAUDE.md 旧约束（需改写）

### Option B：保持 `MiniMax-M3` 裸名，1M 窗口仅靠 `CLAUDE_CODE_MAX_CONTEXT_TOKENS` env var 表达

**优点**：
- 不违反 CLAUDE.md 旧约束
- 兼容 VS Code 扩展场景

**缺点**：
- 与官方推荐不符
- MiniMax 后端可能用 model id 自身识别上下文窗口（而非 env var），导致 1M 窗口行为退化为默认（可能 200K）
- 用户需手动验证后端行为

### Option C：在 env var 里用 `MiniMax-M3[1m]`，model id 仍叫 `MiniMax-M3`

**优点**：
- UI 显示简洁 ID
- env var 携带后缀

**缺点**：
- 引入"显示名"与"实际请求 model 字段"不一致的概念
- 给后续维护增加认知负担
- 文档、测试、UI 三处都要同步区分两种名称

## Consequences

- DB 迁移：旧 `model_id="MiniMax-M3"` 行在 `SqliteDaoImpl::new` 中一次性 UPDATE 为 `MiniMax-M3[1m]`。不可逆（但无回滚需求）。
- 旧 `context_window_enabled` 列 DROP COLUMN（用户已确认"废弃代码全部删除"）。
- `MiniMax-M2.7-highspeed` 不动（独立命运）。
- 如果将来 MiniMax 改回不带后缀的 id，需再次 DB migration。考虑成本，远期可在 env_overrides 字面量里改 + 写一个 V2 migration。
- VS Code 扩展用户不受 cc-switch-tui 覆盖（他们通常不用 `cl-*` alias），无实际影响。