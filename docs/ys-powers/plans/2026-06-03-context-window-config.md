# Implementation Plan: 模型上下文窗口（Context Window）显示与配置

## Overview

在 cc-switch-tui 的后端数据模型、API、Shell Alias 生成以及前端 UI 中，完整实现模型上下文窗口的展示与实例级开关控制。计划按垂直切片组织，每个任务交付可独立验证的功能。

## Architecture Decisions

1. **模型级别定义，实例级别开关** — `ModelTemplate.context_window` 定义模型固有能力，`ProviderInstance.context_window_enabled` 控制是否生效。不开放实例级数值覆盖，保持简单。
2. **SQLite 兼容迁移** — 使用 `PRAGMA table_info` 检测缺失列并 `ALTER TABLE ADD COLUMN`，确保旧数据库不损坏。
3. **环境变量注入在 shell.rs** — context window 相关变量与 `ANTHROPIC_AUTH_TOKEN` 等一并注入 alias 函数，遵循现有 `build_env` 模式。不改动 opencode 配置生成。
4. **前端条件渲染** — 只有当 `model.contextWindow` 存在时才显示 toggle，避免对不支持模型造成干扰。

## Dependency Graph

```
templates.rs (ModelTemplate 新增 context_window)
    │
    ├── domain/template.rs ──┐
    ├── domain/instance.rs ──┤
    │                          │
    ├── dao/mod.rs             │
    │   └── dao/sqlite_impl.rs │
    │                          │
    ├── api/templates.rs       │
    │   └── web/src/api/types.ts
    │                          │
    └── api/instances.rs       │
        └── web/src/api/hooks.ts
            │
            ├── web/src/components/ModelSelect.tsx
            ├── web/src/components/InstanceForm.tsx
            ├── web/src/routes/InstanceDetailPage.tsx
            └── web/src/components/InstancesTable.tsx

shell.rs (build_env 注入)
    └── 依赖 domain/template.rs + domain/instance.rs
```

## Task List

### Phase 1: 后端数据模型与持久化

- [ ] **Task 1: Domain 模型扩展**
  - **Description:** 在 Rust domain 层为 `ModelTemplate` 和 `ProviderInstance` 新增字段，并更新模板注册数据。
  - **Acceptance criteria:**
    - [ ] `ModelTemplate` 新增 `context_window: Option<u64>`，位于 struct 末尾
    - [ ] `ProviderInstance` 新增 `context_window_enabled: bool`，位于 `kv_cache_enabled` 之后
    - [ ] `src/templates.rs` 中 MiniMax-M3 的 `context_window = Some(1_000_000)`
    - [ ] MiniMax-M2.7 和 Kimi 的 `context_window = None`
  - **Verification:**
    - [ ] `cargo check` 无编译错误
    - [ ] `cargo test` 现有测试通过（需同步更新测试中的 struct 构造）
  - **Dependencies:** None
  - **Files touched:**
    - `src/domain/template.rs`
    - `src/domain/instance.rs`
    - `src/templates.rs`
  - **Estimated scope:** Small (3 files)

- [ ] **Task 2: DAO 层扩展与数据库迁移**
  - **Description:** 在 Dao trait 和 SqliteDaoImpl 中新增 `context_window_enabled` 的读写支持，包括旧表结构的自动迁移。
  - **Acceptance criteria:**
    - [ ] `Dao` trait 新增 `set_context_window_enabled(&mut self, id: &str, enabled: bool)`
    - [ ] `sqlite_impl.rs` 构造函数中通过 `PRAGMA table_info` 检测并添加 `context_window_enabled INTEGER NOT NULL DEFAULT 0` 列
    - [ ] `refresh_instances` 的 SELECT 和 struct 构造包含新字段
    - [ ] `create_instance` INSERT 包含新字段（默认 false）
    - [ ] `set_context_window_enabled` 实现并刷新内存缓存
    - [ ] `rename_instance` 的 INSERT 包含新字段
    - [ ] 新增测试覆盖：旧表迁移、创建实例默认值、set 方法独立测试
  - **Verification:**
    - [ ] `cargo test dao::sqlite_impl` 全部通过
    - [ ] 手动验证：用旧数据库文件启动，列自动添加，数据不丢失
  - **Dependencies:** Task 1
  - **Files touched:**
    - `src/dao/mod.rs`
    - `src/dao/sqlite_impl.rs`
  - **Estimated scope:** Medium (2 files + 测试)

