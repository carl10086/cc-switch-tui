# Implementation Plan: MiniMax-M3 model id 改 `[1m]` 后缀 + 废弃 context_window 自动注入

> 上游 spec：[docs/ys-powers/specs/2026-06-30-minimax-m3-1m-suffix-design.md](../specs/2026-06-30-minimax-m3-1m-suffix-design.md)
> 上游 intent：[docs/ys-powers/intent/minimax-m3-1m-suffix.md](../intent/minimax-m3-1m-suffix.md)
> ADR：[docs/adr/0001-minimax-m3-1m-suffix-overrides-claude-md-rule.md](../../adr/0001-minimax-m3-1m-suffix-overrides-claude-md-rule.md)
> 工作分支：`feat/minimax-m3-1m-suffix`（已创建）
> 涉及文件：~18 文件，~101 处引用

## Overview

整体是一次跨前后端、跨多层的「废弃代码删除 + model id 重命名」重构。触及 `instance.context_window_enabled: bool` 与 `ModelTemplate.context_window: Option<u64>` 字段的下游所有引用，并把 `MiniMax-M3` model id 全部改为 `MiniMax-M3[1m]`。

**关键约束**：
- 用户原话确认：「废弃代码全部删除」（无 deprecated 中间态）
- `MiniMax-M2.7-highspeed` / kimi 模板 / oc-* / ys-proxy / subshell 隔离全部不动
- SQLite 迁移必须幂等（重复启动安全）
- 必须保留 `cargo build` / `npm run build` / `make build`（embed 含 web dist）在每个 phase checkpoint 通过

## Architecture Decisions

### 关键决策 1：3 个 atomic commit（不能更细拆分）

`instance.context_window_enabled` 在 star graph 中是中心节点：

```
                  instance.context_window_enabled: bool
                  ModelTemplate.context_window: Option<u64>
                                  │
        ┌────────────┬────────────┼────────────┬──────────────┐
        ▼            ▼            ▼            ▼              ▼
   domain field   build_env    DAO trait    API DTOs       tests
                                                       (30+ sites)
   ◄── 所有下游必须同步更新，否则 cargo build fail
```

如果分多次提交，中间 commit 必 build 失败。**因此 Phase 1 必须一次提交**。

同理 Phase 2（前端 `contextWindowEnabled` 在多个 React 组件 / hooks / types / validate.ts 间共享）也必须一次提交。

Phase 3（仅 docs）独立提交。

### 关键决策 2：垂直切片按"用户可感知完成度"划分，而非按代码层

| Phase | 用户感知 | 提交 |
|---|---|---|
| 1 | 后端完整重构，DB 迁移可用，aliases.zsh 正确生成 | 1 commit |
| 2 | 前端 UI 移除 context_window toggle，与后端契约一致 | 1 commit |
| 3 | 文档反映新约束 | 1 commit + 验证 |

每个 phase 在 checkpoint 留下可工作的系统状态。

### 关键决策 3：TDD 子顺序

按 spec 已定义的 6 个新测试：
- 3 个 DAO migration 测试（idempotent / rename / drop column）—— Task 3 前先写 RED
- 3 个 shell::tests 断言（aliases 包含 M3[1m] / AUTO_COMPACT_WINDOW / 不含 DISABLE_COMPACT）—— Task 2 前先写 RED

旧测试改写在重写阶段进行。

### 关键决策 4：web ModelSelect 用 id 推断，硬编码而非保留 API 字段

`inferContextFromModelId(modelId: string): string | null` 根据 `[1m]` / `[200k]` 后缀返回 "1M context" / "200K context"。删除 `ModelTemplate.contextWindow` API 字段后，model id 本身就是 source of truth。

### 关键决策 5：ARCHITECTURE.md 不强制改

只在确实描述了被废弃机制（如"context window 上下文窗口 env vars 由 instance.toggle 控制"）时才改。如没有，仅 CLAUDE.md 一处更新即可。

---

## Dependency Graph

