# Spec: 废除"选中"逻辑

## Objective

**问题**: 当前系统有"选中当前实例"的概念，生成的 `aliases.zsh` 会额外生成一个 `claude()` 函数。这增加了复杂度且用户不需要这个功能。

**目标**: 废除"选中"逻辑，只生成用户定义的 alias 函数，不再生成额外的 `claude()` 函数。

---

## 技术变更

### 1. `src/shell.rs` — `generate_aliases()`

**修改前**:
```rust
pub fn generate_aliases(
    dir: &std::path::Path,
    instances: &[ProviderInstance],
    templates: &[ProviderTemplate],
    current_instance_id: Option<&str>,  // ← 删除这个参数
)
```

**修改后**:
```rust
pub fn generate_aliases(
    dir: &std::path::Path,
    instances: &[ProviderInstance],
    templates: &[ProviderTemplate],
    // current_instance_id 参数删除
)
```

同时删除生成 `claude()` 函数的逻辑块。

### 2. 调用处更新

`src/app/state.rs` 中的调用需要更新，删除 `current_instance_id` 参数。

---

## 涉及文件

| 文件 | 变更 |
|------|------|
| `src/shell.rs` | 删除 `current_instance_id` 参数和 `claude()` 生成逻辑 |
| `src/app/state.rs` | 更新 `generate_aliases()` 调用 |

---

## 保留内容（不删除）

以下功能保留，未来可能需要：
- `Dao::get_current_instance()` / `Dao::set_current_instance()` 方法
- 数据库 `is_current` 字段
- TUI 中的切换高亮逻辑

---

## 验收标准

- [ ] `generate_aliases()` 不再接受 `current_instance_id` 参数
- [ ] 生成的 `aliases.zsh` 只包含用户定义的 alias 函数
- [ ] 不再生成 `claude()` 函数
- [ ] 测试用例通过

---

## Open Questions

~~1. 是否需要同时删除 DAO 中的 `get_current_instance` / `set_current_instance` 方法？~~ → **已确认：删除**

---

## 完整涉及文件

| 文件 | 变更 |
|------|------|
| `src/shell.rs` | 删除 `current_instance_id` 参数和 `claude()` 生成逻辑 |
| `src/app/state.rs` | 更新 `generate_aliases()` 调用，删除 `current_instance_id` 参数 |
| `src/dao/mod.rs` | 删除 `get_current_instance` / `set_current_instance` 方法声明 |
| `src/dao/memory_impl.rs` | 删除 `get_current_instance` / `set_current_instance` 实现 |
| `src/dao/sqlite_impl.rs` | 删除 `get_current_instance` / `set_current_instance` 实现和 `is_current` 字段相关逻辑 |
