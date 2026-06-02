# Spec: 模型上下文窗口（Context Window）显示与配置

## Objective

在 cc-switch-tui 的 Web UI 和 Shell Alias 生成中，显式支持模型上下文窗口（context window）的展示与开关控制。

**用户故事：**
- 作为用户，我在选择模型时能看到该模型支持的上下文窗口大小（如 MiniMax-M3 显示 "1M context"），从而了解模型能力。
- 作为用户，我在创建/编辑实例时，可以为支持扩展上下文的模型开启一个开关，让生成的 shell alias 自动注入正确的环境变量以启用完整上下文窗口。
- 作为用户，我开启该开关前会看到明确的提示，了解 `DISABLE_COMPACT=1` 的副作用（禁用所有自动压缩及手动 `/compact` 命令）。

**成功标准：**
1. MiniMax-M3 模型在 ModelSelect 中显示 "1M context" badge
2. 创建/编辑 MiniMax-M3 实例时出现 toggle 开关，其他模型（如 M2.7、Kimi）不出现
3. 开启开关后生成的 alias 包含 `DISABLE_COMPACT=1`、`CLAUDE_CODE_MAX_CONTEXT_TOKENS=1000000`、`CLAUDE_CODE_AUTO_COMPACT_WINDOW=1000000`
4. 关闭开关或模型无预设值时，不注入上述变量
5. 列表页的 Flags 列对开启实例显示 "CTX" badge
6. 所有变更通过现有测试 + 新增测试

## Tech Stack

- **后端：** Rust 1.85+，Axum，rusqlite，serde，chrono
- **前端：** React 18，TypeScript 5.5+，Tailwind CSS 3.4+，TanStack Query 5.51+
- **测试：** Rust 内置 test + tempfile；前端 Vitest + React Testing Library
- **数据：** SQLite（文件级持久化，已有 PRAGMA table_info 迁移模式）

## Commands

```bash
# 后端构建与测试
cargo build --release
cargo test

# 前端构建与测试
cd web && npm run build
npm run test        # vitest run
npm run typecheck   # tsc --noEmit

# 端到端验证（本地启动后）
cargo run -- server
cd web && npm run dev
```

## Project Structure

```
src/
  domain/
    template.rs          # ModelTemplate 新增 context_window: Option<u64>
    instance.rs          # ProviderInstance 新增 context_window_enabled: bool
  dao/
    mod.rs               # Dao trait 新增 set_context_window_enabled
    sqlite_impl.rs       # 新增列迁移 + 方法实现
  api/
    templates.rs         # TemplateModelSummary 新增 context_window
    instances.rs         # InstanceSummary/Detail/Request 新增 context_window_enabled
  shell.rs               # build_env 注入 context window 环境变量
  templates.rs           # MiniMax-M3 的 context_window = Some(1_000_000)

web/src/
  api/
    types.ts             # TemplateModel/Instance/InstanceDetail 新增字段
    hooks.ts             # useCreateInstance/useUpdateInstance payload 新增字段
  components/
    ModelSelect.tsx       # 显示 context window badge
    InstanceForm.tsx      # 条件显示 toggle + tooltip
    InstancesTable.tsx    # Flags 列显示 CTX badge
  routes/
    InstanceDetailPage.tsx # 条件显示 toggle + tooltip
  lib/
    validate.ts           # instanceSchema 新增 context_window_enabled
```

## Code Style

**Rust：** 匹配现有风格。`Option` 字段放在 struct 末尾，DAO 变更遵循 `set_kv_cache_enabled` 模式。

```rust
// domain/template.rs
pub struct ModelTemplate {
    pub id: String,
    pub name: String,
    pub env_overrides: HashMap<String, String>,
    pub opencode_model_id: String,
    /// 上下文窗口大小（tokens）。None 表示不暴露 context window 配置。
    pub context_window: Option<u64>,
}

// shell.rs build_env 中
if instance.context_window_enabled {
    if let Some(window) = model.context_window {
        env.insert("DISABLE_COMPACT".to_string(), "1".to_string());
        env.insert("CLAUDE_CODE_MAX_CONTEXT_TOKENS".to_string(), window.to_string());
        env.insert("CLAUDE_CODE_AUTO_COMPACT_WINDOW".to_string(), window.to_string());
    }
}
```

