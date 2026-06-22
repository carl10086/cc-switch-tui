# Spec: alias-env-diagnostic

## Objective

为 cc-switch-tui 生成的 `cl-*` zsh alias 增加运行时环境变量诊断输出，帮助用户（主要是开发者和高级用户）排查 Claude Code 启动时实际继承的 provider 配置，尤其是 `DISABLE_COMPACT`、`CLAUDE_CODE_MAX_CONTEXT_TOKENS`、`CLAUDE_CODE_AUTO_COMPACT_WINDOW` 等与 auto-compact 相关的变量是否按预期注入。

用户故事：
- 作为用户，当我调用 `cl-mini` 时，我能在终端看到实际传给 Claude Code 的环境变量快照，从而判断 auto-compact 失败是否由 wrap 方式导致。
- 作为开发者，当我切换 provider 时，我能确认上一个 provider 的 env 已被正确清理，当前 provider 的 env 已正确注入。

成功标准：
- 生成的 `~/.cc-switch-tui/aliases.zsh` 包含一个通用 helper 函数 `__cc_switch_print_env`。
- 每个 `cl-*` 函数在 `export` 之后、`command claude "$@"` 之前调用该 helper。
- helper 输出一行到 stderr，包含 compact 相关变量、模板注入变量、alias 标记变量；credential 值显示为 `<redacted>`，未设置变量显示为 `<unset>`。
- 默认开启；设置 `CC_SWITCH_QUIET=1` 可关闭。
- 单元测试覆盖 helper 生成、调用、redaction、开关行为。

## Tech Stack

- 后端：Rust 2024, axum 0.7, tokio
- Shell 生成：`src/shell.rs`（生成 `~/.cc-switch-tui/aliases.zsh`）
- 目标 shell：zsh（`emulate zsh`）

## Commands

```bash
# 开发
cargo test                    # 运行 Rust 单元测试与集成测试
cargo test shell::            # 仅运行 shell 模块测试
cargo clippy                  # lint

# 生成并查看 aliases.zsh（启动服务后通过 Web UI Apply，或调用 API）
# 手动验证：
source ~/.cc-switch-tui/aliases.zsh
cl-mini --help                # 应看到诊断输出行
CC_SWITCH_QUIET=1 cl-mini --help  # 不应看到诊断输出
```

## Project Structure

```
src/
  shell.rs                    # 修改：生成 helper 函数并在 cl-* 中调用
  templates.rs                # 只读参考：模板 env 来源
  domain/
    instance.rs, template.rs  # 只读参考：ProviderInstance / ProviderTemplate
web/
  src/pages/InstancesPage.tsx # 当前未改动；未来如需 UI 开关再扩展
docs/ys-powers/specs/
  2026-06-23-alias-env-diagnostic-design.md  # 本文档
```

## Code Style

生成的 zsh helper 函数示例：

```zsh
function __cc_switch_print_env {
  local alias_name=$1
  [[ -n $CC_SWITCH_QUIET ]] && return 0

  local -a keys=(
    DISABLE_COMPACT
    CLAUDE_CODE_MAX_CONTEXT_TOKENS
    CLAUDE_CODE_AUTO_COMPACT_WINDOW
    ANTHROPIC_BASE_URL
    ANTHROPIC_MODEL
    ANTHROPIC_DEFAULT_HAIKU_MODEL
    ANTHROPIC_DEFAULT_OPUS_MODEL
    ANTHROPIC_DEFAULT_SONNET_MODEL
    API_TIMEOUT_MS
    CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC
    CMUX_PRESERVE_CLAUDE_AUTH_SELECTION_ENV
    CC_SWITCH_ALIAS
  )

  local parts=()
  for k in $keys; do
    local v=${(P)k}
    if [[ -z $v ]]; then
      parts+=("$k=<unset>")
    elif [[ $k =~ '(_TOKEN|_API_KEY|_SECRET|_PASSWORD)$' ]]; then
      parts+=("$k=<redacted>")
    else
      parts+=("$k=$v")
    fi
  done

  echo "[cc-switch-tui] $alias_name: ${(j: :)parts}" >&2
}
```

