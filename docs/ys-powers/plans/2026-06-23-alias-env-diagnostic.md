# Implementation Plan: alias-env-diagnostic

## Overview

在 `src/shell.rs` 中扩展 `aliases.zsh` 生成逻辑：新增一个 zsh helper 函数 `__cc_switch_print_env`，由每个 `cl-*` alias 在 `export` 块之后、`command claude "$@"` 之前调用。该函数将 cc-switch-tui 可能注入的环境变量（除 credential 外）以紧凑单行格式打印到 stderr，用于排查 auto-compact 等配置问题。默认开启，可通过 `CC_SWITCH_QUIET=1` 关闭。

## Architecture Decisions

- **helper 内联在 `aliases.zsh` 中**：不引入额外文件，与用户现有 `source ~/.cc-switch-tui/aliases.zsh` 工作流兼容。
- **变量列表动态收集**：基于 `get_all_env_vars()` 的结果，保证模板新增 `default_env` / `model.env_overrides` 时自动覆盖。
- **credential redaction 用后缀匹配**：`_TOKEN` / `_API_KEY` / `_SECRET` / `_PASSWORD` 结尾的 key 值替换为 `<redacted>`，与 `web/src/lib/mask.ts` 的掩码规则保持一致。
- **输出发送到 stderr**：避免污染 Claude Code 的 stdout，不影响管道或编辑器集成。
- **默认开启、环境变量关闭**：不需要前端 UI 改动，用户可针对单次调用或整个 shell 会话关闭。

## Dependency Graph

```
src/shell.rs::get_all_env_vars()
    │
    ├── src/shell.rs::format_print_env_helper()   # 新增：生成 helper 函数定义
    │       │
    │       └── 依赖：变量列表 + redaction 规则
    │
    ├── src/shell.rs::format_function()           # 修改：在 cl-* 函数体中插入 helper 调用
    │       │
    │       └── 依赖：format_print_env_helper 已定义
    │
    └── src/shell.rs::tests                       # 修改/新增：验证 helper 与调用
```

实现顺序为 helper 定义 → helper 调用 → 测试。

## Task List

### Phase 1: Helper 函数生成

- [ ] **Task 1: 实现 `__cc_switch_print_env` helper 字符串生成**
  - **Description:** 在 `src/shell.rs` 中新增一个 Rust 函数（如 `format_print_env_helper(templates)`），返回 zsh helper 函数字符串。helper 接收 alias_name 参数，检查 `CC_SWITCH_QUIET`，遍历 `get_all_env_vars()` 返回的变量名，输出 `[cc-switch-tui] <alias>: KEY=VALUE ...` 到 stderr；credential 值替换为 `<redacted>`，未设置变量替换为 `<unset>`。
  - **Acceptance criteria:**
    - [ ] 生成的字符串包含 `function __cc_switch_print_env { ... }`。
    - [ ] helper 第一行检查 `[[ -n $CC_SWITCH_QUIET ]]` 并直接返回。
    - [ ] 使用 `${(P)k}` 获取变量值，支持 zsh 动态变量名。
    - [ ] credential 后缀变量显示为 `<redacted>`，未设置变量显示为 `<unset>`。
  - **Verification:**
    - [ ] `cargo test shell::` 通过（先写测试再实现）。
    - [ ] 手动检查生成内容：`cargo run` 后查看 `~/.cc-switch-tui/aliases.zsh`。
  - **Dependencies:** None
  - **Files likely touched:** `src/shell.rs`
  - **Estimated scope:** Small

### Phase 2: 在 cl-* 函数中调用 Helper

- [ ] **Task 2: 在 `format_function` 中插入 `__cc_switch_print_env` 调用**
  - **Description:** 修改 `format_function`，在生成的 `cl-*` 函数体中，将 `__cc_switch_print_env <alias>` 放在 `export` 块之后、`command claude "$@"` 之前。确保 `render_aliases()` 在生成任何 `cl-*` 函数之前先插入 helper 定义。
  - **Acceptance criteria:**
    - [ ] `aliases.zsh` 中 helper 定义出现在所有 `cl-*` 函数之前。
    - [ ] 每个 `cl-*` 函数体包含 `__cc_switch_print_env <alias>` 调用。
    - [ ] 调用位置在 `export` 块之后、`command claude` 之前。
  - **Verification:**
    - [ ] `cargo test shell::` 通过。
    - [ ] 手动 `source aliases.zsh && cl-mini --help`，stderr 出现诊断行。
  - **Dependencies:** Task 1
  - **Files likely touched:** `src/shell.rs`
  - **Estimated scope:** Small

### Phase 3: 测试覆盖

- [ ] **Task 3: 新增/更新单元测试验证诊断输出行为**
  - **Description:** 在 `src/shell.rs` 的测试模块中新增测试：验证 helper 函数存在、每个 `cl-*` 调用 helper、credential 被 redacted、未设置变量显示 `<unset>`、`CC_SWITCH_QUIET=1` 时 helper 不输出。同时确保现有测试不因新增调用而失败。
  - **Acceptance criteria:**
    - [ ] 新增测试覆盖 helper 生成与调用。
    - [ ] 新增测试覆盖 credential redaction。
    - [ ] 新增测试覆盖 `<unset>` 标记。
    - [ ] 现有 `shell::` 测试全部通过。
  - **Verification:**
    - [ ] `cargo test shell::` 全部通过。
    - [ ] `cargo test` 全量通过。
    - [ ] `cargo clippy` 无新增 warning。
  - **Dependencies:** Task 2
  - **Files likely touched:** `src/shell.rs`
  - **Estimated scope:** Small

## Checkpoint: After Phase 3

- [ ] `cargo test shell::` 全部通过
- [ ] `cargo test` 全量通过
- [ ] `cargo clippy` 无新增 warning
- [ ] 手动验证：`source ~/.cc-switch-tui/aliases.zsh && cl-mini --help` 能在 stderr 看到 `[cc-switch-tui] cl-mini: ...`
- [ ] 手动验证：`CC_SWITCH_QUIET=1 cl-mini --help` 不输出诊断行
- [ ] 在继续下一步（如前端 UI 开关）前需人工 review

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| zsh `${(P)k}` 在某些旧版本不兼容 | 中 | 项目已要求 `emulate zsh`，且 `${(P)...}` 是 zsh 标准参数扩展；如需兼容可在测试中用 zsh 实际执行验证 |
| 默认开启导致终端噪音过大 | 中 | 提供 `CC_SWITCH_QUIET=1` 即时关闭；若反馈负面可后续改为默认关闭 |
| credential redaction 规则遗漏 | 高 | 后缀列表与前端 `mask.ts` 保持一致；测试中显式断言 `ANTHROPIC_AUTH_TOKEN=<redacted>` |
| 变量顺序/格式变化破坏现有测试 | 低 | 现有测试只检查子串存在；新增测试独立，不依赖精确顺序 |

## Open Questions

- 是否需要把同样的诊断输出加到 `oc-*` alias？当前 spec 范围仅限 `cl-*`。
- 后续是否要在前端 Settings 增加持久化开关？当前通过 `CC_SWITCH_QUIET` 环境变量控制。
