# Plan: 支持编辑已存在 instance 的 model

> 对应 spec: `docs/ys-powers/specs/2026-06-01-edit-instance-model-design.md`

> **Task 3 状态：取消**（A1 决策：alias 唯一性由 `state.rs::validate_alias` 列表检查兜底，未加 DB UNIQUE 约束）
> **Task 5 状态：未实施**（独立 migrate 工具，作为可选后续工作）
> 详见 §8 用户拍板结果

## 1. 依赖关系分析

```
              ┌─────────────────────────┐
              │  src/domain/instance.rs │ ← Task 4: validate_alias
              │  src/domain/error.rs    │
              └────────────┬────────────┘
                           │ 错误类型 + 校验函数
                           ▼
              ┌─────────────────────────┐
              │  src/dao/mod.rs         │ ← Task 2: update_instance 签名
              │  src/dao/sqlite_impl.rs │ ← Task 1, 2, 3, 4, 5
              │  src/dao/memory_impl.rs │
              └────────────┬────────────┘
                           │ DAO 行为
                           ▼
   ┌───────────────────────┼───────────────────────┐
   ▼                       ▼                       ▼
┌──────────┐         ┌──────────┐          ┌────────────────┐
│ src/app/ │         │ src/     │          │ src/bin/       │
│ state.rs │         │ shell.rs │          │ migrate.rs     │
│ Task 6   │         │ Task 7   │          │ Task 5         │
│ (id 拼接 │         │ (alias   │          │ (独立迁移工具) │
│  + 编辑) │         │  生成)   │          │                │
└──────────┘         └──────────┘          └────────────────┘
   │                       │
   └─────────┬─────────────┘
             ▼
       tests/edit_instance_test.rs  (Task 6)
       tests/migration_test.rs      (Task 5)
```

**关键依赖**：
- Task 1（id 拼接）必须先于所有其他任务（改了 `state.rs:530` 影响全局）
- Task 2（DAO 签名）必须先于 Task 6（EditInfoPanel 才能调用新方法）
- Task 3（UNIQUE）和 Task 4（alias 校验）相对独立，可与 Task 2 并行
- Task 5（migrate 工具）独立于 UI 改动，可较早完成
- Task 7（shell 验证）依赖 Task 1、6 完成

---

## 2. 代码现状 vs spec 的 2 个冲突点

**这两个点必须在 plan review 阶段澄清，否则 Task 1-3 实现会被阻塞：**

### 冲突点 A：alias 唯一性范围

| 来源 | 规则 |
|------|------|
| 现有 `state.rs:578` | alias **全表唯一**（不区分 template） |
| spec §Core Design | `(template_id, alias)` **联合唯一**（不同 template 允许同 alias） |

**两种选择**：
- **A1（推荐）**：保留现有全表唯一规则。spec 改为"alias 全表唯一"。实现简单，不需要 DB UNIQUE 约束。  
- **A2**：按 spec 加 UNIQUE 约束。需要修改 `validate_alias` 唯一性检查逻辑为 `(template_id, alias)`。

### 冲突点 B：`validate_alias` 是否下沉 DAO

| 来源 | 规则 |
|------|------|
| 现有 `state.rs:560` | 校验**只在 state 层**，DAO 层不校验 |
| spec §Boundaries | "alias 写入 DAO 前必须 `validate_alias`" |

**两种选择**：
- **B1（推荐）**：下沉到 DAO 层，state 层和 DAO 层双重校验。`update_instance` / `set_alias` / `rename_instance` 入口都调用 `validate_alias`。防御性更强，符合 spec。
- **B2**：保留 state 层单点校验。代码改动更小，但任何绕过 state 层的代码（如 migrate 工具）都不会校验。

---

## 3. 任务分解

### Task 1: id 拼接逻辑改为 `{template_id}-{alias}`

**目标**：让所有 instance.id 用新格式，全局一致。

**改动点**：
- `src/app/state.rs:530` — `format!("{}-{}-{}", template_id, model_id, alias)` → `format!("{}-{}", template_id, alias)`
- `src/domain/instance.rs:6` — 文档注释从 `"template_id-model_id-alias"` → `"template_id-alias"`
- `src/shell.rs` — 硬编码测试 id 同步改（`minimax-MiniMax-M2.7-highspeed-cl-mini` → `minimax-cl-mini`）
- `src/dao/sqlite_impl.rs` — 测试硬编码 id 同步改
- `src/dao/memory_impl.rs` — 无硬编码 id（test 用的也是新格式，verify）
- `src/app/state.rs` — mock instance 测试 id 同步改