### Checkpoint: Phase 1 完成
- [ ] `cargo test` 全部通过
- [ ] `cargo build --release` 成功
- [ ] `cargo clippy` 无新警告（可选）

### Phase 2: 后端 API 与 Shell Alias

- [ ] **Task 3: API 层扩展**
  - **Description:** 更新 Templates API 和 Instances API，暴露新增字段。
  - **Acceptance criteria:**
    - [ ] `api/templates.rs` 的 `TemplateModelSummary` 新增 `context_window: Option<u64>`，并在 `From` 实现中映射
    - [ ] `api/instances.rs` 的 `InstanceSummary`、`InstanceDetail` 新增 `context_window_enabled: bool`
    - [ ] `CreateInstanceRequest` 新增 `context_window_enabled: Option<bool>`，创建时默认 false
    - [ ] `PatchInstanceRequest` 新增 `context_window_enabled: Option<bool>`
    - [ ] `patch` handler 中新增对 `context_window_enabled` 的更新逻辑（类似 `kv_cache_enabled`）
    - [ ] `duplicate` handler 复制时继承原实例的 `context_window_enabled`
  - **Verification:**
    - [ ] `cargo test` 通过
    - [ ] 启动 server 后，`GET /api/templates` 响应中 MiniMax-M3 的 `contextWindow` 为 1000000
    - [ ] `GET /api/instances` 响应中包含 `contextWindowEnabled` 字段
  - **Dependencies:** Task 2
  - **Files touched:**
    - `src/api/templates.rs`
    - `src/api/instances.rs`
  - **Estimated scope:** Medium (2 files)

- [ ] **Task 4: Shell Alias 环境变量注入**
  - **Description:** 在 `shell.rs` 的 `build_env` 中，当实例开启且模型有预设值时注入三个环境变量；同时更新 `get_all_env_vars` 确保 unset 列表完整。
  - **Acceptance criteria:**
    - [ ] `build_env` 中，在 `CC_SWITCH_ALIAS` 注入之前新增逻辑：若 `instance.context_window_enabled` 为 true 且对应模型的 `context_window` 为 Some，则注入 `DISABLE_COMPACT=1`、`CLAUDE_CODE_MAX_CONTEXT_TOKENS={值}`、`CLAUDE_CODE_AUTO_COMPACT_WINDOW={值}`
    - [ ] `get_all_env_vars` 将上述三个变量加入 unset 集合
    - [ ] 新增测试：开启时 alias 内容包含三个变量；关闭时或不支持模型时不包含
    - [ ] 更新现有测试中 `ProviderInstance` 的构造代码（添加 `context_window_enabled: false`）
  - **Verification:**
    - [ ] `cargo test shell::` 通过
    - [ ] 生成的 alias 函数内容经人工抽查确认正确
  - **Dependencies:** Task 1
  - **Files touched:**
    - `src/shell.rs`
  - **Estimated scope:** Small (1 file + 测试)

### Checkpoint: Phase 2 完成
- [ ] `cargo test` 全部通过
- [ ] 后端 API 响应包含新字段（curl 或浏览器验证）
- [ ] 生成 alias 内容正确（检查 `~/.cc-switch-tui/aliases.zsh` 或通过 Web 预览）

### Phase 3: 前端基础类型与 Hooks

- [ ] **Task 5: 前端类型与验证**
  - **Description:** 更新前端 API 类型定义、hooks 和表单验证 schema。
  - **Acceptance criteria:**
    - [ ] `web/src/api/types.ts` 中 `TemplateModel` 新增 `contextWindow?: number`
    - [ ] `Instance` 和 `InstanceDetail` 新增 `contextWindowEnabled: boolean`
    - [ ] `web/src/api/hooks.ts` 中 `useCreateInstance` 和 `useUpdateInstance` 的 payload 包含 `contextWindowEnabled`
    - [ ] `web/src/lib/validate.ts` 的 `instanceSchema` 新增 `contextWindowEnabled: z.boolean().default(false)`
  - **Verification:**
    - [ ] `cd web && npm run typecheck` 无错误
    - [ ] `cd web && npm run test` 现有测试通过
  - **Dependencies:** Task 3
  - **Files touched:**
    - `web/src/api/types.ts`
    - `web/src/api/hooks.ts`
    - `web/src/lib/validate.ts`
  - **Estimated scope:** Small (3 files)

### Checkpoint: Phase 3 完成
- [ ] `npm run typecheck` 通过
- [ ] `npm run test` 通过

