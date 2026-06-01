# Spec: 添加 MiniMax-M3 模型支持

## Objective

为 cc-switch-tui 添加 MiniMax-M3 模型支持，并将其设为 MiniMax provider 的默认模型。

M3 是 MiniMax 于 2026-05-31 发布的原生多模态基础模型，支持 1M token 上下文窗口。通过 Anthropic API 兼容接口可直接调用（已通过 `curl` 验证）。

**用户故事：**
- 作为用户，我可以在 cc-switch-tui 中选择 MiniMax-M3 模型
- 作为用户，切换 MiniMax provider 时默认使用 M3 模型
- 作为用户，使用 M3 时能够充分利用其 1M 上下文窗口能力

**成功标准：**
- [ ] `cc-switch-tui` 启动后 MiniMax provider 包含 `MiniMax-M3` 和 `MiniMax-M2.7-highspeed` 两个模型
- [ ] 新建 MiniMax 实例时默认选择 `MiniMax-M3`
- [ ] 生成的 alias 环境变量中 `ANTHROPIC_MODEL` 为 `MiniMax-M3`
- [ ] 生成的 opencode 配置中 `model` 为 `minimax-cn/MiniMax-M3`
- [ ] 所有测试通过

## Tech Stack

- **语言**: Rust (edition 2021)
- **构建工具**: Cargo
- **测试框架**: 内置 `cargo test`
- **依赖**: `serde`, `serde_json`, `chrono`, `rusqlite`, `crossterm`, `ratatui`, `ureq`

## Commands

```bash
# 构建
cargo build --release

# 测试
cargo test

# 格式化
cargo fmt

# 静态检查
cargo clippy
```

## Project Structure

```
src/
  app/
    templates.rs      # Provider 模板定义（本次主要改动）
    state.rs          # 应用状态管理（测试数据需同步）
  shell.rs            # Shell alias 生成（测试数据需同步）
  opencode_config.rs  # OpenCode 配置生成（自动跟随模板）
  opencode_fetch.rs   # OpenCode 模型拉取（无需改动）
  dao/
    sqlite_impl.rs    # DAO 实现（测试数据需同步）
tests/
  template_test.rs    # 模板测试（断言需同步）
  bug_repro_get_sorted.rs  # 排序测试（测试数据需同步）
docs/ys-powers/specs/  # 设计文档
```

## Code Style

遵循现有 Rust 代码风格：

```rust
fn minimax_template() -> ProviderTemplate {
    let mut default_env = HashMap::new();
    default_env.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        "https://api.minimaxi.com/anthropic".to_string(),
    );
    // ...
    ProviderTemplate {
        id: "minimax".to_string(),
        name: "MiniMax".to_string(),
        // ...
    }
}
```

- 使用 `to_string()` 而非 `String::from()` 或 `"...".into()`
- HashMap 插入使用显式的 `to_string()`
- 模型 ID 使用原始字符串（如 `"MiniMax-M3"`）

## Testing Strategy

- **单元测试**: `cargo test`，覆盖模板生成、alias 生成、DAO 操作
- **集成测试**: `tests/` 目录下的测试文件
- **验证方式**: 修改后运行 `cargo test` 确保全部通过
- **覆盖率要求**: 所有与 minimax 相关的测试断言必须更新以反映新的模型列表

## Boundaries

- **Always**:
  - 运行 `cargo test` 通过后再提交
  - 保持模板结构一致性（`ProviderTemplate` 字段完整）
  - 模型 ID 与 API 返回严格一致（大小写敏感）

- **Ask first**:
  - 修改 `ProviderTemplate` 结构体（添加/删除字段）
  - 引入新的依赖 crate
  - 修改 CI/CD 配置

- **Never**:
  - 删除旧模型 `MiniMax-M2.7-highspeed`（保留向后兼容）
  - 修改与 MiniMax 无关的 provider（如 Kimi）
  - 提交 API key 或敏感信息

## Open Questions