**验收**：
- `cargo test` 全过
- 全局 grep `MiniMax-M2.7-highspeed-` 只剩 `instance.model_id` 字段，**不再出现在 id 拼接中**

**验证**：`cargo test && grep -rn "format!.*-.*-.*alias" src/`

---

### Task 2: 扩展 DAO `update_instance` 签名

**目标**：让 DAO 支持"原地改 model + alias + api_key"。

**改动点**：
- `src/dao/mod.rs:30` — 旧签名替换为：
  ```rust
  fn update_instance(
      &mut self,
      id: &str,
      model_id: String,    // 新
      alias: String,      // 新（重命名 alias）
      api_key: String,
  ) -> Result<(), AppError>;
  ```
- `src/dao/sqlite_impl.rs:161` — 单 SQL：`UPDATE instances SET model_id=?, alias=?, api_key=? WHERE id=?`
- `src/dao/memory_impl.rs:58` — 同样更新 3 个字段
- 旧 API `update_instance(id, api_key)` 的所有调用方更新

**注**：当 alias 改变时，instance.id 仍不变（因为新 id 格式下 alias 不在 id 里），无需 rename。简化了 spec 里的"双 UPDATE"。

**验收**：
- `cargo test dao` 全过
- 新增单测 `test_update_instance_changes_all_three_fields` + `test_update_instance_preserves_other_fields` (created_at, opencode_model_id, kv_cache)

**验证**：`cargo test dao && cargo clippy`

---

### Task 3: `(template_id, alias)` 联合 UNIQUE 约束

**目标**：DB 层兜底防止 alias 冲突。

**⚠️ 依赖冲突点 A 的决策**：
- A1（推荐）：无需 UNIQUE，仍用现有"应用层 alias 全表唯一"逻辑
- A2：执行此 task，添加 `UNIQUE(template_id, alias)` 约束

**改动点（A2 路径）**：
- `src/dao/sqlite_impl.rs:25` — CREATE TABLE 加 `, UNIQUE(template_id, alias)`
- `src/dao/sqlite_impl.rs:122` — INSERT 错误处理识别 UNIQUE violation → `AliasAlreadyExists`
- 新增单测 `test_create_instance_duplicate_alias_rejected` + `test_update_instance_duplicate_alias_rejected`

**验收**：
- 重复 alias（无论同/不同 template）→ DB 拒绝 → `AppError::AliasAlreadyExists`
- 现有数据不冲突

**验证**：`cargo test`

---

### Task 4: `validate_alias` 规则更新 + 下沉 DAO

**目标**：alias 只能小写字母/数字/`-`/`_`，所有写入路径强制校验。

**改动点**：
- `src/domain/instance.rs` 新增 `validate_alias` 函数（参考 spec §alias 校验规则）
- `src/app/state.rs:560` — 现有 `validate_alias` 替换为调用 `instance::validate_alias`，去掉大写允许 + 加长度限制（≤32）
- `src/dao/sqlite_impl.rs` — `create_instance` / `update_instance` / `set_alias` / `rename_instance` 入口调用 `validate_alias`
- `src/dao/memory_impl.rs` — 同上
- 新增单测（`src/domain/instance.rs` 内嵌）：
  - `test_validate_alias_accepts_lowercase_alnum_dash_underscore`
  - `test_validate_alias_rejects_uppercase`
  - `test_validate_alias_rejects_whitespace`
  - `test_validate_alias_rejects_empty`
  - `test_validate_alias_rejects_too_long`

**验收**：
- 大写/空白/超长/空 alias 在所有写入入口返回 `InvalidAlias`
- 现有 `cl-mini` 风格 alias 仍通过

**验证**：`cargo test`

---

### Task 5: 独立 migrate 工具

**目标**：`cargo run --bin migrate` 完成 4 步迁移（export → verify → truncate → import）。

**新增文件**：
- `src/bin/migrate.rs` —— CLI 入口，argv 解析（不引入 clap）
- `src/migrate/` —— 拆 4 个子模块（`export.rs`, `transform.rs`, `verify.rs`, `truncate.rs`, `import.rs`）
- `tests/migration_test.rs` —— 12 个测试覆盖 spec §Testing Strategy 全部场景

**关键模块**：

