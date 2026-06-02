# Spec: One-shot Apply 工作流

## Objective

**问题**：当前 UI 误让用户以为 apply 需要多次点击。
- `AliasesPage` 顶部一个 Apply 按钮（实际已写入所有产物，但用户不知道）
- `OpencodePage` 左侧 instance 列表 + 右侧 Apply 按钮（每个 instance 都要点一次，给用户"N 次点击"的错觉）

**用户实际期望**：单页面、单按钮、一键 apply 所有产物（aliases.zsh + N 个 opencode config）。

**目标**：让"一键 apply"在 UI 上是真实可见的工作流——单一聚合页面统一预览 + 单一执行按钮。

**非目标**（明确不做）：
- 不改后端 API 形状
- 不改 apply 的写入语义（保持"先 opencode 后 aliases"）
- 不引入 apply 历史、apply 进度、apply 撤销
- 不拆 endpoint

## Tech Stack

不变：React 18 + TypeScript 5 + Vite 5；Rust 1 + axum 0.7。

## 关键事实（先纠正假设）

**后端 `/api/aliases/apply` 已经是 one-shot**（无需新加 endpoint）：
- `src/shell.rs:62` `generate_aliases()` → 内部已调 `opencode_config::generate_opencode_configs()`
- 一次 POST 写入：1 个 `aliases.zsh` + N 个 `opencode/{alias}.json`
- `OpencodePage` 上的 per-instance Apply 调 `/api/opencode-config/:id/apply`（写单文件），**这只在"只想改某一个 opencode 不想重生成 aliases"时有意义**

## Commands

不变：开发期 `make dev`，测试 `make test`。

## Project Structure

仅前端改动：

```
web/src/
├── routes/
│   ├── ApplyPage.tsx          ← 新增
│   ├── AliasesPage.tsx        ← 改：剥掉 Apply 按钮
│   └── OpencodePage.tsx       ← 改：剥掉 Apply 按钮
├── App.tsx                    ← 改：加 /apply 路由
└── components/
    └── Layout.tsx             ← 改：sidebar 加 Apply 入口
```

后端：零改动。

## 设计

### 路由结构

```
┌─ Sidebar ──────────────┐
│  • Instances           │
│  • Aliases (preview)   │
│  • OpenCode (preview)  │
│  ──────────────────    │
│  • Apply  ★ (新)       │
│  ──────────────────    │
│  • Settings            │
└────────────────────────┘
```

### ApplyPage 布局

```
┌─ Apply 全部产物 ─────────────────────────┐
│ 即将写入 3 个文件到 ~/.cc-switch-tui/      │
│                                          │
│ • ~/.cc-switch-tui/aliases.zsh   (5.2 KB) │
│ • ~/.cc-switch-tui/opencode/cl-mini.json  │
│ • ~/.cc-switch-tui/opencode/dev.json      │
│                                          │
│ ┌─ aliases.zsh ───────────────────────┐  │
│ │ export KIMI_API_KEY=sk-***          │  │
│ │ export ANTHROPIC_BASE_URL=...       │  │
│ │ alias cl-mini='...'                 │  │
│ │ [Reveal secrets]                    │  │
│ └─────────────────────────────────────┘  │
│                                          │
│ [展开] opencode/cl-mini.json             │
│ [展开] opencode/dev.json                 │
│                                          │
│              [ Apply All (3 files) ]     │
└──────────────────────────────────────────┘
```

**Apply 后**：成功 → 顶部绿色 toast "✓ Wrote 3 files"（3 秒自动消失）；失败 → 红色 banner 显示哪几个失败 + 原因。

### AliasesPage（只读）

剥掉顶部 Apply 按钮。保留：aliases.zsh 文本预览 + Reveal secrets 切换。
底部加一个"想生效？→ Apply"链接跳 `/apply`。

### OpencodePage（只读 + 跳 Apply）

保留：左侧 instance 列表（可点击切换预览） + 右侧当前 instance 的 JSON。
**Apply 按钮剥掉**。底部加一个"想写入所有产物？→ Apply"链接跳 `/apply`。

### 复用：aliases.zsh 预览

ApplyPage 直接复用 `AliasesPreview` 组件（含 Reveal secrets 逻辑）。
OpenCode JSON 预览：复用 `OpencodePage` 中现有的 `<pre>{JSON.stringify(config, null, 2)}</pre>` 模式，或抽到 `OpencodeConfigPreview` 共享组件。

### 数据获取

ApplyPage 用现成的 hooks：
- `useAliasesContent()` → 拿 aliases.zsh 文本
- `useInstances()` → 拿到 instance id 列表后并发 `useOpencodeConfig(id)` 拉所有 config

不用新加后端 endpoint，preview 全是只读 GET。

### Apply 调用

**复用 `useApplyAliases()`**（保持原 endpoint）。后端一次写完所有产物。

```typescript
// 成功后 invalidate 所有相关 query，刷新预览
onSuccess: () => {
  qc.invalidateQueries({ queryKey: ['aliases', 'content'] });
  qc.invalidateQueries({ queryKey: ['opencode-config'] });
}
```

## Code Style

不变：保持现有 React 18 函数组件 + hooks + TanStack Query 模式。

新组件命名：`ApplyPage`（page-level route component）。

## Testing Strategy

### 必须改 / 新增的测试

- **web（Vitest + @testing-library）**：
  - `ApplyPage.test.tsx`（新）：
    - 渲染：显示所有产物名 + 总数
    - Apply 按钮：点击 → 调 POST /api/aliases/apply → 显示成功 toast
    - Apply 按钮：mock 失败 → 显示错误 banner
    - 折叠展开：opencode JSON 默认折叠，点击展开
  - `AliasesPage.test.tsx`（新）：不应有 Apply 按钮
  - `OpencodePage.test.tsx`（新）：不应有 Apply 按钮
  - 现有 `AliasesPreview.test.tsx` 不动

- **Rust**：不动（零后端改动）

### 验证

```bash
cd web && npm run typecheck     # 0 error
cd web && npm test              # 全部 pass
cd web && npm run build         # 成功
cargo test                      # 现有 64/64 继续 pass
```

### 手动验证清单

1. `make dev` → 打开 http://localhost:5173/apply
2. 看到 N+1 个产物列表
3. 点 Apply → toast 显示 "Wrote N+1 files"
4. 终端 `ls ~/.cc-switch-tui/opencode/` 确认文件存在
5. `cat ~/.cc-switch-tui/aliases.zsh` 确认明文 KEY（确认写入是明文）
6. AliasesPage / OpencodePage 顶部的 Apply 按钮不再有

## Boundaries

- **Always**: 改动前跑 typecheck + test；commit 前 build；写测试覆盖新组件
- **Ask first**: 改后端 API 形状（本次明确不做）；改 sidebar 顺序；改 toast 显示时长
- **Never**: 删现有路由 / 删测试 / 改 include_dir! 嵌入流程

## Success Criteria

1. ✅ ApplyPage 单按钮一键写入 `aliases.zsh` + 所有 `opencode/*.json`
2. ✅ AliasesPage / OpencodePage 顶部无 Apply 按钮（只读预览）
3. ✅ ApplyPage 显示所有产物的可折叠预览（aliases 默认展开，opencode 默认折叠）
4. ✅ Apply 失败时显示具体哪个文件失败 + 原因
5. ✅ Apply 成功后 toast 显示写入的文件数
6. ✅ 全部测试 pass：Rust 64/64 + Vitest 全绿 + TypeScript 0 error

## Open Questions

无（已通过 explore-then-ask 阶段锁定全部 5 个问题）。