1. **1M 上下文环境变量配置**：根据 CLAUDE.md，`DISABLE_COMPACT=1` + `CLAUDE_CODE_MAX_CONTEXT_TOKENS=1000000` 可启用完整 1M 上下文。但这些变量是 provider-scoped，会影响 MiniMax 下所有模型（包括 M2.7 highspeed）。是否只在 M3 的 `env_overrides` 中设置，还是在 provider 级别统一设置？
   → **决策**：本次不在模板中硬编码 `DISABLE_COMPACT` 和 `CLAUDE_CODE_MAX_CONTEXT_TOKENS`，由用户自行在需要时配置。避免影响 M2.7 highspeed 用户的默认行为。

2. **默认模型排序**：M3 应该排在 M2.7 highspeed 之前（作为默认），这种排序是否符合用户预期？
   → **决策**：M3 排在第一位，M2.7 highspeed 排在第二位。

---

# Implementation Plan

## Phase 1: 模板核心修改

**文件**: `src/app/templates.rs`

修改 `minimax_template()`：
1. 在 `models` 列表中添加 `MiniMax-M3`（排在第一位）
2. 保留 `MiniMax-M2.7-highspeed`（排在第二位）
3. 将 `ANTHROPIC_DEFAULT_HAIKU_MODEL`、`ANTHROPIC_DEFAULT_OPUS_MODEL`、`ANTHROPIC_DEFAULT_SONNET_MODEL` 从 `MiniMax-M2.7-highspeed` 改为 `MiniMax-M3`
4. M3 的 `env_overrides` 中设置 `ANTHROPIC_MODEL` = `MiniMax-M3`
5. M2.7 highspeed 的 `env_overrides` 中设置 `ANTHROPIC_MODEL` = `MiniMax-M2.7-highspeed`

## Phase 2: 测试同步更新

按依赖顺序更新以下文件中的硬编码模型引用：

1. `tests/template_test.rs`:
   - `models.len()` 从 `1` 改为 `2`
   - 添加对 M3 的验证断言

2. `src/shell.rs`（测试代码）:
   - 所有硬编码 `MiniMax-M2.7-highspeed` 改为 `MiniMax-M3`
   - 测试中的 `ModelTemplate` 和 `ProviderInstance` 数据同步

3. `src/app/state.rs`（测试代码）:
   - 测试数据中的模型 ID 同步更新

4. `src/dao/sqlite_impl.rs`（测试代码）:
   - 测试数据中的 `minimax-MiniMax-M2.7-highspeed` 实例 ID 和模型 ID 同步

5. `tests/bug_repro_get_sorted.rs`:
   - 测试数据中的模型信息同步

## Phase 3: 验证

1. 运行 `cargo test` 确保全部通过
2. 运行 `cargo build --release` 确保编译无警告
3. 手动检查生成的 alias 和 opencode 配置格式正确

## Task Breakdown

- [ ] Task 1: 修改 `src/app/templates.rs` 添加 M3 并设为默认
  - Acceptance: `minimax_template()` 返回包含 M3 和 M2.7 highspeed 两个模型，默认环境变量指向 M3
  - Verify: `cargo test template_test` 通过（需先更新测试）
  - Files: `src/app/templates.rs`

- [ ] Task 2: 更新 `tests/template_test.rs` 断言
  - Acceptance: 测试通过，验证 M3 为第一个模型
  - Verify: `cargo test --test template_test`
  - Files: `tests/template_test.rs`

- [ ] Task 3: 更新 `src/shell.rs` 测试数据
  - Acceptance: shell 相关测试全部通过
  - Verify: `cargo test shell`
  - Files: `src/shell.rs`

- [ ] Task 4: 更新 `src/app/state.rs` 测试数据
  - Acceptance: state 相关测试全部通过
  - Verify: `cargo test state`
  - Files: `src/app/state.rs`

- [ ] Task 5: 更新 `src/dao/sqlite_impl.rs` 测试数据
  - Acceptance: DAO 相关测试全部通过
  - Verify: `cargo test dao`
  - Files: `src/dao/sqlite_impl.rs`

- [ ] Task 6: 更新 `tests/bug_repro_get_sorted.rs` 测试数据
  - Acceptance: 排序测试通过
  - Verify: `cargo test --test bug_repro_get_sorted`
  - Files: `tests/bug_repro_get_sorted.rs`

- [ ] Task 7: 全量测试验证
  - Acceptance: `cargo test` 全部通过，无编译警告
  - Verify: `cargo test` && `cargo clippy`
  - Files: 无（验证步骤）
