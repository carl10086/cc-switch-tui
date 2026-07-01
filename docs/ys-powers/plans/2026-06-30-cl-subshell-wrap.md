# Implementation Plan: cl-* alias 用 subshell 包裹 + 删除 unset

> 上游 spec：[docs/ys-powers/specs/2026-06-30-cl-subshell-wrap-design.md](../specs/2026-06-30-cl-subshell-wrap-design.md)
> 工作分支：`feat/cl-subshell-wrap`
> 涉及文件：仅 `src/shell.rs`

## Overview

修改 `src/shell.rs::format_function` 渲染 cl-* function 体的逻辑：删除 `unset` 行、外层包 `(...)`、删 `unset_vars` 参数；改写 3 个旧测试（断言已不存在的 unset 行为）+ 删除 1 个过时的测试 + 新增 3 个断言新行为的运行时测试。零数据库、零 HTTP、零 oc-* 改动。

## Architecture Decisions

- **不改 oc-* / ys-proxy / `__cc_switch_print_env` / sentinel / `PROXY_OVERRIDE_LINE`**：scope 内只动 cl-* function 体生成逻辑
- **删 `unset_vars` 参数**（用户已确认 dead code 必删）：调用点 line 24 同步去掉实参
- **运行时测试走真实 zsh 子进程**：与现有 `test_function_body_isolates_previous_alias_export` / `test_ys_proxy_sentinel_overrides_anthropic_base_url` 同模式（`std::process::Command::new("zsh")` + 临时目录 + stub `claude` 二进制）
- **垂直切片**：每个任务交付一个**可独立编译 / 可独立测试**的中间状态，CI 在任意中间点都能跑

## Task List

### Phase 1: Source change

#### Task 1: 修改 `format_function` 签名与函数体 + 更新 caller

**Description:** 删除 `unset_vars` 参数；函数体输出改为 `(...)` 包裹 + 去掉 `unset ...` 行 + 内部缩进 +2 空格；调用点 `render_aliases` 同步去实参。

**Acceptance criteria:**
- [ ] `format_function` 签名只保留 `(name, env, kv_cache_enabled)`
- [ ] 函数体内不再生成 `unset ...` 行
- [ ] 函数体外层以 `  (` 开头、内部 `  )` 结尾（缩进 +2）
- [ ] `export` / `__cc_switch_print_env` / `command claude "$@"` 三段仍按原顺序出现在 subshell 内
- [ ] `PROXY_OVERRIDE_LINE` 仍以字面量形式保留在 subshell 内
- [ ] `render_aliases` line 24 调用点不再传 `&all_env_vars`
- [ ] `get_all_env_vars` 函数被移除（仅此 caller 用，且无 setter；如仍有其他引用则保留）

**Verification:**
- [ ] `cargo build` 成功
- [ ] `cargo fmt --check` 0 diff
- [ ] `cargo clippy -D warnings` 0 warning
- [ ] `cargo test --lib shell::tests` —— 此时会 fail（预期，下个 task 修）

**Dependencies:** None

**Files likely touched:**
- `src/shell.rs`（`format_function` 函数体、`render_aliases` line 24 caller、`get_all_env_vars` 可能整段删除）

**Estimated scope:** S（1 个文件，1 个函数体 + 1 个调用点）

---

### Checkpoint 1: Source change compiles
- [ ] `cargo build` 成功
- [ ] `cargo fmt --check` / `cargo clippy -D warnings` 0 警告
- [ ] 已知 4 个测试会 fail（`test_generate_aliases_content` / `test_generate_aliases_contains_unset_vars` / `test_unset_includes_anthropic_base_url_even_when_template_omits_it` / `test_function_body_isolates_previous_alias_export` 注释需更新），其他 pass

### Phase 2: Test rewrites + dead code deletion

#### Task 2: 改写 `test_generate_aliases_content` + 改写 `test_generate_aliases_contains_unset_vars` + 删除 `test_unset_includes_anthropic_base_url_even_when_template_omits_it`

**Description:** 
- `test_generate_aliases_content` (line 282)：移除 `assert!(content.contains("unset"))` (line 322)；改为断言 `content.contains("function cl-mini {\n  (")` 与 `!content.lines().any(|l| l.trim_start().starts_with("unset "))`
- `test_generate_aliases_contains_unset_vars` (line 328) → 重命名为 `test_generate_aliases_omits_unset_line`：断言 ① cl-* 函数体外层 `(` / `)` 各占独立行，② 整文件无以 `unset ` 开头的行（在 cl-* 块内），③ `export ANTHROPIC_AUTH_TOKEN=sk-test` 与 `export CC_SWITCH_ALIAS=cl-mini` 仍存在
- `test_unset_includes_anthropic_base_url_even_when_template_omits_it` (line 977)：整段删除（含注释），`unset_vars` 参数已不存在