```
                   (无依赖起点)
                        │
        ┌───────────────┼───────────────┐
        ▼               ▼               ▼
   T1: domain        T13: CLAUDE.md  T14: ARCHITECTURE.md (可选)
   删除字段             更新 [1m] 描述
        │
        ├── T2: templates (M3 → M3[1m] + AUTO_COMPACT_WINDOW)
        ├── T3: shell::build_env 删 auto-inject block
        ├── T4: DAO trait 删 set_context_window_enabled
        ├── T5: SQLite migration + DELETE COLUMN
        └── T6: API DTO 4 structs + handlers
                   │
                   └── T7: 同步所有 fixture / instance 字面量
                              │
                              └── Checkpoint A: cargo test --lib 全绿
                                          │
                                          ▼
                              T8: web types + validate
                                          │
                                          ├── T9: web hooks + components
                                          ├── T10: ModelSelect infer
                                          └── T11: web routes InstanceDetailPage
                                                     │
                                                     └── Checkpoint B: npm test 全绿
                                                                 │
                                                                 ▼
                                                  T12: make build + manual verify
```

---

## Task List

### Phase 1: Backend 完整重构（1 commit，10 文件 ~47 处引用）

#### Task 1: 删除 domain 字段（`context_window_enabled` + `context_window: Option<u64>`）

**Description**：删除 `ProviderInstance.context_window_enabled: bool` 与 `ModelTemplate.context_window: Option<u64>` 两个字段。不删则下游 build fail。

**Acceptance criteria**:
- [ ] `src/domain/instance.rs` 删除 `pub context_window_enabled: bool`
- [ ] `src/domain/template.rs` 删除 `pub context_window: Option<u64>`
- [ ] 仅这两个 domain 文件改动；下游更新见后续 task

**Verification**:
- [ ] `cargo build --lib` 在只改 domain 后**预期 fail**（下游引用）—— 确认字段已彻底删除

**Dependencies**: None

**Files**:
- `src/domain/instance.rs`
- `src/domain/template.rs`

**Estimated scope**: XS

---

#### Task 2: 改写 templates.rs：`MiniMax-M3` → `MiniMax-M3[1m]` + 新增 `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`

**Description**：在 `minimax_template()` 中：
1. M3 model 的 id / name / opencode_model_id 改为 `MiniMax-M3[1m]` / `"MiniMax M3 [1m]"`
2. M3 model 的 4 个 `ANTHROPIC_DEFAULT_*_MODEL` env_overrides 值改为 `MiniMax-M3[1m]`
3. M3 model 的 env_overrides 新增 `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`
4. M3 model 删除 `context_window: Some(1_000_000)` 字段（已被 env 字面量替代）
5. `opencode_models` vec 中 `"MiniMax-M3"` → `"MiniMax-M3[1m]"`
6. M2.7 model / kimi 模板 / default_env 全部不动

**TDD 子步**：
- 先写 RED 测试 `test_aliases_contain_minimax_m3_1m_model_id`（断言 aliases.zsh 含 `"MiniMax-M3[1m]"`）
- 先写 RED 测试 `test_aliases_contain_auto_compact_window_var`（断言含 `CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`）
- 先写 RED 测试 `test_aliases_do_not_contain_disabled_compact_var`（断言不含 `DISABLE_COMPACT`）
- 改 templates.rs 让测试 GREEN

**Acceptance criteria**:
- [ ] `cargo test --lib shell::tests::test_aliases_*` 全部 pass
- [ ] M3 id 字符串在 `templates.rs` 中只出现 `MiniMax-M3[1m]` 一种
- [ ] M3 env_overrides 含 5 个 key（4 个 ANTHROPIC_DEFAULT + AUTO_COMPACT_WINDOW）
- [ ] M2.7 env_overrides / kimi 模板 0 改动

**Verification**:
- [ ] `cargo test --lib shell::tests::test_aliases_contain_minimax_m3_1m_model_id` pass
- [ ] `cargo test --lib shell::tests::test_aliases_contain_auto_compact_window_var` pass
- [ ] `cargo test --lib shell::tests::test_aliases_do_not_contain_disabled_compact_var` pass

