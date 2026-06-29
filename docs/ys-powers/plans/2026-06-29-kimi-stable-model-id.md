# Implementation Plan: kimi-stable-model-id

## Overview

将 `src/templates.rs::kimi_template()` 中所有"model id"语义字段统一为 stable alias `kimi-for-coding`，覆盖 Claude Code 路径（`env_overrides` 注入）和 OpenCode 路径（`opencode_model_id` / `opencode_models`）。背景：Kimi 官方建议第三方工具统一使用 `kimi-for-coding` 作为请求体 model 字段，后端会映射到最新发布的模型；当前实现仍写死版本号 `k2p5`，未来 Kimi 切换版本时客户端配置会失效。

curl 实测已验证 `kimi-for-coding` 在 `https://api.kimi.com/coding/v1/messages` 路由层可接受、推理可用（HTTP 200，response.model 回显 `kimi-for-coding`）。

## Architecture Decisions

- **`env_overrides` 必须补齐 4 个 `ANTHROPIC_MODEL*`**：与 `CLAUDE.md` 硬约束一致（不要依赖 model 字段后缀，用 env 覆盖）。`kimi-for-coding` 不带 `[1m]` 后缀，但仍需注入以防 Claude Code 用默认 model 名发请求被 Kimi 后端拒识。
- **`opencode_model_id` 从 `k2p5` 改为 `kimi-for-coding`**：UI 下拉显示值与 OpenCode 配置文件实际写入值保持一致。
- **`opencode_models` 列表压成单元素 `["kimi-for-coding"]`**：原列表 `["k2p5", "k2p6", "kimi-k2-thinking"]` 暴露了 Kimi 内部版本号，用户选 `k2p5` 后 Kimi 升级就会失效；统一收敛到 stable alias。
- **`opencode_provider_id = "kimi-for-coding"` 保持不变**：这是 providers.dev 上的 provider key，与本次改动无关。
- **不动 `scripts/list-opencode-models.py`**：其中的 `kimi-for-coding` 是 provider key 而非 model id。

## Dependency Graph

```
tests/template_test.rs::test_kimi_template_registered
    │ (RED: update assertions to expect new state)
    │
    └── src/templates.rs::kimi_template()
            │ (GREEN: implement env_overrides + opencode_model_id + opencode_models)
```

实现顺序：先写失败的测试断言 → 改 templates → cargo test → make build 重建 aliases.zsh。

## Task List

### Phase 1: Tests

- [ ] **Task 1: 更新 `test_kimi_template_registered` 断言 + 新增 `opencode_model_id` / `opencode_models` 覆盖**
  - **Description:** 在 `tests/template_test.rs::test_kimi_template_registered` 中：
    1. 删除 `assert!(model.env_overrides.is_empty())`。
    2. 新增 4 个 `ANTHROPIC_MODEL*` env 断言（HAIKU/OPUS/SONNET/顶层 MODEL 都等于 `kimi-for-coding`），仿照同文件 MiniMax 断言写法。
    3. 新增 `assert_eq!(model.opencode_model_id, "kimi-for-coding")`。
    4. 新增 `assert_eq!(kimi.opencode_models, vec!["kimi-for-coding".to_string()])`。
  - **Acceptance criteria:**
    - [ ] 测试代码已落地，4 个 ANTHROPIC_MODEL* 断言完整。
    - [ ] 跑 `cargo test test_kimi_template_registered` 当前**失败**（因为 templates.rs 还没改），RED 状态确认。
  - **Verification:**
    - [ ] `cargo test test_kimi_template_registered` 输出 FAIL，且 panic 信息能看出是 `env_overrides` / `opencode_model_id` / `opencode_models` 维度失败。
  - **Dependencies:** None
  - **Files likely touched:** `tests/template_test.rs`
  - **Estimated scope:** Small

### Phase 2: Implementation

