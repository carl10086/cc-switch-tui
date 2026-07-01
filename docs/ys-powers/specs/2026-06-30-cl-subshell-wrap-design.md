# Spec: cl-* alias 用 subshell 包裹 + 删除 unset

> 上游 intent：[docs/ys-powers/intent/alias-prefix-env.md](../intent/alias-prefix-env.md)
> 工作分支：`feat/cl-subshell-wrap`

## Objective

**做什么**：修改 `src/shell.rs::format_function` 渲染 cl-* function 体的逻辑，删除函数体开头的 `unset ...` 行，并在函数体外层包 `(...)` 形成 subshell。

**为什么**：当前 cl-* function 用 `export` 写 env，命令结束后变量残留在父 shell 里——破坏用户 `~/.zshrc` 设的同名变量（如 `MINIMAX_API_KEY`），也污染交互环境。subshell 是 POSIX 标准的隔离原语，把 export 限制在子进程内。

**目标用户**：在 terminal 用 `cl-mini` / `cl-km1` / `cl-km2` / `ys-proxy cl-mini` 调用 Claude Code 的开发者。

**成功的样子**：

| 验收项 | 期望 |
|---|---|
| `cl-mini --version` 能跑通 | claude 用正确的 ANTHROPIC_BASE_URL 连到 provider |
| 调用结束后父 shell 干净 | `env \| grep -E '^(ANTHROPIC\|API_TIMEOUT_MS\|CC_SWITCH_ALIAS)'` 输出为空 |
| `ys-proxy cl-mini --version` 仍生效 | subshell 内能读到 `CC_SWITCH_PROXY_URL`，BASE_URL 被 sentinel 覆盖 |
| `~/.zshrc` 设的 `MINIMAX_API_KEY` 不被破坏 | 调用前后值不变 |
| `__cc_switch_print_env` 诊断输出仍正常 | stderr 仍能看到 alias 的生效 env |

## Tech Stack

- Rust 2024 edition（项目既定）
- `format!()` 宏 + `Vec<String>::join("\n")` 拼接字符串（沿用 `format_function` 现有模式）
- 不引入新依赖
- 测试用 Rust 内置 `#[test]` + `std::process::Command` 调外部 zsh 验证运行时行为（已有 `tests/aliases_test.rs` 模式参考）

## Commands

```bash
# 构建
make build

# 测试（重点关注 shell 模块）
cargo test --lib shell::tests
make test

# Lint / Format
make lint              # cargo clippy -D warnings + eslint
make fmt               # cargo fmt
cargo fmt --check      # 仅检查不写

# 单测改完后手动跑全套
cargo test --lib shell::tests -- --nocapture
```

## Project Structure

本次改动触及的文件：

```
src/shell.rs                                # 唯一生产代码改动点
  ├─ format_function()                      # 函数体重写（包 subshell + 删 unset + 改签名）
  └─ tests                                  # 3 改 + 3 增

tests/aliases_test.rs                       # 不动（已是不同关注点）

docs/ys-powers/specs/2026-06-30-cl-subshell-wrap-design.md   # 本文件
docs/ys-powers/intent/alias-prefix-env.md   # 上游 intent（已确认）
```

不动：
- `src/opencode_config.rs`（oc-* 走另一条渲染路径）
- `src/shell.rs::build_env` / `get_all_env_vars` / `render_aliases` / `generate_aliases`（上层调用方不变）
- `src/shell.rs` 顶部 helper / ys-proxy 字符串
- 数据库 / DAO / HTTP handler

## Code Style

### 函数签名变更

```rust
// 旧
fn format_function(
    name: &str,
    env: &HashMap<String, String>,
    unset_vars: &[String],          // ← 删（变 dead code）
    kv_cache_enabled: bool,
) -> String { ... }

// 新
fn format_function(
    name: &str,
    env: &HashMap<String, String>,
    kv_cache_enabled: bool,
) -> String { ... }
```

唯一调用点（line 24）相应去掉 `&all_env_vars` 实参。

### 函数体目标形态

