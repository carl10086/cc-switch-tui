# Spec: 修复 OpenCode Model 下拉框列表跳动问题

## Objective

修复 TUI 中 OpenCode Model 选择下拉框的列表项在每一帧渲染时顺序不稳定、导致视觉上"反复跳动"的问题，使用户能够正常用 Up/Down 键选中目标 model。

## Background

`App::get_opencode_models_for_provider_id()` 每次被调用时都会新建一个 `HashSet<String>` 来去重合并模板硬编码的 model 和从 API 拉取缓存的 model。由于 Rust 的 `HashSet` 使用 `RandomState`（SipHash + 随机种子），**每次新创建的 `HashSet` 内部迭代顺序可能不同**。而 `draw_opencode_model_select` 在每一帧渲染时都重新调用该方法，导致列表项顺序在视觉上持续跳动。

## Acceptance Criteria

- [ ] OpenCode Model 选择列表（新建向导和编辑模式）在渲染时顺序稳定，不再跳动。
- [ ] 列表项按字母升序排列，便于用户快速定位。
- [ ] Up/Down 键选择高亮项行为正常，选中后 Enter 能正确提交对应的 model ID。

## Commands

```bash
# 本地运行验证修复效果
cargo run

# 运行单元测试确保无回归
cargo test

# 代码格式检查
cargo fmt -- --check

# Clippy 静态检查
cargo clippy
```

## Project Structure

本次改动仅涉及单个文件中的单个方法：

```
src/
  app/
    state.rs          # 修改 get_opencode_models_for_provider_id() 方法
```

## Code Style

- 保持现有代码格式，不引入额外风格变化。
- 最小改动原则：仅在最内层返回前增加排序逻辑，不动其他代码。

## Testing Strategy

1. **手动验证**（主要）：
   - `cargo run` 启动应用
   - 按 `n` 进入新建向导，选择 Provider，进入 OpenCode Model 选择页
   - 观察列表是否稳定、按字母序排列
   - 用 Up/Down 选择不同项，确认高亮位置正确且不回跳
   - 进入编辑模式（`e` → 选择 OpenCode Model），同样验证稳定性

2. **回归测试**：
   - `cargo test` 全量通过，确保 `AppState` 相关测试无回归。

## Boundaries

### Always Do
- 对 `get_opencode_models_for_provider_id` 的返回结果做稳定排序。
- 保持去重逻辑不变（`HashSet` 仍可用来去重，只是在 `into_iter().collect()` 后排序）。

### Ask First About
- 如需调整排序规则（如按业务优先级而非字母序），需重新讨论。

### Never Do
- 不引入新的外部依赖（如 `indexmap`、`itertools` 等）。
- 不修改 TUI 渲染逻辑或键盘事件处理逻辑。
- 不改其他无关代码。