```rust
// src/migrate/transform.rs
pub fn transform_id(old_id: &str, template_id: &str) -> Result<String, AppError> {
    let prefix = format!("{}-", template_id);
    let suffix = old_id.strip_prefix(&prefix)
        .ok_or_else(|| AppError::MigrationError(...))?;
    let segments: Vec<&str> = suffix.splitn(2, '-').collect();
    match segments.len() {
        2 => Ok(format!("{}{}", prefix, segments[1])), // 旧格式
        1 => Ok(old_id.to_string()),                    // 已新格式
        _ => unreachable!(),
    }
}
```

**新建 DB 表 schema**：
```sql
CREATE TABLE instances (
    id TEXT PRIMARY KEY,
    template_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    api_key TEXT NOT NULL,
    alias TEXT NOT NULL DEFAULT '',
    opencode_model_id TEXT NOT NULL DEFAULT '',
    kv_cache_enabled INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
    -- 依赖冲突点 A 决策：可选 UNIQUE(template_id, alias)
);
```

**验收**：
- 模拟 v0.2.x 旧 DB → 一次 migrate 成功
- 关键字段（api_key, alias, opencode_model_id, created_at, kv_cache）零丢失
- 旧表保留为 `instances_backup_v0_2`
- 第二次运行检测到已迁移则跳过

**验证**：
- `cargo test --test migration_test`
- `cargo run --bin migrate -- --dry-run`（用临时 DB）

---

### Task 6: EditInfoPanel 新增 Model 字段

**目标**：TUI 中按 `e` 编辑 instance 时，可改 model。

**改动点**：
- `src/app/state.rs:35-50` — `EditField` enum 新增 `Model` 变体
- `src/app/state.rs:43-48` — `AppState::EditInfoPanel` 新增 `model_index: usize` 字段
- `src/app/state.rs:649-650` — `EditField` 列表加 `Model`，其他字段 focus_index 顺移
- `src/app/state.rs:608-680` — `handle_edit_info_panel` 新增 `Up`/`Down` 切换 model_index 分支
- `src/app/state.rs` 新增 `handle_save_edit` —— 收集所有字段，调用 `dao.update_instance(...)`
- `src/ui/edit.rs` — `draw_edit` 新增 Model 行渲染（带 `[↑↓]` 提示）
- 新增 `tests/edit_instance_test.rs`：
  - `test_edit_model_changes_model_id`
  - `test_edit_model_preserves_id_when_alias_unchanged`
  - `test_edit_model_duplicate_alias_rejected`
  - `test_edit_model_persists_to_dao`

**EditInfoPanel 新布局**：
```
┌─ 编辑 instance ─────────────────────┐
│ > Model:    MiniMax M3  [↑↓]        │  ← focus 1（新增）
│   Alias:    cl-mini                 │  ← focus 2
│   API Key:  sk-***...               │  ← focus 3
│   KV Cache: [ ]                      │  ← focus 4
│   [Delete]                          │  ← focus 5
│   [Save]  [Cancel]                  │
└─────────────────────────────────────┘
```

**验收**：
- 端到端：TUI 中按 `e` → `Tab` 切到 Model → `↑↓` 选 M3 → `Enter` 保存
- DB 验证：model_id 已变，alias 保留，id 仍为 `{template_id}-{alias}`

**验证**：`cargo test --test edit_instance_test` + 手动 TUI

---

### Task 7: shell alias 文件随 model 重生成

**目标**：改 model 后 `Enter` 激活时，shell alias 文件 env 变量跟随新 model。

**改动点**：
- `src/shell.rs:54` — 现有 `template.models.iter().find(|m| m.id == instance.model_id)` 已正确
- `src/app/state.rs` — `regenerate_aliases` 路径验证
- 手动 TUI 测试 + 现有 `src/shell.rs` 单测覆盖

**验收**：
- 改 model 后 `Enter` 激活，alias 文件中 `ANTHROPIC_MODEL` 等 env 变量指向新 model
- 旧 model 的 env 残留被覆盖

**验证**：
- 手动：`cargo run` → 改 model → 检查 `~/.cc-switch-tui/aliases` 或等价文件
- `cargo test shell`

---

### Task 8: 全量验证

**验收**：
- `cargo test` 全部通过
- `cargo clippy` 无 warning
- `cargo build --release` 成功
- `cargo run --bin migrate -- --dry-run` 可执行

**验证**：依次执行四条命令。

---

## 4. 垂直切片（Vertical Slice）演示

为避免一次性大量改动引入回归，**Task 1 和 Task 2 合并为一个端到端 slice**：