**Dependencies**: Task 1（context_window 字段先删才能删它在 templates.rs 的使用）

**Files**:
- `src/templates.rs`
- `src/shell.rs::tests`（新增 3 个测试函数）

**Estimated scope**: S

---

#### Task 3: 删除 `src/shell.rs::build_env` auto-inject block（约 13 行）

**Description**：删除 `if let Some(model) = ... if let Some(window) = ...` 的三行 env.insert。环境变量现在完全由 `default_env` + `env_overrides` 字面量提供。

**Acceptance criteria**:
- [ ] `src/shell.rs::build_env` 不再引用 `instance.context_window_enabled` / `model.context_window`
- [ ] env 注入逻辑保留其他部分（default_env + env_overrides merge）

**Verification**:
- [ ] `cargo build --lib` 不报 `context_window` / `context_window_enabled` 未使用错误

**Dependencies**: Task 1

**Files**:
- `src/shell.rs::build_env`（line 88-103 附近）

**Estimated scope**: XS

---

#### Task 4: 删除 `Dao::set_context_window_enabled` trait 方法 + 两个 impl

**Description**：
- `src/dao/mod.rs` 的 `Dao` trait 删除 `fn set_context_window_enabled`
- `src/dao/sqlite_impl.rs::set_context_window_enabled` 整段删除
- `src/dao/memory_impl.rs::set_context_window_enabled` 整段删除

**Acceptance criteria**:
- [ ] trait 方法签名 0 处
- [ ] 两个 impl 0 处实现

**Verification**:
- [ ] `cargo build --lib` 不报 `set_context_window_enabled` 未实现

**Dependencies**: Task 1

**Files**:
- `src/dao/mod.rs`
- `src/dao/sqlite_impl.rs`
- `src/dao/memory_impl.rs`

**Estimated scope**: XS

---

#### Task 5: SQLite migration + DELETE context_window_enabled column

**Description**：在 `SqliteDaoImpl::new` 现有 schema setup 后追加两条幂等 SQL：

```rust
// Step 1: rename old model_id rows
let _ = conn.execute(
    "UPDATE instances SET model_id = 'MiniMax-M3[1m]' WHERE model_id = 'MiniMax-M3'",
    [],
);

// Step 2: DROP COLUMN context_window_enabled
let columns: Vec<String> = ...;  // 复用已有 pragma_table_info 查询
if columns.contains(&"context_window_enabled".to_string()) {
    let _ = conn.execute(
        "ALTER TABLE instances DROP COLUMN context_window_enabled",
        [],
    );
}
```

同时：
- `SELECT` / `INSERT INTO` / `UPDATE` 移除 `context_window_enabled` 列
- 构造 `ProviderInstance` 时不再设该字段

**TDD 子步**（3 个新测试先 RED 后 GREEN）：
- `test_migration_renames_old_minimax_m3_model_id`：旧 schema + 含 `MiniMax-M3` 行 → 启动后行变 `MiniMax-M3[1m]`
- `test_migration_drops_context_window_enabled_column`：旧 schema 含该列 → 启动后 `pragma_table_info` 不含
- `test_migration_is_idempotent`：连续启动 2 次无副作用

**Acceptance criteria**:
- [ ] `cargo test --lib dao::tests` 全部 pass（含 3 个新增）
- [ ] 旧测试 `test_context_window_column_migration` 与 `test_create_instance_with_context_window_enabled` 删除
- [ ] 旧测试 `test_set_context_window_enabled_not_found` 删除（trait 方法已删）

**Verification**:
- [ ] `cargo test --lib dao::tests::test_migration_*` 全部 pass
- [ ] `cargo test --lib dao::tests` 0 failed

**Dependencies**: Task 1, Task 4

**Files**:
- `src/dao/sqlite_impl.rs`（schema + SELECT/INSERT/UPDATE + 3 个新测试 + 删 2 个旧测试）
- `src/dao/memory_impl.rs`（构造 ProviderInstance 不再设字段）