**Acceptance criteria:**
- [ ] 上述 3 个测试函数全部更新到位（改名/删除）
- [ ] 测试名体现新意图（"omits_unset_line" / "contains_subshell_wrap"）
- [ ] 无遗留 `#[ignore]` / 占位注释
- [ ] `cargo test --lib shell::tests` 改写涉及的测试全部 pass

**Verification:**
- [ ] `cargo test --lib shell::tests::test_generate_aliases_content` pass
- [ ] `cargo test --lib shell::tests::test_generate_aliases_omits_unset_line` pass
- [ ] `cargo grep test_unset_includes_anthropic_base_url` 无匹配（已删除）

**Dependencies:** Task 1

**Files likely touched:**
- `src/shell.rs`（tests 模块内 2 处改写 + 1 处删除）

**Estimated scope:** S（单文件多测试函数改写）

---

#### Task 3: 改写 `test_function_body_isolates_previous_alias_export` 为 `test_function_body_subshell_isolates_claude_url`

**Description:** 
- 函数名改 `test_function_body_subshell_isolates_claude_url`（新意图：通过 subshell 而不是 unset 实现隔离）
- 保留核心断言（cl-mini 在 cl-kimi 调用后仍用 minimaxi URL）
- 强化断言（**新行为**）：脚本末尾 `env | grep -E '^(ANTHROPIC|API_TIMEOUT_MS|CC_SWITCH_ALIAS)='` 输出为空，证明父 shell 未被污染
- 函数顶部注释从"cl-mini 在父 shell 被污染时应仍用 minimaxi URL"改为"通过 subshell 隔离，cl-mini 在 cl-kimi 残留后仍用 minimaxi URL，且调用结束后父 shell 干净"

**Acceptance criteria:**
- [ ] 测试重命名并加注释
- [ ] 新增父 shell 不污染断言
- [ ] cargo test pass

**Verification:**
- [ ] `cargo test --lib shell::tests::test_function_body_subshell_isolates_claude_url` pass

**Dependencies:** Task 1, Task 2（避免测试间 fixture 冲突）

**Files likely touched:**
- `src/shell.rs`（tests 模块内 1 处改写）

**Estimated scope:** S

---

### Checkpoint 2: All rewrites pass
- [ ] `cargo test --lib shell::tests` 0 failed（除尚未新增的 3 个新测试）
- [ ] 现有 11 个测试（原 12+ 减去 1 个删除）+ 改写 3 个全部 green
- [ ] `cargo fmt --check` / `cargo clippy -D warnings` 0 警告

### Phase 3: New tests

#### Task 4: 新增 `test_function_body_no_parent_shell_pollution`

**Description:** 真实 zsh 子进程验证 cl-* 调用结束后父 shell 无 `ANTHROPIC_*` / `API_TIMEOUT_MS` / `CC_SWITCH_ALIAS` 残留。

实现要点：
- 用 `fixture()` helper 生成 minimaxi provider
- `generate_aliases()` 写临时目录
- stub `claude` 二进制仅打印 `"OK\n"` 后退出
- zsh 脚本：`source aliases.zsh && cl-mini > /dev/null 2>&1 && env | grep -E '^(ANTHROPIC|API_TIMEOUT_MS|CC_SWITCH_ALIAS)=' > poll.txt`
- 断言 `poll.txt` 内容为空

**Acceptance criteria:**
- [ ] 新测试函数存在并 pass
- [ ] zsh 不可用时优雅 return（沿用现有 `if zsh --version is_err() return` 模式）
- [ ] 测试注释清晰说明验证目标

**Verification:**
- [ ] `cargo test --lib shell::tests::test_function_body_no_parent_shell_pollution` pass

**Dependencies:** Task 1

**Files likely touched:**
- `src/shell.rs`（tests 模块内 1 处新增）

**Estimated scope:** S

---

#### Task 5: 新增 `test_function_body_preserves_zshrc_exports`

**Description:** 真实 zsh 子进程验证用户在父 shell export 的 `MINIMAX_API_KEY` 等不被 cl-* 调用破坏。