**前端：** 匹配现有 React + Tailwind 风格。条件渲染用 `&&`，tooltip 用原生的 `title` 属性。

```tsx
// 模型选择中显示 badge
{models.map((m) => (
  <option key={m.id} value={m.id}>
    {m.name} ({m.id})
    {m.contextWindow ? ` · ${formatTokens(m.contextWindow)} context` : ''}
  </option>
))}

// toggle 条件渲染
{currentModel?.contextWindow && (
  <label className="flex items-center gap-2 text-sm" title="Disables all compaction, including manual /compact. Monitor context usage carefully.">
    <input
      type="checkbox"
      checked={values.contextWindowEnabled}
      onChange={(e) => set('contextWindowEnabled', e.target.checked)}
      className="rounded"
    />
    <span>Enable extended context window ({formatTokens(currentModel.contextWindow)} tokens)</span>
  </label>
)}
```

## Testing Strategy

| 层级 | 框架 | 位置 | 覆盖范围 |
|------|------|------|----------|
| 单元 | Rust test | `src/dao/sqlite_impl.rs` | 新增列迁移、create/update/duplicate 时 context_window_enabled 的读写 |
| 单元 | Rust test | `src/shell.rs` | build_env 注入逻辑：开启时包含三个变量，关闭时不包含 |
| 单元 | Vitest | `web/src/components/__tests__/` | ModelSelect 渲染 badge；InstanceForm 条件渲染 toggle |
| 集成 | Vitest | `web/src/routes/__tests__/` | InstanceDetailPage PATCH 提交包含 contextWindowEnabled |
| 类型 | tsc | `web/` | 全量类型检查无错误 |

**新增测试清单：**
- `test_context_window_column_migration` — SQLite 兼容旧表结构，自动添加列
- `test_create_instance_with_context_window_enabled` — 创建时正确写入
- `test_set_context_window_enabled` — DAO 方法独立测试
- `test_build_env_injects_context_window_vars` — shell.rs 注入逻辑
- `test_model_select_shows_context_badge` — 前端组件渲染

## Boundaries

- **Always do：**
  - 修改 struct 时同步更新对应测试的构造代码
  - SQLite schema 变更使用 PRAGMA table_info 检测缺失列（不破坏旧数据库）
  - 前端 API 类型与 Rust serde rename_all = "camelCase" 严格对齐
  - 环境变量名称与官方文档完全一致（`DISABLE_COMPACT`, `CLAUDE_CODE_MAX_CONTEXT_TOKENS`, `CLAUDE_CODE_AUTO_COMPACT_WINDOW`）

- **Ask first：**
  - 添加新的 npm/cargo 依赖
  - 修改 opencode 配置生成逻辑（本次需求不涉及，但未来若需同步到 opencode config 需讨论）
  - 给现有实例设置默认值（是否迁移旧数据开启/关闭）

- **Never do：**
  - 删除或修改 `env_overrides` 现有用法（保持兼容）
  - 在实例级别允许自定义 context window 数值（超出本次范围）
  - 修改 `ANTHROPIC_MODEL` 等现有环境变量的设置逻辑
  - 提交包含真实 API key 的测试数据

## Open Questions（已确认）

1. **旧实例默认值：** 已有实例的 `context_window_enabled` 默认设为 `false`。**→ 符合预期。**
2. **`CLAUDE_CODE_AUTO_COMPACT_WINDOW` 数值：** 与 `MAX_CONTEXT_TOKENS` 设为相同值。**→ 相同即可。**
3. **MiniMax-M2.7 是否也支持 1M？** 当前只给 M3 设置 `context_window`。**→ 仅 M3 支持，M2.7 不设 context_window。**

---

**Status:** 已确认设计方向，待实现。