### Phase 4: 前端 UI 组件

- [ ] **Task 6: ModelSelect 与 InstancesTable**
  - **Description:** 在模型选择下拉中显示 context window badge，在实例列表 Flags 列显示 CTX 标记。
  - **Acceptance criteria:**
    - [ ] `ModelSelect` 中，有 `contextWindow` 的模型 option 显示 "· 1M context"（使用 `formatTokens` 辅助函数）
    - [ ] `InstancesTable` 的 Flags 列在 `contextWindowEnabled` 为 true 时显示 "CTX" badge（样式类似现有 "KV" badge）
    - [ ] 新增/更新对应测试
  - **Verification:**
    - [ ] `npm run test -- ModelSelect` 通过
    - [ ] `npm run test -- InstancesTable` 通过（如有测试）
    - [ ] 浏览器中查看模型下拉和实例列表，视觉确认
  - **Dependencies:** Task 5
  - **Files touched:**
    - `web/src/components/ModelSelect.tsx`
    - `web/src/components/InstancesTable.tsx`
    - `web/src/components/__tests__/ModelSelect.test.tsx`
  - **Estimated scope:** Small (2-3 files)

- [ ] **Task 7: InstanceForm 与 InstanceDetailPage**
  - **Description:** 在实例创建表单和详情编辑页中，为支持 context window 的模型条件渲染 toggle 开关。
  - **Acceptance criteria:**
    - [ ] `InstanceForm` 中，根据当前所选模型的 `contextWindow` 是否存在，条件渲染 toggle
    - [ ] Toggle 标签：`Enable extended context window (1M tokens)`
    - [ ] Toggle 的 `title` tooltip：`Disables all compaction, including manual /compact. Monitor context usage carefully.`
    - [ ] `InstanceDetailPage` 中同步相同逻辑
    - [ ] 保存时 `contextWindowEnabled` 正确提交到 API
    - [ ] 新增/更新对应测试
  - **Verification:**
    - [ ] `npm run test -- InstanceForm` 通过
    - [ ] `npm run test -- InstanceDetailPage` 通过
    - [ ] 浏览器端到端验证：创建 M3 实例 → 显示 toggle → 开启 → 保存 → 刷新后状态保持
  - **Dependencies:** Task 6
  - **Files touched:**
    - `web/src/components/InstanceForm.tsx`
    - `web/src/routes/InstanceDetailPage.tsx`
    - `web/src/routes/__tests__/InstanceDetailPage.test.tsx`
  - **Estimated scope:** Medium (2-3 files + 测试)

### Checkpoint: Phase 4 完成
- [ ] `npm run test` 全部通过
- [ ] `npm run build` 成功
- [ ] 浏览器端到端验证通过

## Final Checkpoint: 全量验证
- [ ] `cargo test` 全部通过（后端）
- [ ] `npm run test` 全部通过（前端）
- [ ] `npm run build` 成功（前端生产构建）
- [ ] 创建 MiniMax-M3 实例，开启 context window，生成 alias 包含三个环境变量
- [ ] 创建 MiniMax-M2.7 实例，确认无 toggle 出现
- [ ] 列表页 CTX badge 正确显示

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| SQLite 旧表迁移失败导致数据丢失 | High | 使用 `PRAGMA table_info` 检测而非假设 schema；迁移前备份 `.db` 文件；测试中验证 `:memory:` 和文件两种路径 |
| 前端类型与后端 serde 命名不一致 | Medium | 严格遵循 `camelCase` 映射；类型检查 (`tsc --noEmit`) 作为 gates |
| shell.rs 测试遗漏 env var 注入场景 | Low | 新增独立测试覆盖开启/关闭/不支持模型三种场景 |
| 修改大量 struct 构造导致测试编译失败 | Low | 每次修改 struct 后立刻运行 `cargo test`，逐个修复编译错误 |

## Parallelization Opportunities

- **Task 4 (Shell)** 理论上可与 Task 2 (DAO) 并行，因为两者只依赖 Task 1 (Domain)。但由于都在 Rust 后端且共享编译，顺序执行更稳妥。
- **Task 5 (前端类型)** 可与 Task 3/4 (后端 API) 并行，只要前后端约定好字段名。
- **Task 6 和 7 (前端 UI)** 必须顺序执行，因为都依赖 Task 5。

**推荐执行顺序：** Task 1 → Task 2 → Task 3 → Task 4 → Checkpoint → Task 5 → Task 6 → Task 7 → Final Checkpoint
