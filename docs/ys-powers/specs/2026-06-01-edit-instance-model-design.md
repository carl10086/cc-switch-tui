# Spec: 支持编辑已存在 instance 的 model

## Objective

让用户能够在不删除重建的情况下，修改已存在 ProviderInstance 的 `model_id`，并保证 instance.id、shell alias 文件、opencode 配置都随之正确更新。

**用户故事：**
- 作为用户，我在 List 视图选中已配置的 instance，按 `e` 进入编辑后，能直接修改 model
- 作为用户，修改 model 后 alias 名（`cl-mini`）保持不变，shell alias 文件自动重新生成
- 作为用户，跨 model 切换时所有依赖该 instance 的配置（claude code env、opencode.json、shell alias）一并更新
- 作为用户，老版本 v0.2.x 写入的数据（id 含 model 段）能自动迁移到新 id 格式，无需手动操作

**成功标准：**
- [ ] EditInfoPanel 出现新的 "Model" 字段，可列出当前 template 下的所有 model
- [ ] 修改 model 后 `Enter` 保存，instance 立刻反映新 model，DB 持久化
- [ ] shell alias 文件重新生成，alias 名（`cl-mini`）不变，绑定的 provider/model 更新
- [ ] opencode config 中的 `model` 字段更新为新 model 的 `opencode_model_id`（仅在原 `opencode_model_id` 为空时）
- [ ] 启动时检测旧 id 格式（`{template_id}-{model_id}-{alias}`），自动迁移到 `{template_id}-{alias}` 格式
- [ ] 迁移期间数据（api_key、opencode_model_id、kv_cache、created_at）零丢失
- [ ] 同 template 下 alias 重复时创建/编辑都被拒绝（DB UNIQUE 约束）
- [ ] 所有原有测试通过 + 新增的 id 解耦与迁移测试通过

## Tech Stack

- **语言**: Rust (edition 2021)
- **构建工具**: Cargo
- **存储**: SQLite（`src/dao/sqlite_impl.rs`）+ 内存实现（`src/dao/memory_impl.rs`）
- **依赖**: `serde`, `serde_json`, `chrono`, `rusqlite`, `crossterm`, `ratatui`, `ureq`（无新增依赖）

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
    templates.rs       # Provider/Model 模板定义（无需改动）
    state.rs           # 状态机（EditInfoPanel 新增 Model 选项）
  dao/
    mod.rs             # DAO trait（update_instance 扩展接受 model_id；alias 校验）
    sqlite_impl.rs     # SQLite 实现（id 格式变更 + alias 校验 + UNIQUE 约束）
    memory_impl.rs     # 内存实现（id 格式变更 + alias 校验）
  domain/
    error.rs           # 启用 InvalidAlias 校验
    instance.rs        # 更新 id 文档注释为新格式
  shell.rs             # shell alias 生成（id 来源改为 {template_id}-{alias}）
  opencode_config.rs   # opencode config 生成（无需大改，跟随 instance）
  ui/
    edit.rs            # EditInfoPanel 渲染（新增 Model 行）
    list.rs            # List 视图（无需改动）
src/bin/
  migrate.rs           (新增)  # 独立迁移工具：export → verify → truncate → import
tests/
  template_test.rs     # 模板断言（无需改动）
  edit_instance_test.rs (新增)  # 编辑 model 的端到端测试
  migration_test.rs    (新增)    # 旧 id 格式迁移测试
```

## Code Style

遵循现有 Rust 风格（`src/app/templates.rs` 是参考）：

```rust
// DAO trait 扩展示例
fn update_instance(
    &mut self,
    old_id: &str,
    new_model_id: String,
    new_alias: String,
) -> Result<(), AppError>;
```

- 使用 `to_string()` 而非 `String::from()`
- 错误处理用 `Result<T, AppError>` 链式传播
- 字符串拼接优先 `format!`，避免反复 `+`
- 新增模块顶部加一行 `///` 文档注释说明用途

