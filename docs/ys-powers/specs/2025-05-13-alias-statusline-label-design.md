# Spec: Statusline Provider Alias Label

## 1. Objective

让 `cc-switch-tui` 生成的 alias 函数携带当前 alias 名称的环境变量 `CC_SWITCH_ALIAS`，使得 ccstatusline 可以通过 Custom Command Widget 读取并显示当前 provider alias（如 `cl-km2`、`cl-mini`）。

**目标用户**: 使用多个 Claude Code provider alias 的开发者，需要在 statusline 中一眼识别当前会话对应的 alias。

## 2. Core Features & Acceptance Criteria

- [ ] `generate_aliases` 生成的每个 alias 函数中，`unset` 行包含 `CC_SWITCH_ALIAS`
- [ ] `generate_aliases` 生成的每个 alias 函数中，`export` 行包含 `CC_SWITCH_ALIAS=<alias_name>`
- [ ] `CC_SWITCH_ALIAS` 的值等于 instance 的 alias 字段（如 `cl-km2`）
- [ ] 单元测试验证 `CC_SWITCH_ALIAS` 同时出现在 unset 列表和 export 行中
- [ ] 代码编译通过，全部现有测试通过
- [ ] 用户在 ccstatusline 中配置 `sh -c 'echo ${CC_SWITCH_ALIAS:-default}'` 后能在 statusline 看到 alias 名称

## 3. Tech Stack

- Rust (cc-switch-tui)
- ccstatusline (npm/bunx，不修改其源码)

## 4. Project Structure

只修改一个文件：

```
src/shell.rs
```

修改点：
1. `build_env`: 插入 `CC_SWITCH_ALIAS` 到 env HashMap
2. `get_all_env_vars`: 把 `CC_SWITCH_ALIAS` 加入 unset 列表

## 5. Code Style

- 保持现有 Rust 代码风格
- 不引入新依赖
- 最小改动，只加两行逻辑

## 6. Testing Strategy

- 现有单元测试 `test_generate_aliases_contains_unset_vars` 需扩展，验证 `CC_SWITCH_ALIAS` 出现在 unset 行
- 新增或扩展测试，验证 `CC_SWITCH_ALIAS=<alias>` 出现在 export 行
- `cargo test` 全部通过

## 7. Boundaries

**Always do:**
- 确保 `CC_SWITCH_ALIAS` 被 unset，避免 alias 切换时残留旧值

**Ask first about:**
- 是否需要自动配置 ccstatusline（本次不做，保持 scope 最小）

**Never do:**
- 不修改 ccstatusline 源码
- 不修改 Claude Code 配置
- 不改其他无关模块