**Estimated scope**: M

---

#### Task 6: API DTO 清理（4 structs + handlers + ModelTemplate DTO + config）

**Description**：
- `src/api/instances.rs`：从 4 个 DTO 删除 `context_window_enabled`（`CreateInstanceRequest` / `PatchInstanceRequest` / `InstanceSummary` / `InstanceDetail`）
- `src/api/instances.rs::create` handler：删 `req.context_window_enabled.unwrap_or(false)` 行
- `src/api/instances.rs::patch` handler：删 `set_context_window_enabled` 调用块
- `src/api/templates.rs::ModelTemplate`：删 `pub context_window: Option<u64>`（DTO 字段）
- `src/api/config.rs::line 95` 构造 ProviderInstance 时删字段

**Acceptance criteria**:
- [ ] 4 个 DTO 0 处引用 `context_window_enabled`
- [ ] 2 个 handler 不再调用 `set_context_window_enabled`
- [ ] `ModelTemplate` API DTO 删 `context_window: Option<u64>`
- [ ] API 编译通过（`cargo build --lib` 不报错）

**Verification**:
- [ ] `cargo build --lib` 0 错误
- [ ] `grep -rn context_window_enabled src/api/` 0 命中

**Dependencies**: Task 1

**Files**:
- `src/api/instances.rs`
- `src/api/templates.rs`
- `src/api/config.rs`

**Estimated scope**: S

---

#### Task 7: 同步所有 fixture / test instance 字面量（~26 处）

**Description**：在 Rust 测试代码中，所有 `ProviderInstance { ..., context_window_enabled: false/true }` 字面量删除该字段；所有 `ModelTemplate { ..., context_window: Some(1_000_000) }` / `None` 字面量删除该字段。包括：
- `src/shell.rs::tests`（~19 处 fixture 调用）
- `src/dao/sqlite_impl.rs::tests`（构造 instance 行）
- `src/dao/memory_impl.rs::tests`
- `tests/dao_test.rs`
- 其他引用 `context_window_enabled` / `context_window:` 字面量的测试文件

**Acceptance criteria**:
- [ ] `cargo grep context_window_enabled src/ tests/` 0 命中
- [ ] `cargo grep "context_window:" src/ tests/` 0 命中（除 struct 定义已删之外的字面量）

**Verification**:
- [ ] `cargo build --tests` 0 错误
- [ ] `grep -rn "context_window_enabled\|context_window:" src/ tests/ --include="*.rs"` 0 命中

**Dependencies**: Task 1, Task 2, Task 5, Task 6

**Files**:
- `src/shell.rs::tests`（fixture 调用 ~19 处）
- `src/dao/sqlite_impl.rs::tests`
- `src/dao/memory_impl.rs::tests`
- `tests/dao_test.rs`

**Estimated scope**: M（机械批量改）

---

### Checkpoint A: Phase 1 完整 commit

**Do this**: 把 Task 1-7 合并为 1 个 commit：

```
refactor(backend): drop context_window_enabled; rename MiniMax-M3 → MiniMax-M3[1m]

- Domain: delete ProviderInstance.context_window_enabled + ModelTemplate.context_window
- Templates: M3 → M3[1m], add CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000
- Shell::build_env: remove auto-inject block
- DAO: drop set_context_window_enabled (trait + impls)
- SQLite: idempotent migration (UPDATE model_id + DROP COLUMN)
- API DTOs: remove context_window_enabled from 4 structs + handlers
- Tests: update all fixtures; add 6 new tests (3 migration + 3 aliases)
```

**Verify**:
- [ ] `cargo test --lib` 全部 pass（含 6 个新测试）
- [ ] `cargo fmt --check` 0 diff
- [ ] `cargo clippy --lib -- -D warnings` 0 warning
- [ ] `grep -rn "context_window_enabled\|context_window:" src/ tests/ --include="*.rs"` 0 命中（domain 定义已删）