## Core Design（方案 A：id 解耦）

### 核心变化

| 字段 | 旧格式 | 新格式 |
|------|--------|--------|
| `instance.id` | `{template_id}-{model_id}-{alias}` | `{template_id}-{alias}` |
| 主键稳定性 | ❌ 改 model 必变 | ✅ 改 model 不变 |
| `(template_id, alias)` 唯一性 | ❌ 无约束 | ✅ DB UNIQUE 约束 |

### id 拼接逻辑

`src/app/state.rs` 中的 id 构造点（当前 `state.rs:530`）改为：

```rust
let id = format!("{}-{}", template_id, alias);
```

所有依赖 `instance.id` 的下游（`shell.rs` 的 alias 生成、`opencode_config.rs` 的 provider 路径）自动跟随。

### EditInfoPanel 新结构

```
┌─ 编辑 instance ─────────────────────┐
│ > Model:    MiniMax M3              │  ← 新增（focus_index = 1）
│   Alias:    cl-mini                 │  ← 移位（focus_index = 2）
│   API Key:  sk-***...               │  ← 移位（focus_index = 3）
│   KV Cache: [ ]                      │  ← 移位（focus_index = 4）
│   [Delete]                          │  ← 移位（focus_index = 5）
│   [Save]  [Cancel]                  │
└─────────────────────────────────────┘
```

按 `Tab` 切换 focus，按 `Enter` 选中/编辑。Model 字段按 `j/k` 上下选。

### 修改 model 的事务

DAO 层提供新方法：

```rust
pub trait Dao {
    // 旧：仅改 api_key
    // fn update_instance(&mut self, id: &str, api_key: String) -> Result<(), AppError>;
    
    // 新：原子改 model + alias
    fn update_instance(
        &mut self,
        old_id: &str,
        new_model_id: String,
        new_alias: String,
        api_key: String,
    ) -> Result<(), AppError>;
}
```

**实现策略（SQLite）：**
1. `BEGIN TRANSACTION`
2. 校验 `(new_template_id, new_alias)` 不与任何其他 instance 冲突
3. `UPDATE providers_instance SET model_id=?, alias=?, api_key=? WHERE id=?`
4. `UPDATE providers_instance SET id=? WHERE id=?` （重写 id）
5. `COMMIT`

注：因为新 id 格式下 model_id 不在 id 里，所以"改 model"可能根本**不需要改 id**——除非 alias 也同时改了。`update_instance` 接收 alias 参数就是为了支持"同步改 alias"。

### opencode 联动策略

按用户决策："保持当前逻辑，不主动改 opencode_model_id"。

- 若 `instance.opencode_model_id` 为空 → 按新 model 的 `ModelTemplate.opencode_model_id` 自动填入（与创建流程一致）
- 若 `instance.opencode_model_id` 已设置 → 保留用户原值，不动

实现位置：`src/opencode_config.rs:33-43` 已有 fallback 逻辑，复用即可。

## 数据迁移策略

### 核心理念

迁移是**离线、可重复、可验证**的操作。运行一个独立的 `migrate` 子命令，按 4 步原子化执行：

```
export  →  verify  →  truncate  →  import
```

- **不在线改 DB**：应用启动时不做任何迁移，DAO 代码保持纯净
- **不重复**：每次 export 输出同样的内容；import 前 truncate 保证目标表无残留
- **关键数据零丢失**：verify 阶段对比关键字段（id、template_id、model_id、api_key、alias、opencode_model_id、kv_cache_enabled、created_at），不一致立即终止
- **失败可回滚**：truncate 在 import 验证通过后才执行；中途失败时旧数据完整保留

### 工具位置

`src/bin/migrate.rs` —— Cargo 多 binary 机制，无需引入 clap/structopt。简单 argv 解析即可：

```
$ cargo run --bin migrate
  --db <path>          # 默认 .cc-switch-tui/db.sqlite
  --export <out.json>  # 导出到此文件
  --run                # 实际执行 export → verify → truncate → import
  --dry-run            # 只跑 export + verify，不动 DB
```