```rust
fn format_function(
    name: &str,
    env: &HashMap<String, String>,
    kv_cache_enabled: bool,
) -> String {
    // subshell 内 export —— 父 shell 不受影响（POSIX 隔离）。
    // 排序后输出，保持生成的 aliases.zsh diff 稳定。
    let mut export_lines: Vec<String> = env
        .iter()
        .map(|(k, v)| format!("    export {}={}", k, shell_escape(v)))
        .collect();
    export_lines.sort();
    let export_block = export_lines.join("\n");

    // ys-proxy 通过命令前缀方式把 CC_SWITCH_PROXY_URL 注入到本次调用；
    // 若命令前缀设置了它，则覆盖 ANTHROPIC_BASE_URL。
    // 限制为 localhost / 127.0.0.1，避免父 shell 中误 export 的 sentinel
    // 把请求路由到任意 host。
    let proxy_override_line = PROXY_OVERRIDE_LINE;

    let claude_cmd = if kv_cache_enabled {
        "command claude --exclude-dynamic-system-prompt-sections --settings '{\"includeGitInstructions\":false}' \"$@\""
    } else {
        "command claude \"$@\""
    };

    format!(
        "function {} {{\n  (\n{}\n{}\n    __cc_switch_print_env {}\n    {}\n  )\n}}",
        name, export_block, proxy_override_line, name, claude_cmd
    )
}
```

### 生成的 zsh 目标示例

```zsh
function cl-mini {
  (
    export ANTHROPIC_AUTH_TOKEN=sk-cp-...
    export ANTHROPIC_BASE_URL=https://api.minimaxi.com/anthropic
    ...
    [[ $CC_SWITCH_PROXY_URL == http://localhost:* || $CC_SWITCH_PROXY_URL == http://127.0.0.1:* ]] && export ANTHROPIC_BASE_URL="$CC_SWITCH_PROXY_URL"
    __cc_switch_print_env cl-mini
    command claude "$@"
  )
}
```

### 关键约定

- subshell 的 `(` 与 `)` 各占独立行
- 函数体内部缩进 +2 空格（即从 `function {` 后的 2 空格变成 4 空格）
- 不生成 `unset ...` 行（subshell 是新进程，inherit 干净的父 env，无需清理）
- 保留 `PROXY_OVERRIDE_LINE` 字面量行（条件判断放在 subshell 内仍生效）
- 保留 `__cc_switch_print_env <name>` 调用（file-scope helper，subshell 内可见）
- 保留 `command claude "$@"`（防御同名 alias，subshell 内无害）
- 注释用中文（与现有 `format_function` 一致）
- 测试函数命名：`test_<subject>_<expectation>`

## Testing Strategy

测试框架：Rust 内置 `#[test]` + `#[cfg(test)]` 模块（沿用 `src/shell.rs` 现有模式）。

### 测试位置

- 单元测试（字符串断言）：保留在 `src/shell.rs::tests` 模块内
- 运行时测试（spawn zsh 验证 env 行为）：同样在 `src/shell.rs::tests`，参照现有 `test_print_env_helper_runtime_output` (line 593) 的模式

### 改写的测试（3 个）

| 旧测试 | 新测试 | 新断言 |
|---|---|---|
| `test_generate_aliases_contains_unset_vars` (line 328) | `test_generate_aliases_omits_unset_line` | 输出**不**含 `unset ` 行（关键：`function cl-mini {` 后第一行不是 `unset`） |
| `test_function_body_isolates_previous_alias_export` (line 823) | `test_function_body_uses_subshell` | 输出含 `function <name> {\n  (\n...  )\n}` 结构（即 `(` 与 `)` 各占独立行，紧贴花括号） |
| `test_unset_includes_anthropic_base_url_even_when_template_omits_it` (line 977) | （删除） | `unset_vars` 参数已删除，测试无意义 |

### 新增的测试（3 个）

| 测试 | 验证内容 | 实现要点 |
|---|---|---|
| `test_function_body_no_parent_shell_pollution` | cl-* 调用结束后父 shell 无 `ANTHROPIC_*` / `API_TIMEOUT_MS` / `CC_SWITCH_ALIAS` 残留 | `std::process::Command::new("zsh")` + `source` aliases.zsh + `cl-mini --version` + 子进程退出后 `Command::new("zsh").arg("-c").arg("env \| grep ...")` 断言空 |
| `test_function_body_preserves_zshrc_exports` | 用户在父 shell 设的 `MINIMAX_API_KEY` 等不被破坏 | 子进程先 `export MINIMAX_API_KEY=zshrc_value`，跑 `cl-mini --version`，再 echo 该变量，断言仍是 `zshrc_value` |
| `test_function_body_subshell_reads_ys_proxy_sentinel` | subshell 内能读父 shell 的 `CC_SWITCH_PROXY_URL`，且条件 override 生效 | 子进程先 `CC_SWITCH_PROXY_URL=http://localhost:7480/ys-proxy/cl-mini`，跑 `cl-mini --version`（用一个 mock `claude` 替换真命令，把收到的 env 写到临时文件），读文件断言 `ANTHROPIC_BASE_URL` 等于 sentinel 值 |