**Do not proceed to Phase 2** until all green.

---

### Phase 2: Frontend 完整重构（1 commit，6 文件 26 处引用）

#### Task 8: web types + validate.ts

**Description**：
- `web/src/api/types.ts`：从 `Instance` 接口删 `contextWindowEnabled: boolean`；从 `ModelTemplate` 接口删 `contextWindow?: number`
- `web/src/lib/validate.ts`：Zod schema 删 `contextWindowEnabled: z.boolean().default(false)`

**Acceptance criteria**:
- [ ] `Instance` 类型不含 `contextWindowEnabled`
- [ ] `ModelTemplate` 类型不含 `contextWindow`
- [ ] Zod schema 不含 `contextWindowEnabled`

**Verification**:
- [ ] `grep -rn "contextWindowEnabled" web/src/` 仅命中 ModelSelect / 其他文件（这些在后续 task 删）
- [ ] `npm run typecheck` 0 错误

**Dependencies**: Task 6（API DTO 已不返回该字段）

**Files**:
- `web/src/api/types.ts`
- `web/src/lib/validate.ts`

**Estimated scope**: XS

---

#### Task 9: web hooks + 3 个组件（InstanceForm / InstancesTable / InstanceDetailPage）

**Description**：
- `web/src/api/hooks.ts`：4 处 `contextWindowEnabled` 引用删除（create payload / update payload / 实例类型 / etc.）
- `web/src/components/InstanceForm.tsx`：9 处引用删除（form state 字段 / checkbox UI / set handler / model contextWindow 显示）
- `web/src/components/InstancesTable.tsx`：1 处删除（badge 条件渲染 `{i.contextWindowEnabled && (...)}`）
- `web/src/routes/InstanceDetailPage.tsx`：9 处删除（draft state 字段 / checkbox UI）

**Acceptance criteria**:
- [ ] 4 个文件不含 `contextWindowEnabled` 引用
- [ ] InstanceForm / InstanceDetailPage 不再有 "Context Window" checkbox
- [ ] InstancesTable 不再有 context window badge

**Verification**:
- [ ] `grep -rn "contextWindowEnabled" web/src/` 0 命中
- [ ] `npm run typecheck` 0 错误
- [ ] `npm test` 全 pass

**Dependencies**: Task 8

**Files**:
- `web/src/api/hooks.ts`
- `web/src/components/InstanceForm.tsx`
- `web/src/components/InstancesTable.tsx`
- `web/src/routes/InstanceDetailPage.tsx`

**Estimated scope**: M

---

#### Task 10: ModelSelect 改用 `inferContextFromModelId`

**Description**：替换 `web/src/components/ModelSelect.tsx:43` 的 `m.contextWindow ? ...` 逻辑：

```tsx
// 删除
{m.contextWindow ? ` · ${formatTokens(m.contextWindow)} context` : ''}

// 新增工具函数 + 替换显示
function inferContextFromModelId(modelId: string): string | null {
    if (modelId.includes('[1m]')) return '1M context';
    if (modelId.includes('[200k]')) return '200K context';
    return null;
}

// 显示
{inferContextFromModelId(m.id) ? ` · ${inferContextFromModelId(m.id)}` : ''}
```

`inferContextFromModelId` 放在 `web/src/components/ModelSelect.tsx` 顶部（同文件内）或提取到 `web/src/lib/inferContext.ts`（如需跨文件复用，本 intent 暂未跨文件，仅同文件用即可）。

**Acceptance criteria**:
- [ ] ModelSelect 显示逻辑改用 `inferContextFromModelId`
- [ ] `m.contextWindow` 引用 0 处

**Verification**:
- [ ] `grep -n "contextWindow" web/src/components/ModelSelect.tsx` 0 命中
- [ ] 手动验证：M3[1m] model 显示 "· 1M context"，M2.7-highspeed 不显示

**Dependencies**: Task 8（types.ts 已删 contextWindow 字段，否则会 TS error）

**Files**:
- `web/src/components/ModelSelect.tsx`