子命令行为：
- `migrate --export` — 导出当前所有 instance 为 JSON
- `migrate --run` — 完整流程（内部也做 export）
- `migrate --dry-run` — 导出 + 校验，不动 DB（用于预览）

### 导出格式（v1）

```json
{
  "version": 1,
  "exported_at": "2026-06-01T12:00:00Z",
  "source_schema": "{template_id}-{model_id}-{alias}",
  "target_schema": "{template_id}-{alias}",
  "instances": [
    {
      "old_id": "minimax-MiniMax-M2.7-cl-mini",
      "new_id": "minimax-cl-mini",
      "template_id": "minimax",
      "model_id": "MiniMax-M2.7-highspeed",
      "api_key": "sk-***...",
      "alias": "cl-mini",
      "opencode_model_id": "MiniMax-M2.7-highspeed",
      "kv_cache_enabled": false,
      "created_at": "2026-05-20T08:30:00Z"
    }
  ]
}
```

- `version` 字段：未来 schema 演化时按版本走转换路径
- `old_id` + `new_id` 双记录：verify 阶段直接对比，确保不丢映射
- **关键：导出期间 DB 是只读快照**，用 `BEGIN IMMEDIATE` 事务保证一致性

### 转换逻辑

```rust
fn transform_id(old_id: &str, template_id: &str) -> Result<String, AppError> {
    let prefix = format!("{}-", template_id);
    if !old_id.starts_with(&prefix) {
        return Err(AppError::MigrationError(format!(
            "id {} 不以 {} 开头", old_id, template_id
        )));
    }
    let suffix = &old_id[prefix.len()..];
    let segments: Vec<&str> = suffix.splitn(2, '-').collect();
    match segments.len() {
        2 => {
            // 旧格式：{model_id}-{alias}
            Ok(format!("{}{}", prefix, segments[1]))
        }
        1 => {
            // 已经是新格式
            Ok(old_id.to_string())
        }
        _ => unreachable!(),
    }
}
```

### 验证阶段

`verify` 阶段不依赖 DB，纯粹在 JSON 内部做：

1. 关键字段（api_key, alias, opencode_model_id, created_at, kv_cache_enabled, template_id, model_id）从 `instances[]` 提取，按 `(template_id, alias)` 排序后哈希
2. 同一份 JSON 跑两次 verify → 哈希必须一致（保证输出确定）
3. `new_id` 全局唯一性检查（在所有 instances 内）
4. `old_id → new_id` 映射完整性检查

### Truncate 阶段

只有当 export + verify 全部通过才执行：

```sql
BEGIN IMMEDIATE;
-- 1. 备份旧表
ALTER TABLE instances RENAME TO instances_backup_v0_2;
-- 2. 重建新表（带 UNIQUE 约束）
CREATE TABLE instances (
    id TEXT PRIMARY KEY,
    template_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    api_key TEXT NOT NULL,
    alias TEXT NOT NULL DEFAULT '',
    opencode_model_id TEXT NOT NULL DEFAULT '',
    kv_cache_enabled INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    UNIQUE(template_id, alias)
);
COMMIT;
```

`instances_backup_v0_2` 保留**直到下一次启动**后由用户手动 `DROP`（保护用户数据，让用户能反悔）。提示：保留 N 天后自动清理。

### Import 阶段

逐行 `INSERT INTO instances (...)`，依赖 UNIQUE 约束做最终一致性兜底。如果某条 INSERT 触发 UNIQUE violation，回滚整事务，提示用户：

```
迁移失败：第 N 条 instance (template_id=minimax, alias=cl-mini) 与新数据冲突
请检查导出文件 migration_backup_v0_2_*.json 中是否有重复 alias
```

### 幂等性

