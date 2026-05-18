# KV Cache 优化功能实现计划

## Overview

为每个 provider 实例增加 KV Cache 优化开关，使 `cl-xxx` alias 在启用时追加 `--exclude-dynamic-system-prompt-sections` 和 `--settings '{"includeGitInstructions":false}'` 两个参数。

## 依赖图

```
instance.rs (数据模型)
    │
    ├── sqlite_impl.rs (数据库)
    │       │
    │       └── dao/mod.rs (Dao trait 可能需要新增方法)
    │
    └── shell.rs (alias 生成)
            │
            └── ui/ (UI 编辑入口)
```

## Task List

### Phase 1: 数据模型

- [ ] **Task 1**: 在 `src/domain/instance.rs` 的 `ProviderInstance` 结构体中增加 `kv_cache_enabled: bool` 字段

**Acceptance criteria:**
- `ProviderInstance` 包含 `kv_cache_enabled` 字段，默认 `false`

**Verification:**
- `cargo check` 通过

**Dependencies:** None

**Files:** `src/domain/instance.rs`

---

- [ ] **Task 2**: 在 `src/dao/sqlite_impl.rs` 中：
  - 表创建时增加 `kv_cache_enabled INTEGER DEFAULT 0`
  - 兼容旧表：`ALTER TABLE` 添加列（参考现有 `alias`、`opencode_model_id` 迁移模式）
  - `refresh_instances()` 查询该字段
  - `rename_instance()` 在 INSERT/DELETE 时处理该字段

**Acceptance criteria:**
- 新数据库实例该字段默认为 0
- 旧数据库能正常迁移（列不存在时自动添加）
- 查询和写入该字段正确

**Verification:**
- `cargo test` 中 dao 相关测试通过

**Dependencies:** Task 1

**Files:** `src/dao/sqlite_impl.rs`

---

- [ ] **Task 3**: 在 `src/dao/mod.rs` 的 `Dao` trait 中增加 `set_kv_cache_enabled` 方法

**Acceptance criteria:**
- `Dao` trait 定义 `set_kv_cache_enabled(&mut self, id: &str, enabled: bool) -> Result<(), AppError>`
- `MemoryDaoImpl` 实现该方法（可选，如果需要内存实现）

**Verification:**
- `cargo check` 通过

**Dependencies:** Task 1

**Files:** `src/dao/mod.rs`, `src/dao/memory_impl.rs`

---

### Checkpoint: Phase 1
- [ ] `cargo check` 和 `cargo test` 通过
- [ ] 数据库迁移逻辑正确处理新字段

---

### Phase 2: Alias 生成

- [ ] **Task 4**: 修改 `src/shell.rs` 的 `format_function` 函数或新建辅助函数，当 `kv_cache_enabled = true` 时追加两个参数

**Acceptance criteria:**
- 当 `kv_cache_enabled = true` 时，`command claude "$@"` 变为：
  ```
  command claude --exclude-dynamic-system-prompt-sections --settings '{"includeGitInstructions":false}' "$@"
  ```
- 当 `kv_cache_enabled = false` 时，行为与现在完全一致

**Verification:**
- `cargo test shell::tests` 通过
- 手动检查生成的 `~/.cc-switch-tui/aliases.zsh` 内容正确

**Dependencies:** Task 1, Task 2

**Files:** `src/shell.rs`

---

### Checkpoint: Phase 2
- [ ] alias 生成逻辑正确
- [ ] 两种状态（开启/关闭）都测试通过

---

### Phase 3: UI

- [ ] **Task 5**: 在 `EditInfoPanel` 中增加 KV Cache 开关字段

**具体变更：**
1. `src/app/state.rs`:
   - `EditField` enum 增加 `KvCacheEnabled`
   - `handle_edit_info_panel` 的 `max_index` 从 2 改为 3
   - `handle_edit_info_panel` 的 Enter 处理：focus_index=2 是 OpenCode Model（现有），focus_index=3 是 KV Cache
   - `handle_edit_field` 处理 `EditField::KvCacheEnabled` 的 Enter 切换布尔值
   - 新增 `submit_kv_cache_toggle` 方法调用 `dao.set_kv_cache_enabled`

2. `src/ui/list.rs`:
   - `draw_info_panel` 在 OpenCode Model 字段后增加 KV Cache checkbox 行
   - checkbox 显示 `[x]` 或 `[ ]` 根据 `kv_cache_enabled` 值
   - 高亮时按 Enter 切换值

**Acceptance criteria:**
- 编辑界面显示 KV Cache 开关
- 高亮该行时按 Enter 可以切换开启/关闭
- 切换后调用 `set_kv_cache_enabled` 并重新生成 alias

**Verification:**
- 手动测试：编辑实例 → 高亮 KV Cache 行 → Enter 切换 → 验证 aliases.zsh 内容变化

**Dependencies:** Task 3, Task 4

**Files:** `src/app/state.rs`, `src/ui/list.rs`

---

### Checkpoint: Phase 3
- [ ] UI 可以正常编辑 KV Cache 开关
- [ ] 开启后 aliases.zsh 包含额外参数
- [ ] 关闭后 aliases.zsh 不包含额外参数

---

### Phase 4: 测试和清理

- [ ] **Task 6**: 更新 `sqlite_impl.rs` 中的测试，添加 `kv_cache_enabled` 字段到测试实例

**Acceptance criteria:**
- 所有现有测试通过
- 新增 `kv_cache_enabled` 字段的测试覆盖

**Verification:**
- `cargo test` 全部通过

**Dependencies:** Task 2

**Files:** `src/dao/sqlite_impl.rs`, `src/shell.rs`

---

### Checkpoint: Phase 4
- [ ] 所有测试通过
- [ ] 代码无编译警告

---

## 风险和缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| UI 编辑状态管理复杂 | 中 | 先完成数据模型和 alias 生成，UI 最后做 |
| 数据库迁移遗漏旧数据 | 低 | 参考现有 alias/opencode_model_id 迁移模式 |

## Open Questions

- [ ] 是否需要在新建实例向导中也提供 KV Cache 选项？（当前设计：仅在编辑时设置，默认关闭）
