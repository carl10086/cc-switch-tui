
# cc-switch-tui

Rust + Axum 后端 + 嵌入 React SPA 的单二进制工具，用于在终端 / Web 界面切换多个 Claude Code 模型提供商（MiniMax、Kimi、OpenCode 等）。

<IMPORTANT>
`docs/codebase/ARCHITECTURE.md` 是本项目架构的**唯一权威来源**：分层、数据流、关键抽象（`Dao` trait / `AppState` / `ApiError` / `AppError`）、入口点、错误处理策略、新代码放置规范。任何新增模块、重构或理解现有代码前必读。修改代码时同步更新 ARCHITECTURE.md。
</IMPORTANT>

## Tech Stack

- **后端**: Rust 2024, axum 0.7, tokio, rusqlite(bundled), reqwest 0.12, tracing, thiserror
- **前端**: React 18.3, Vite 5, Tailwind 3, TanStack Query 5, React Router 6, Zod 3, Vitest 4
- **嵌入**: `include_dir!` 把 `web-dist/` 编进二进制（编译期发生）
- **持久化**: SQLite 在 `~/.cc-switch-tui/{db.sqlite, traces.sqlite}`

## Commands

```bash
make dev            # Vite :5173 (HMR) + cargo :7480 — 改 web 即时刷新，改 src/* 需 Ctrl+C 重启
make dev-rust-only  # 仅 cargo；前提: web-dist/ 已存在
make build          # web-build + cargo build --release → dist/cc-switch-tui-macos-arm64
make test           # cargo test + npm test
make typecheck      # web/src 的 tsc --noEmit
make lint           # cargo clippy -D warnings + eslint
make fmt            # cargo fmt
make release        # git push tag（需先 make tag VERSION=x.y.z）
```

后端固定端口 7480；冲突 fallback 与端口文件语义见 `docs/codebase/ARCHITECTURE.md` 的 "Port Management"。

## Boundaries

ARCHITECTURE.md 描述"系统怎么工作"；本节规定"什么不能动"。

- **不要修改** `~/.cc-switch-tui/port` 文件的格式 / 语义；端口逻辑集中在 `src/port.rs`。
- **不要把** provider-specific env（`CLAUDE_CODE_AUTO_COMPACT_WINDOW`、`ANTHROPIC_MODEL` 等）**写入 common config**——必须 per-provider 注入；context window 相关 env vars 由 model template 的 `env_overrides` 字面量决定（不再走 `instance.context_window_enabled` toggle，字段已废弃）。
- **不要新增** Crate / npm 依赖而不更新 `docs/codebase/ARCHITECTURE.md` 的 "Pattern Overview" 与对应层 "Key files" 列表。
- **不要在 PR 中提交** `app.log`、`traces.sqlite`、`dist/`、`web-dist/`、`target/`（已在 `.gitignore` 但需复检）。
- **不要在** `src/proxy/upstream.rs::send_request` **之外**注入 `Authorization` header——`api_key` 注入是 proxy 唯一职责，handler / DAO 都不该接触。
- **不要绕开** `ApiError` 直接 `IntoResponse`——所有错误必须经过 `src/api/error.rs` 的统一 JSON 格式（前端 `web/src/api/client.ts::toApiError` 依赖此结构）。
- **不要省略** `web/src/lib/curl.ts::buildCurl` 的 hop-by-hop 过滤——前端在 trace viewer 暴露给用户"Copy as curl"按钮，泄漏 `host` / `content-length` 会让复现失败。
- **不要在** 改 `web/src/**` **后**只 `cargo run`——embed 是编译期发生的，必须 `make web-build && make build`。
- **写含 API key 的配置文件**（opencode JSON、aliases.zsh）**权限必须 600**——`src/opencode_config.rs::write_opencode_config` 已设置 `0o600`，新增路径需同样处理。

## Provider 配置参考

第三方 provider（MiniMax / Kimi 等）通过 Claude Code 的 env 变量注入。

**对本项目代码的硬约束**（不可妥协）:

1. 上下文相关 env 必须在 provider scope（不要放 common config；不同 provider 窗口大小不同：200K vs 1M）。
2. `model` 字段的 `[1m]` 后缀**仅在 Claude Code 终端场景下可靠**：cc-switch-tui 通过 `ANTHROPIC_MODEL` env 变量在 cl-* 函数体内注入，shell 进程不被重置。**VS Code 扩展场景下 env var 可能被 extension 重置**，需自行验证。当前 follow 官方文档使用 `MiniMax-M3[1m]`（含后缀）。
3. Provider 切换是 alias 级别（shell function 隔离 env），不要在进程内做 env 覆盖——`ys-proxy` wrapper 在子 shell 中重设 `ANTHROPIC_BASE_URL`，是唯一允许的代理路径。

完整 env 表、已知 Issue（#46416 / #50083 / #57964 / #63471 / #63376 / #62353）保留在 `git log -p CLAUDE.md` 的 v1 历史中可查。

### Kimi `k3[1m]` 官方推荐 env 示例

```bash
export ANTHROPIC_BASE_URL=https://api.kimi.com/coding/
export ANTHROPIC_API_KEY=<你的API Key>

export ANTHROPIC_MODEL="k3[1m]"
export ANTHROPIC_DEFAULT_FABLE_MODEL=$ANTHROPIC_MODEL
export ANTHROPIC_DEFAULT_OPUS_MODEL=$ANTHROPIC_MODEL
export ANTHROPIC_DEFAULT_SONNET_MODEL=$ANTHROPIC_MODEL
export ANTHROPIC_DEFAULT_HAIKU_MODEL=$ANTHROPIC_MODEL
export CLAUDE_CODE_SUBAGENT_MODEL=$ANTHROPIC_MODEL

export CLAUDE_CODE_AUTO_COMPACT_WINDOW=1048576
export CLAUDE_CODE_MAX_CONTEXT_TOKENS=1048576
export CLAUDE_CODE_EFFORT_LEVEL=max

claude
```

cc-switch-tui 将上述变量作为 `k3[1m]` 的 `env_overrides` 字面量注入；其他 model id 的上下文大小与路由槽位同理，由各自 `env_overrides` 决定。