每个 `cl-*` 函数体内：

```zsh
function cl-mini {
  unset ANTHROPIC_AUTH_TOKEN CC_SWITCH_ALIAS CMUX_PRESERVE_CLAUDE_AUTH_SELECTION_ENV DISABLE_COMPACT CLAUDE_CODE_MAX_CONTEXT_TOKENS CLAUDE_CODE_AUTO_COMPACT_WINDOW
  export ANTHROPIC_AUTH_TOKEN=sk-...
  export CC_SWITCH_ALIAS=cl-mini
  export CMUX_PRESERVE_CLAUDE_AUTH_SELECTION_ENV=1
  __cc_switch_print_env cl-mini
  command claude "$@"
}
```

Rust 侧约定：
- helper 字符串用单独函数生成，避免污染 `render_aliases` 可读性。
- 敏感 key 的正则/后缀列表用常量数组，便于维护。
- 变量列表通过 `get_all_env_vars()` 动态收集，确保模板新增 env 时自动覆盖。

## Testing Strategy

- **单元测试**：`src/shell.rs` 的 `#[cfg(test)]` 模块，使用 `render_aliases()` 生成内容后做字符串断言。
- **关键测试用例**：
  1. `aliases.zsh` 包含 `function __cc_switch_print_env` 定义。
  2. 每个 `cl-*` 函数体在 `command claude` 之前调用 `__cc_switch_print_env <alias>`。
  3. helper 输出包含 `DISABLE_COMPACT`、上下文变量、模板变量、alias 标记变量。
  4. credential 类变量（如 `ANTHROPIC_AUTH_TOKEN`）输出为 `<redacted>`，不泄露值。
  5. `CC_SWITCH_QUIET=1` 时 helper 第一行即返回，不输出。
- **集成测试**：启动服务后通过 API 获取 aliases 内容并做相同断言（可选，单元测试已覆盖）。
- **手动验证**：`source aliases.zsh` 后调用 `cl-mini --help`，观察 stderr 输出行。

## Boundaries

- **Always**：
  - 将诊断输出发送到 stderr，避免污染 Claude Code 的 stdout。
  - credential 值必须 redact，绝不打印 `_TOKEN` / `_API_KEY` / `_SECRET` / `_PASSWORD` 结尾的变量值。
  - helper 函数名前加 `__cc_switch_` 前缀，避免与用户自定义函数冲突。
  - 更新 `src/shell.rs` 测试，确保不回归。

- **Ask first**：
  - 在前端 UI 增加持久化开关（如 Settings 页面勾选"安静模式"）。
  - 将诊断输出格式改为 JSON/多行/结构化日志。
  - 把 helper 逻辑抽到独立 shell 文件而非 inline 在 `aliases.zsh`。

- **Never**：
  - 不要打印 `ANTHROPIC_AUTH_TOKEN`、`MINIMAX_API_KEY`、`KIMI_API_KEY` 等 credential 的真实值。
  - 不要修改 `~/.claude/settings.json` 或 Claude Code 内部行为。
  - 不要让诊断输出影响 Claude Code 的正常参数传递或 stdin/stdout。
  - 不要在前端 trace 或日志中持久化诊断输出内容。

## Success Criteria

1. `cargo test shell::` 全部通过，且新增测试覆盖诊断 helper 的生成、调用、redaction、开关。
2. 生成的 `aliases.zsh` 中 `__cc_switch_print_env` 函数定义在所有 `cl-*` 函数之前。
3. 运行 `cl-mini --help` 时，stderr 出现一行 `[cc-switch-tui] cl-mini: ...` 开头的诊断信息。
4. 运行 `CC_SWITCH_QUIET=1 cl-mini --help` 时，stderr 不出现诊断信息。
5. 诊断行中 credential 变量显示为 `<redacted>`，未设置变量显示为 `<unset>`。

## Open Questions

- 是否需要给 `oc-*` alias 也加同样的诊断输出？当前范围仅限 `cl-*`。
- 未来是否需要在前端 Settings 页面增加"关闭 alias 诊断输出"的持久化开关？当前通过 `CC_SWITCH_QUIET=1` 环境变量控制。
