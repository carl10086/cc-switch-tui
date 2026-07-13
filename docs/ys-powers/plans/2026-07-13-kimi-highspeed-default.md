# Plan: Kimi highspeed 作为默认 model

## Context

Kimi 后端新增 stable alias `kimi-for-coding-highspeed`（5–6× 输出速度、3× 额度、Allegretto+ 会员）。
当前 `src/templates.rs::kimi_template()` 只有 1 个 model（`kimi-for-coding`），需扩展为 2 个，
并把 highspeed 设为默认（列表首位）。

**不做**：ARCHITECTURE.md 更新、CLAUDE.md 更新、DB migration、前端代码改动、intent 文档。

## Task 1: 单测断言 highspeed model id 出现在 aliases.zsh（RED → GREEN）

**Acceptance criteria:**
- `src/shell.rs::tests` 中新增 `test_aliases_contain_kimi_for_coding_highspeed_model_id`
- 仿 `test_aliases_contain_minimax_m3_1m_model_id` 的最小形态：构造 Kimi instance（template_id="kimi",
  model_id="kimi-for-coding-highspeed"），调 `generate_aliases()`，断言生成的 `aliases.zsh` 含
  字符串 `"kimi-for-coding-highspeed"`
- `cargo test --lib shell::tests::test_aliases_contain_kimi_for_coding_highspeed_model_id` 在实现前
  fail（因当前 template 不含 highspeed），实现后 pass

**Implementation hint:** 在 `src/shell.rs` `#[cfg(test)] mod tests` 里参照 M3[1m] 测试样板。

**Dependencies:** 无（可独立 RED 步）

## Task 2: 扩展 `kimi_template()` 加入 highspeed（GREEN）

**Acceptance criteria:**
- `src/templates.rs::kimi_template()` 的 `models: vec![…]` 从 1 项扩到 2 项：
  - index 0（默认）：`ModelTemplate { id: "kimi-for-coding-highspeed", name: "Kimi for Coding · Highspeed", env_overrides: 指向 highspeed 的 4 个 ANTHROPIC_*_MODEL, opencode_model_id: "kimi-for-coding-highspeed" }`
  - index 1：`ModelTemplate { id: "kimi-for-coding", name: "Kimi for Coding", env_overrides: 指向 normal 的 4 个 ANTHROPIC_*_MODEL, opencode_model_id: "kimi-for-coding" }`
- `opencode_models: vec!["kimi-for-coding-highspeed".to_string(), "kimi-for-coding".to_string()]`
- 其余字段（id/name/default_env/opencode_provider_id/opencode_npm/opencode_base_url/opencode_env_var）**全部不动**
- 不注入 `CLAUDE_CODE_AUTO_COMPACT_WINDOW`（Kimi 无 1M context 概念）
- `cargo test` 全绿；`cargo build` 通过

**Dependencies:** Task 1（GREEN 阶段）