- **重复 export**：同一 DB → 同一 JSON（按 created_at 排序 + 稳定 JSON 序列化）
- **重复 run**：第一次成功后 truncate 已清空旧表，第二次会 export 一个全空 JSON（id 已无 model 段），verify 阶段会把它判为"无需迁移"并跳过 truncate/import
- **export 后中断**：用户保留 JSON 文件，可手动 `--import <file>` 恢复

## alias 校验规则

按用户决策：**alias 只能小写字母、数字、`-`、`_`，不能有空白或大写字母**。

### 校验函数

```rust
// src/domain/instance.rs 新增
pub fn validate_alias(alias: &str) -> Result<(), AppError> {
    if alias.is_empty() {
        return Err(AppError::InvalidAlias("alias 不能为空".to_string()));
    }
    if alias.len() > 32 {
        return Err(AppError::InvalidAlias("alias 长度不能超过 32 字符".to_string()));
    }
    if !alias.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_') {
        return Err(AppError::InvalidAlias(
            "alias 只能包含小写字母、数字、-、_".to_string()
        ));
    }
    Ok(())
}
```

### 接入点

- **DAO `create_instance`** —— 写入前 `validate_alias(&instance.alias)?`
- **DAO `update_instance`** —— 写入前 `validate_alias(&new_alias)?`
- **DAO `set_alias` / `rename_instance`** —— 同步校验
- **TUI 输入层** —— 在 `EditField` 模式下做实时过滤（拒绝大写键、空白键），但底层 DAO 校验是兜底

### 现有 alias 不符合新规则怎么办

老用户的 alias 可能有大写或空白（例如 `cl-Mini`、`cl mini`）。迁移工具**不**自动归一化，保留原值。在 verify 阶段输出警告：

```
WARN: 3 个 instance 的 alias 不符合新规则（大写/空白），保持原值
  - minimax-cl Mini
  - kimi-CL-km2
  - openai-cl Mini
```

用户可手动编辑为合规 alias。**这避免了静默数据破坏**。

## Testing Strategy

### 新增测试文件

**`tests/edit_instance_test.rs`** — 编辑 model 端到端：
- `test_edit_model_changes_model_id` — 改 model 后 instance.model_id 变更，alias 保留
- `test_edit_model_preserves_id_when_alias_unchanged` — 改 model 但 alias 不变 → id 完全不变
- `test_edit_model_changes_id_when_alias_changes` — 改 model 同时改 alias → id 重写
- `test_edit_model_duplicate_alias_rejected` — alias 重复时 update 失败
- `test_edit_model_persists_to_dao` — 改完 model 后读回来是新值

**`tests/migration_test.rs`** — 独立迁移工具：
- `test_export_produces_stable_json` — 同 DB 两次 export 字节级一致
- `test_transform_id_3_segment_to_2_segment` — `minimax-MiniMax-M2.7-cl-mini` → `minimax-cl-mini`
- `test_transform_id_already_new_format_noop` — 已是新格式保持不变
- `test_transform_id_cross_template` — 不同 template 的 instance 各自正确转换
- `test_verify_detects_data_loss` — 模拟 export 时丢掉 api_key，verify 失败
- `test_verify_detects_duplicate_new_id` — 两个 instance 转换后 new_id 冲突 → verify 失败
- `test_run_full_migration_lifecycle` — export → verify → truncate → import 全流程，数据零丢失
- `test_run_idempotent_no_change_second_time` — 第二次 migrate 检测到已迁移则跳过
- `test_truncate_keeps_backup_until_manual_drop` — 旧表保留为 `instances_backup_v0_2`
- `test_alias_validation_rejects_uppercase` — 大写 alias 被 `validate_alias` 拒绝
- `test_alias_validation_rejects_whitespace` — 含空白 alias 被拒绝
- `test_alias_validation_accepts_lowercase_alnum_dash_underscore` — 合规 alias 通过

### 现有测试同步

- `src/shell.rs` 测试：硬编码 id 改为新格式（`minimax-cl-mini` 等）
- `src/dao/sqlite_impl.rs` 测试：硬编码 id 改为新格式 + 添加 UNIQUE 约束测试
- `src/app/state.rs` 测试：mock instance 的 id 改为新格式

