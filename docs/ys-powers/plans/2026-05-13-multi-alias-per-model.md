# Implementation Plan: 允许多别名 per Model

## Overview

将实例 ID 格式从 `{template_id}-{model_id}` 改为 `{template_id}-{model_id}-{alias}`，允许同一 model 通过不同 alias 创建多个实例。同时提供一次性迁移脚本处理旧数据。

## 架构决策

| 决策 | 理由 |
|------|------|
| ID 包含 alias | alias 唯一则 ID 唯一，实现简单 |
| alias 变更时同步更新 ID | 保持 ID 与 alias 一致性 |
| 迁移脚本独立 binary | 减少主程序复杂度，支持 dry-run |
| 迁移脚本用户手动执行 | 避免自动迁移风险 |

---

## Task List

### Phase 1: DAO 层（Foundation）

#### Task 1: 新增 `rename_instance` DAO 方法

**Description:** 在 Dao trait 中新增 `rename_instance` 方法，同时更新实例的 id 和 alias。

**Acceptance criteria:**
- [ ] `Dao` trait 新增 `rename_instance(&mut self, old_id: &str, new_id: &str, alias: String) -> Result<(), AppError>`
- [ ] `MemoryDaoImpl` 实现该方法（删除旧记录，插入新记录）
- [ ] `SqliteDaoImpl` 实现该方法（DELETE + INSERT，或 UPDATE id 和 alias）
- [ ] 新 id 已存在时返回 `InstanceAlreadyExists` 错误

**Verification:**
- [ ] `cargo test -- dao` 相关测试通过

**Dependencies:** None

**Files likely touched:**
- `src/dao/mod.rs`
- `src/dao/memory_impl.rs`
- `src/dao/sqlite_impl.rs`

**Estimated scope:** M

---

### Phase 2: 核心逻辑变更

#### Task 2: 修改 `submit_create()` ID 生成逻辑

**Description:** 创建实例时，使用包含 alias 的新 ID 格式。

**Acceptance criteria:**
- [ ] 创建实例时 id = `{template_id}-{model_id}-{alias}`
- [ ] 同一 model + 不同 alias 可创建多个实例

**Verification:**
- [ ] `cargo test` 通过
- [ ] 可在 TUI 中创建同一 model 的多个实例

**Dependencies:** Task 1

**Files likely touched:**
- `src/app/state.rs`

**Estimated scope:** S

---

#### Task 3: 修改 `handle_edit_field()` alias 变更逻辑

**Description:** 用户编辑 alias 时，同步更新 id。

**Acceptance criteria:**
- [ ] alias 变更时调用 `dao.rename_instance()`
- [ ] 更新 `current_instance_id` 指向新 id（如果当前选中实例被重命名）

**Verification:**
- [ ] 在 TUI 中编辑已有实例的 alias，验证 id 同步更新

**Dependencies:** Task 2

**Files likely touched:**
- `src/app/state.rs`

**Estimated scope:** S

---

#### Task 4: 修改 `draw_instance_list()` 遍历逻辑

**Description:** UI 列表渲染改为遍历所有实例而非按固定 id 格式查找。

**Acceptance criteria:**
- [ ] 列表正确显示所有实例
- [ ] 列表按 template 分组显示

**Verification:**
- [ ] 启动 TUI，验证实例列表正确显示

**Dependencies:** Task 2

**Files likely touched:**
- `src/ui/list.rs`

**Estimated scope:** S

---

### Phase 3: 测试与脚本

#### Task 5: 更新测试用例 ID 格式

**Description:** 更新 `shell.rs` 中测试用例的 id 为新格式。

**Acceptance criteria:**
- [ ] `test_generate_aliases_content` 使用新 ID 格式
- [ ] `test_create_instance_duplicate` 验证逻辑正确（相同 alias 重复创建被拒绝）

**Verification:**
- [ ] `cargo test -- shell` 通过

**Dependencies:** Task 2

**Files likely touched:**
- `src/shell.rs`

**Estimated scope:** XS

---

#### Task 6: 新增迁移脚本

**Description:** 创建独立 binary `migrate_instances_id`，处理旧数据迁移。

**Acceptance criteria:**
- [ ] 支持 `--dry-run` 参数（只打印 SQL，不执行）
- [ ] 支持 `--backup` 参数（默认开启）
- [ ] 支持 `--db-path` 参数
- [ ] 扫描 alias 非空的记录，计算新 id 并更新
- [ ] 检测冲突：新 id 已存在时报错退出
- [ ] alias 为空的记录保持不变

**Verification:**
- [ ] 在备份数据库上测试 dry-run
- [ ] 验证迁移后数据完整性

**Dependencies:** Task 1（需要使用 rusqlite）

**Files likely touched:**
- `tools/migrate_instances_id.rs` (新增)
- `Cargo.toml` (新增 binary)

**Estimated scope:** M

---

### Checkpoint: 核心功能完成

- [ ] `cargo test` 全部通过
- [ ] 可在 TUI 中创建同一 model 的多个实例
- [ ] 迁移脚本 dry-run 测试正常
- [ ] 代码可编译，无警告

---

### Phase 4: 集成验证

#### Task 7: 手动集成测试

**Description:** 在真实数据上验证完整流程。

**Acceptance criteria:**
- [ ] 备份当前数据库
- [ ] 运行迁移脚本
- [ ] 启动 TUI，验证所有实例正常显示
- [ ] 验证 alias 切换功能正常
- [ ] 验证 alias 编辑后 id 同步更新

**Verification:**
- [ ] 所有实例 alias 命令正确生成
- [ ] `claude` 命令切换正常

**Dependencies:** Task 6

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| 迁移脚本执行失败 | 高 | 默认开启备份，可回滚 |
| 迁移时新 id 冲突 | 中 | 检测到冲突时报错退出，不执行 |
| alias 编辑时 id 更新导致 current_instance 丢失 | 低 | 更新 current_instance_id 指向新 id |

---

## Open Questions

~~1. 迁移时机~~ → 已确认：用户手动运行迁移脚本

~~2. alias 变更时 ID 更新~~ → 已确认：alias 变更时 id 随之改变

---

## 执行顺序

```
Task 1 → Task 2 → Task 3, Task 4, Task 5 (可并行)
                      ↓
                  Task 6 (独立)
                      ↓
               Checkpoint: 核心功能完成
                      ↓
                  Task 7 (手动集成测试)
```