- [ ] **Task 2: 修改 `src/templates.rs::kimi_template()`**
  - **Description:** 在 `src/templates.rs::kimi_template()` 中：
    1. 新增局部 `env_overrides: HashMap<String, String>`，填 4 个 key 全部指向 `"kimi-for-coding"`。
    2. `models[0].env_overrides` 从 `HashMap::new()` 改为上面的 `env_overrides`。
    3. `models[0].opencode_model_id` 从 `"k2p5"` 改为 `"kimi-for-coding"`。
    4. `opencode_models` 从 `vec!["k2p5", "k2p6", "kimi-k2-thinking"]` 压成 `vec!["kimi-for-coding"]`。
  - **Acceptance criteria:**
    - [ ] `cargo test test_kimi_template_registered` 通过（GREEN）。
    - [ ] `cargo test` 全量通过，无 regression。
    - [ ] `cargo clippy -D warnings` 无新增 warning。
  - **Verification:**
    - [ ] `cargo test` 全量通过。
    - [ ] `cargo clippy -D warnings` 通过。
  - **Dependencies:** Task 1
  - **Files likely touched:** `src/templates.rs`
  - **Estimated scope:** Small

### Phase 3: Rebuild + Smoke

- [ ] **Task 3: 重建 `aliases.zsh` 并 smoke 验证**
  - **Description:** `make build` 重新生成 `~/.cc-switch-tui/aliases.zsh`，肉眼检查 `cl-km1` / `cl-km2` 函数体包含 `export ANTHROPIC_MODEL=kimi-for-coding` 等 4 行；旧 `~/.cc-switch-tui/opencode/cl-km*.json` 配置因 opencode_model_id 字段已变更，下次 apply 时会被覆盖（不需手工干预）。
  - **Acceptance criteria:**
    - [ ] `~/.cc-switch-tui/aliases.zsh` 中每个 `cl-km*` 函数体包含 `ANTHROPIC_MODEL=kimi-for-coding`、`ANTHROPIC_DEFAULT_HAIKU_MODEL=kimi-for-coding`、`ANTHROPIC_DEFAULT_OPUS_MODEL=kimi-for-coding`、`ANTHROPIC_DEFAULT_SONNET_MODEL=kimi-for-coding`。
    - [ ] `~/.cc-switch-tui/opencode/cl-km*.json` 重新生成时 `model` 字段等于 `kimi-for-coding`（如有相关 apply）。
  - **Verification:**
    - [ ] `make build` 成功。
    - [ ] `grep -E 'ANTHROPIC_MODEL=kimi-for-coding' ~/.cc-switch-tui/aliases.zsh` 有匹配。
  - **Dependencies:** Task 2
  - **Files likely touched:** `~/.cc-switch-tui/aliases.zsh`（运行时产物，不入 git）
  - **Estimated scope:** Small

## Checkpoint: After Phase 3

- [ ] `cargo test` 全量通过
- [ ] `cargo clippy -D warnings` 通过
- [ ] `make build` 成功
- [ ] `~/.cc-switch-tui/aliases.zsh` 中 `cl-km*` 函数体出现 4 行 `ANTHROPIC_*=kimi-for-coding`

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| 旧 `~/.cc-switch-tui/opencode/cl-km*.json` 写死了 `k2p5`，用户重新 apply 前 OpenCode 仍用旧值 | 低 | 下次 apply 自动覆盖；如有用户反馈再单独发 follow-up |
| `opencode_models` 列表压缩后前端 model 下拉只剩 1 项，UX 变化 | 低 | 这是预期行为；版本号不应该暴露给用户选择 |
| Kimi 后端某天把 `kimi-for-coding` 也弃用 | 极低 | 与所有 SaaS API 同风险；如有变动需要重新探索 stable alias，与本次改动解耦 |

## Open Questions

- 是否需要在 `CLAUDE.md` 或 `ARCHITECTURE.md` 补一条说明"为什么 Kimi 选 stable alias 而 MiniMax 用显式版本号"？本次 plan 不覆盖，留作后续 doc 任务。