## Boundaries

- **Always**:
  - 迁移是离线操作，由独立 `migrate` 子命令触发，应用启动时**不**自动迁移
  - 迁移采用 `export → verify → truncate → import` 四步流程，verify 不通过绝不允许 truncate
  - alias 写入 DAO 前必须 `validate_alias`（小写字母/数字/`-`/`_`）
  - 编辑保存时校验 `(template_id, alias)` 唯一性
  - 改 model 后重生成 shell alias 文件（用户下次 `Enter` 触发）
  - 写代码前先写失败测试（TDD）

- **Ask first**:
  - 修改 `ProviderInstance` 结构体（添加/删除字段）——目前不需要
  - 引入新依赖 crate
  - 删改任何测试断言

- **Never**:
  - 删除 M2.7 highspeed 模板（向后兼容）
  - 跨 template 编辑 model（已确认不允许）
  - 改 model 时清空 `opencode_model_id` 用户已设值
  - 迁移时丢弃任何用户数据

## Implementation Plan

### Phase 1: id 解耦（核心）

1. **修改 id 拼接逻辑** `src/app/state.rs:530` → 改为 `{template_id}-{alias}`
2. **扩展 DAO trait** `src/dao/mod.rs:30` → `update_instance` 接收 `new_model_id + new_alias + api_key`
3. **实现 SQLite 端** `src/dao/sqlite_impl.rs` → 事务化 update + UNIQUE 约束
4. **实现内存端** `src/dao/memory_impl.rs` → 同步逻辑
5. **更新所有硬编码 id 的测试** —— 旧 id 改为新 id 格式

**验收**: `cargo test` 通过，id 格式全局一致

### Phase 2: alias 校验

1. **实现 `validate_alias`** `src/domain/instance.rs`
2. **DAO 入口接入校验** `create_instance` / `update_instance` / `set_alias` / `rename_instance`
3. **添加 alias 校验单测** `src/domain/instance.rs` 内嵌

**验收**: 大写/空白 alias 写入 DB 立即返回 `AppError::InvalidAlias`

### Phase 3: 独立 migrate 工具

1. **创建 `src/bin/migrate.rs`** —— CLI 入口
2. **实现 export 模块** —— JSON 序列化、版本号、稳定排序
3. **实现 transform 模块** —— 3 段 id 拆分为 2 段
4. **实现 verify 模块** —— 关键字段哈希 + 重复检测
5. **实现 truncate 模块** —— rename 旧表 + 建新表 + UNIQUE 约束
6. **实现 import 模块** —— 逐行 INSERT，UNIQUE 兜底
7. **添加迁移测试** `tests/migration_test.rs`

**验收**: 模拟 v0.2.x 旧 DB → migrate 一次成功 → 数据无损 → 旧表保留为 backup

### Phase 3: EditInfoPanel 增强

1. **state.rs 中 EditInfoPanel 新增 Model 选项** `focus_index = 1`
2. **ui/edit.rs 渲染 Model 行**（带 `[↑↓]` 提示）
3. **handle_edit_info_panel 增加 Model 处理分支**
4. **添加编辑端到端测试** `tests/edit_instance_test.rs`

**验收**: 在 TUI 中按 `e` → 改 Model → 保存 → 看到 instance 反映新 model

### Phase 4: shell alias 重生成

1. **`regenerate_aliases` 验证逻辑** —— 改 model 后重新生成时使用新 model 的 env
2. **手动测试** —— TUI 中改 model 后按 `Enter` 激活，检查 alias 文件内容

**验收**: alias 文件中 `ANTHROPIC_MODEL` 等变量跟随新 model

## Task Breakdown