**Estimated scope**: XS

---

### Checkpoint B: Phase 2 完整 commit

**Do this**: 把 Task 8-10 合并为 1 个 commit：

```
refactor(web): drop contextWindowEnabled; infer context from model id

- types.ts: remove from Instance + ModelTemplate
- validate.ts: remove from Zod schema
- hooks.ts: remove from 4 places
- InstanceForm.tsx: remove form field + checkbox UI
- InstancesTable.tsx: remove badge conditional render
- InstanceDetailPage.tsx: remove draft state + checkbox
- ModelSelect.tsx: replace m.contextWindow with inferContextFromModelId(id)
```

**Verify**:
- [ ] `npm run typecheck` 0 错误
- [ ] `npm test` 全 pass
- [ ] `grep -rn "contextWindow\|contextWindowEnabled" web/src/` 0 命中
- [ ] `npm run build` 0 错误

**Do not proceed to Phase 3** until all green.

---

### Phase 3: Docs + 最终验证

#### Task 11: CLAUDE.md 改写 `[1m]` 后缀硬约束

**Description**：按 ADR 0001 + spec 改写片段，CLAUDE.md 中：

1. 改写"不要依赖 `[1m]` 后缀"硬约束——按 VS Code 扩展场景 vs Claude Code 终端场景区分
2. 删除"DISABLE_COMPACT=1 + CLAUDE_CODE_MAX_CONTEXT_TOKENS 配合机制"段落（机制随字段废弃）

**Acceptance criteria**:
- [ ] CLAUDE.md `grep "[1m]"` 命中改动描述
- [ ] CLAUDE.md `grep "DISABLE_COMPACT=1"` 段已删除

**Verification**:
- [ ] `git diff CLAUDE.md` 显示改动
- [ ] 描述与 ADR 0001 内容一致

**Dependencies**: 无

**Files**:
- `CLAUDE.md`

**Estimated scope**: XS

---

#### Task 12: ARCHITECTURE.md 可选更新

**Description**：检查 `docs/codebase/ARCHITECTURE.md` 是否描述了被废弃的机制（context_window toggle / per-instance context window 配置）。如有则更新；如无则跳过。

**Acceptance criteria**:
- [ ] 检查 ARCHITECTURE.md；如有相关描述则更新
- [ ] 如无相关描述，commit 注明"无变化"

**Verification**:
- [ ] `grep -n "context_window" docs/codebase/ARCHITECTURE.md` 决定是否需改

**Dependencies**: 无

**Files**:
- `docs/codebase/ARCHITECTURE.md`（可能 0 改动）

**Estimated scope**: XS

---

#### Task 13: 最终验证

**Do this**:

```bash
# Backend
cargo test --lib                    # 91+ 个测试全 pass（含 6 新增）
cargo build --release               # build 成功
cargo fmt --check                   # 0 diff
cargo clippy --lib --tests -- -D warnings  # 0 warning

# Frontend
npm test                            # vitest 全 pass
npm run build                       # vite build 成功

# Full
make test                           # cargo + npm 全套
make lint                           # cargo clippy + eslint
make build                          # web-build + cargo build --release

# 手动 end-to-end
cargo run --release
# 1. 检查 aliases.zsh 含 MiniMax-M3[1m] + CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000
grep -E "MiniMax-M3\[1m\]|CLAUDE_CODE_AUTO_COMPACT_WINDOW" ~/.cc-switch-tui/aliases.zsh

# 2. 启动后 DB schema 不含 context_window_enabled
sqlite3 ~/.cc-switch-tui/db.sqlite ".schema instances"
# 期望: 不含 context_window_enabled

# 3. 旧 model_id 行已被 rename（手动创建测试行验证）
sqlite3 ~/.cc-switch-tui/db.sqlite "SELECT model_id FROM instances"
# 期望: 旧 MiniMax-M3 行不存在，全是 MiniMax-M3[1m]

# 4. Web UI 上 InstanceForm / InstanceDetailPage 无 "Context Window" checkbox
# 5. cl-mini --version 能跑通
```