实现要点：
- zsh 脚本：先 `export MINIMAX_API_KEY=zshrc_value_should_survive`，再 `source aliases.zsh && cl-mini > /dev/null 2>&1`，最后 `echo "AFTER=$MINIMAX_API_KEY"`
- 断言输出包含 `AFTER=zshrc_value_should_survive`

**Acceptance criteria:**
- [ ] 新测试函数存在并 pass
- [ ] 测试名称 / 注释清晰
- [ ] 与现有 `test_ys_proxy_rejects_non_localhost_sentinel_value` 的"父 shell 预先 export"模式一致

**Verification:**
- [ ] `cargo test --lib shell::tests::test_function_body_preserves_zshrc_exports` pass

**Dependencies:** Task 1

**Files likely touched:**
- `src/shell.rs`（tests 模块内 1 处新增）

**Estimated scope:** S

---

#### Task 6: 新增 `test_function_body_subshell_reads_ys_proxy_sentinel`

**Description:** 真实 zsh 子进程验证 subshell 内能读到父 shell 的 `CC_SWITCH_PROXY_URL` sentinel，且条件 override 在 subshell 内生效。

实现要点：
- 用 `fixture()` 生成 minimaxi provider
- stub `claude` 二进制打印 `echo "CLAUDE_URL=$ANTHROPIC_BASE_URL"`
- zsh 脚本：`export CC_SWITCH_PROXY_URL=http://localhost:7480/ys-proxy/cl-mini && source aliases.zsh && cl-mini > out.txt 2>&1`
- 断言 `out.txt` 含 `CLAUDE_URL=http://localhost:7480/ys-proxy/cl-mini`（即 subshell 内条件判断生效）
- 与现有 `test_ys_proxy_sentinel_overrides_anthropic_base_url` 不同：本测试**不通过 ys-proxy wrapper**，而是直接 `cl-mini`，因为用户预先 export 了 sentinel 模拟 ys-proxy 已注入的环境——更精确隔离 subshell 能否读 sentinel 的能力

**Acceptance criteria:**
- [ ] 新测试函数存在并 pass
- [ ] 与现有 `test_ys_proxy_sentinel_overrides_anthropic_base_url` 互为补充（一个测 ys-proxy wrapper，一个测 subshell 直读）

**Verification:**
- [ ] `cargo test --lib shell::tests::test_function_body_subshell_reads_ys_proxy_sentinel` pass

**Dependencies:** Task 1

**Files likely touched:**
- `src/shell.rs`（tests 模块内 1 处新增）

**Estimated scope:** S

---

### Checkpoint 3: All tests pass + final verification
- [ ] `cargo test --lib shell::tests` —— 全部 14 个测试（原 11 + 改写 3 + 新增 3 - 删除 1 + ys-proxy 套件）pass
- [ ] `make test`（cargo + npm）全 pass
- [ ] `make lint` clean
- [ ] 手动：`cargo run` 生成 `~/.cc-switch-tui/aliases.zsh`，diff 对比 spec 目标示例（cl-* 块结构、`(...)` 包裹、无 `unset `）
- [ ] 手动：开新 shell `cl-mini --version`，跑完后 `env | grep -E '^(ANTHROPIC|API_TIMEOUT_MS|CC_SWITCH_ALIAS)'` 为空

## Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| `get_all_env_vars` 还有其他隐藏 caller 导致删除报错 | Low | grep 全项目确认；只服务 `render_aliases` 一处 |
| 测试 fixture 改变后某个旧测试意外通过但语义错 | Low | Task 2/3 改写后跑 `cargo test --lib shell::tests` 全量；任何新通过但语义跑偏的测试会被后续 Task 4-6 的运行时测试 catch |
| zsh 不可用导致运行时测试 skip（macOS 普遍有，但 CI runner 不一定有） | Med | 沿用现有 `if zsh --version is_err() return` 模式；新测试统一加这个 guard |
| subshell 内 `export` 排序变化导致 diff 不稳定 | Low | 保留现有 `export_lines.sort()`；spec 已说明排序稳定 |
| `PROXY_OVERRIDE_LINE` 在 subshell 内能否正常工作（条件判断仍生效） | Med | Task 6 专门测这条 |

## Parallelization Opportunities

- **Task 4 / 5 / 6**：三个新增测试**互相独立**，可串行写完各 ~30 行；但建议串行（同一文件相邻位置，便于 review）
- **Task 2 的 3 个改动**：必须顺序（同一测试模块，相邻行段）
- Task 3 必须依赖 Task 2（避免 fixture 命名冲突时改写互相覆盖）

## Open Questions

无。所有设计决策已收口。