- [ ] **Task 1**: 修改 id 拼接逻辑 + 同步所有硬编码测试
  - Acceptance: `cargo test` 全过，id 格式全局一致
  - Verify: `cargo test && cargo clippy`
  - Files: `src/app/state.rs`, `src/shell.rs`, `src/dao/sqlite_impl.rs`, `src/dao/memory_impl.rs`

- [ ] **Task 1**: 修改 id 拼接逻辑为 `{template_id}-{alias}` + 同步所有硬编码测试
  - Acceptance: `cargo test` 全过，id 格式全局一致
  - Verify: `cargo test && cargo clippy`
  - Files: `src/app/state.rs`, `src/shell.rs`, `src/dao/sqlite_impl.rs`, `src/dao/memory_impl.rs`, `src/domain/instance.rs`

- [ ] **Task 2**: 扩展 DAO `update_instance` 支持 model_id + alias + api_key
  - Acceptance: 新签名编译通过，单元测试覆盖正常/异常路径
  - Verify: `cargo test dao`
  - Files: `src/dao/mod.rs`, `src/dao/sqlite_impl.rs`, `src/dao/memory_impl.rs`

- [ ] **Task 3**: 添加 `(template_id, alias)` UNIQUE 约束
  - Acceptance: 创建/编辑时 alias 重复被 DB 拒绝
  - Verify: `cargo test` + 新增 unique violation 测试
  - Files: `src/dao/sqlite_impl.rs`

- [ ] **Task 4**: 实现 `validate_alias` 并接入 DAO 所有写入路径
  - Acceptance: 大写/空白 alias 写入立即返回 `InvalidAlias`
  - Verify: `cargo test` 通过 + 新增 3 个 alias 校验单测
  - Files: `src/domain/instance.rs`, `src/dao/sqlite_impl.rs`, `src/dao/memory_impl.rs`

- [ ] **Task 5**: 创建 `src/bin/migrate.rs` —— export/transform/verify/truncate/import 全流程
  - Acceptance: 模拟 v0.2.x 旧 DB → 一次 migrate 成功 → 数据无损 → 旧表保留 backup
  - Verify: `tests/migration_test.rs` 全部通过
  - Files: `src/bin/migrate.rs`, `src/dao/sqlite_impl.rs`, `tests/migration_test.rs`

- [ ] **Task 6**: EditInfoPanel 新增 Model 字段 + alias 字段保持
  - Acceptance: TUI 中按 `e` → 改 Model → 保存 → instance 反映新 model
  - Verify: `tests/edit_instance_test.rs` 通过 + 手动 TUI 测试
  - Files: `src/app/state.rs`, `src/ui/edit.rs`

- [ ] **Task 7**: 验证 shell alias 文件随 model 重生成
  - Acceptance: 改 model 后 `Enter` 激活生成的 alias 文件 env 变量正确
  - Verify: 手动 + shell 模块单测
  - Files: `src/shell.rs`, `src/app/state.rs`

- [ ] **Task 8**: 全量验证
  - Acceptance: `cargo test` + `cargo clippy` + `cargo build --release` 全过；`cargo run --bin migrate --dry-run` 可执行
  - Verify: 四条命令依次执行
  - Files: 无（验证步骤）

## Resolved Decisions

以下问题在 Phase 1 已与用户确认，记录在此供回溯：

1. **Edit 行为**：原地改 model + alias 文件重生成。instance.id 解耦后改 model 不影响 id 稳定性。
2. **Opencode 联动**：保持当前逻辑——原值空时按新 model 重算；已设值则保留。
3. **跨 template 编辑**：不允许。
4. **id 格式**：`{template_id}-{alias}`，不加 hash 后缀，唯一性由 DB UNIQUE 保证。
5. **数据迁移**：独立 `migrate` 子命令，流程 `export → verify → truncate → import`，应用启动不自动迁移。
6. **alias 校验**：只能小写字母、数字、`-`、`_`，无空白/大写。老数据不合规的由用户手动修正，迁移不自动归一化。
7. **alias 唯一性**：`(template_id, alias)` 维度，DB UNIQUE 约束兜底。