**Slice A: id 解耦 + DAO update 扩签名**
1. 改 `state.rs:530` id 拼接为新格式
2. 同步所有硬编码 id 测试 → 跑 `cargo test`，应失败（因为 dao 的 update_instance 仍接 api_key）
3. 扩展 DAO `update_instance` 签名 → 跑 `cargo test`，应通过
4. 验证：grep 全局无 `model_id}-{` 残留在 id 拼接

**Slice B: alias 校验下沉**
1. 在 `domain/instance.rs` 写 `validate_alias` 函数 + 5 个单测（RED）
2. 接入 DAO 各写入路径（GREEN）
3. 替换 `state.rs:560` 现有 validate_alias（REFACTOR）

**Slice C: EditInfoPanel 增强**
1. 写 `test_edit_model_changes_model_id`（RED）
2. 实现 `handle_save_edit` + `draw_edit` Model 行（GREEN）
3. 手动 TUI 验证（VERIFY）

**Slice D: migrate 工具**
1. 写 `test_transform_id_3_segment_to_2_segment`（RED）
2. 实现 `src/migrate/transform.rs`（GREEN）
3. 依序加 export/verify/truncate/import + 对应测试
4. 集成测试 `test_run_full_migration_lifecycle`（端到端）

---

## 5. 实施顺序与检查点

```
[Task 1] id 拼接
   ↓ CP1: cargo test 通过
[Task 2] DAO 签名
   ↓ CP2: cargo test dao 通过
[Task 4] alias 校验  ───┐
   ↓ CP3               │  （可与 Task 3 并行）
[Task 3] UNIQUE 约束   ───┘
   ↓ CP4: cargo test 通过
[Task 5] migrate 工具
   ↓ CP5: tests/migration_test.rs 通过
[Task 6] EditInfoPanel
   ↓ CP6: tests/edit_instance_test.rs 通过 + 手动 TUI
[Task 7] shell alias
   ↓ CP7: 手动 + cargo test shell
[Task 8] 全量验证
```

**关键检查点**：
- **CP1**（Task 1 后）：grep 全局无 `model_id}` 残留
- **CP2**（Task 2 后）：新签名编译通过 + dao 单测覆盖
- **CP3**（Task 4 后）：所有非法 alias 写入被拒
- **CP4**（Task 3 后）：冲突 alias 报 `AliasAlreadyExists`（仅 A2 路径）
- **CP5**（Task 5 后）：模拟 v0.2.x 旧 DB 端到端迁移成功
- **CP6**（Task 6 后）：TUI 改 model 全流程通
- **CP7**（Task 7 后）：alias 文件 env 跟随新 model

---

## 6. 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| 冲突点 A / B 决策未拍板 | Task 1-3 实现阻塞 | **plan review 阶段必须先回答这两个问题** |
| 现有 alias `cl-Mini` 不符合新规则 | 老用户升级后无法创建新 instance | 迁移工具 WARN 提示，不强制；老数据保留 |
| 迁移中断在 truncate 后 import 前 | 用户数据全丢 | truncate 是 rename 旧表，旧数据完整保留在 `instances_backup_v0_2` |
| EditInfoPanel 新增 Model 字段引入 UI 错位 | 已有 UX 习惯被打乱 | 垂直切片 C 阶段手动 TUI 验证 |
| `update_instance` 改名破坏外部调用 | 编译失败 | 全局 grep `dao.update_instance` 同步更新所有调用方 |

---

## 7. 待用户拍板的 2 个问题

> 阻塞 Task 1-3 实施。请在 plan review 时一并回答。

**问题 1（冲突点 A）**：alias 唯一性范围？
- **A1（推荐）**：全表唯一（与现有 state.rs:578 一致）
- A2：`(template_id, alias)` 联合唯一（按 spec）

**问题 2（冲突点 B）**：`validate_alias` 是否下沉 DAO 层？
- **B1（推荐）**：下沉到 DAO，所有写入路径强制校验
- B2：只保留 state 层单点校验

---

## 8. 用户拍板结果（2026-06-01）

✅ **A1**：alias 全表唯一，Task 3 (UNIQUE 约束) **取消**，alias 唯一性由 Task 4 (DAO 下沉校验) 兜底
✅ **B1**：`validate_alias` 下沉到 DAO 层

**Task 3 调整**：合并到 Task 4 实施。在 Task 4 中通过 DAO 入口的全表 alias 检查保证唯一性。