**Acceptance criteria**:
- [ ] 全部自动化命令 0 错误
- [ ] 手动 5 步全部预期结果

**Dependencies**: Task 11, Task 12

**Files**: 无（仅验证）

**Estimated scope**: S

---

### Checkpoint C: Ready for /ys-review

**Do this**:
- [ ] 3 个 commit（Phase 1 / Phase 2 / Phase 3）全部在 `feat/minimax-m3-1m-suffix` 分支
- [ ] `make test` + `make lint` + `make build` 全绿
- [ ] 5 步手动验证通过
- [ ] git push -u origin feat/minimax-m3-1m-suffix
- [ ] 准备进 `/ys-review`

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| Phase 1 backend 一次提交涉及 10 文件 ~47 处，commit diff 大 | Low | 单 commit 逻辑连贯；review 时按文件而非单 commit 看 |
| SQLite DROP COLUMN 在旧版本 SQLite 失败 | Low | rusqlite bundled SQLite ≥ 3.35（实测）；DROP COLUMN 自 3.35 起原生支持 |
| 旧 DB 实例 `model_id="MiniMax-M3"` 未被迁移到 `MiniMax-M3[1m]` | Med | Phase 1 migration 已 UPDATE；Task 13 手动验证步骤 3 覆盖 |
| web ModelSelect 显示逻辑与后端模板不同步 | Med | `inferContextFromModelId` 显式列举已知后缀；新增 model 后缀需手动加 case |
| `make build` 因 web dist 未重新生成而 embed 旧 dist | Low | `make build` 会先跑 web-build 再 cargo build |
| web 端 Zod schema 删除后旧 form 状态残留导致 runtime crash | Low | Task 9 同步删 InstanceForm 的 form state 字段；Task 13 手动验证步骤 4 |
| `context_window_enabled` 字段在 DB schema 残留导致旧 DB 启动时 migration 报错 | Low | Task 5 DROP COLUMN 用 pragma_table_info 守卫；幂等 |
| ARCHITECTURE.md 描述与新实现脱节 | Low | Task 12 检查；如有则更新；如无则跳过 |

## Parallelization Opportunities

- **Phase 3 文档 task**（Task 11 / Task 12）可与 Phase 1 / Phase 2 并行准备（不依赖代码改动）
- **Phase 1 内部 tasks 1-6** 理论可并行（grep 之后各自更新），但会导致中间 commit build fail —— 因此合并为 1 commit
- **Phase 2 内部 tasks 8-10** 同上，合并为 1 commit

实际：3 个 commit 必须串行（依赖关系严格）。

## Open Questions

无。spec 阶段已收口所有开放问题（Q1 / Q2 / Q3 + ASK-1 / ASK-2）。

## Reference

- 上游 intent：[docs/ys-powers/intent/minimax-m3-1m-suffix.md](../intent/minimax-m3-1m-suffix.md)
- 上游 spec：[docs/ys-powers/specs/2026-06-30-minimax-m3-1m-suffix-design.md](../specs/2026-06-30-minimax-m3-1m-suffix-design.md)
- ADR：[docs/adr/0001-minimax-m3-1m-suffix-overrides-claude-md-rule.md](../../adr/0001-minimax-m3-1m-suffix-overrides-claude-md-rule.md)
- 官方文档：[platform.minimaxi.com/docs/token-plan/claude-code](https://platform.minimaxi.com/docs/token-plan/claude-code)
- 量化统计：
  - 后端 src/：`context_window*` 73 处 / `MiniMax-M3` 43 处
  - 前端 web/src/：`contextWindow*` 26 处
  - 总：~101 处引用
- SQLite version：rusqlite 0.32 bundled SQLite ≥ 3.35（`ALTER TABLE DROP COLUMN` 支持）

## 后续阶段

- `/build` —— 按本 plan 的 3 个 commit 顺序实施
- `/ys-review` —— 五维度代码审查
- `/ship` —— 交付前检查与 go/no-go