### 测试覆盖率期望

- 现有 12+ 测试全部继续 pass
- 新 3 测试断言本次新行为
- `cargo clippy -D warnings` 0 warning
- `cargo fmt --check` 0 diff

## Boundaries

### Always do

- 改 `format_function` 后跑 `cargo test --lib shell::tests`
- 提交前跑 `cargo fmt` 与 `make lint`
- 函数体外的 helper（`__cc_switch_print_env`）和 ys-proxy wrapper 字符串完全不动
- 改测试时删除的 dead 测试代码不留尾（不留注释占位、不留 `#[ignore]`）
- 注释保持中文（与现有 `format_function` 一致）

### Ask first

- 加任何新 crate 依赖（本任务不需要）
- 改 `oc-*` / ys-proxy / sentinel / `__cc_switch_print_env` 任一项
- 改 `src/opencode_config.rs` / `src/api/aliases.rs` / `src/port.rs`
- 改 ARCHITECTURE.md / CLAUDE.md
- 改 Cargo.toml / package.json

### Never do

- 触碰 SQLite schema 或 DAO 层任何代码
- 在生成的 `~/.cc-switch-tui/aliases.zsh` 里写永久变更（那是工具输出，下次运行会被覆盖）
- 在 cl-* 输出里重新加 `unset ...` 行（subshell 不需要）
- 移除 `PROXY_OVERRIDE_LINE`（ys-proxy sentinel 仍依赖它）
- 跳过测试直接提交
- 把 `unset_vars` 参数作为 `_` 前缀"保留"在签名里（用户已确认删 dead code）

## Success Criteria

| # | 标准 | 验证方式 |
|---|---|---|
| 1 | `cargo test --lib shell::tests` 全部 pass | 终端输出 0 failed |
| 2 | `cargo run` 生成的 `~/.cc-switch-tui/aliases.zsh` 中 cl-* 三条 function 体外层有 `(` / `)` | 肉眼 diff + `grep -A1 'function cl-'` |
| 3 | cl-* 输出**不**含 `unset ` 行 | `grep 'unset ' ~/.cc-switch-tui/aliases.zsh` 在 cl-* 块中无匹配 |
| 4 | `cl-mini --version` 后父 shell 无 ANTHROPIC_* / API_TIMEOUT_MS / CC_SWITCH_ALIAS | `env \| grep -E '^(ANTHROPIC\|API_TIMEOUT_MS\|CC_SWITCH_ALIAS)='` 输出为空 |
| 5 | `ys-proxy cl-mini --version` 仍能让 claude 走本地 proxy | 需 cc-switch-tui 服务在 7480；否则断言 `__cc_switch_print_env` 输出中 `ANTHROPIC_BASE_URL` 已被 sentinel 覆盖 |
| 6 | 用户 `~/.zshrc` 里的 `MINIMAX_API_KEY` 调用前后不变 | diff 前后 `env \| grep MINIMAX_API_KEY` |
| 7 | `cargo clippy -D warnings` clean | 终端输出 0 warning |
| 8 | `cargo fmt --check` clean | 终端输出 0 diff |
| 9 | 改写后的 3 个测试 + 新增的 3 个测试全部存在且 pass | `cargo test --lib shell::tests -- --list` 列出 6 个对应测试名 |

## Open Questions

无。所有设计决策已在 Phase 1 收口。

## Reference

- 上游 intent：`docs/ys-powers/intent/alias-prefix-env.md`
- 实测验证：`/tmp/subshell-verify.zsh`（TEST 1/2/3 全部通过）
- 用户手动验证：`~/.cc-switch-tui/aliases.zsh` 已按 subshell 形态改写并跑通
- 备份：`/tmp/aliases.zsh.bak.1782805582`
- 架构权威：`docs/codebase/ARCHITECTURE.md`（pattern 5 描述 shell integration 现状）