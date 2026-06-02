# Plan: 修复 v0.4.0 M1 UX 三个 Bug

> 来源 spec：`docs/ys-powers/specs/2026-06-02-fix-m1-ux-bugs-design.md`
> 分支：`feat/web-replaces-tui`（当前分支直接做）
> 目标版本：v0.4.1（不发 tag，按用户"不要发布"要求）

## 1. 概览

修复 v0.4.0 上线后用户报告的 3 个高严重度 bug：

| ID | 现象 | 用户确认方向 |
|---|---|---|
| **B1** | Instance 行点击无反应，无法编辑/删除 | 行可点击 + 行内删除按钮 + ConfirmDialog |
| **B2** | OpenCode Model ID 字段是 input，不是下拉 | 后端透出 `opencode_model_id` per model + 前端改下拉 |
| **B3** | aliases.zsh 预览未脱敏，API Key 全明文 | 前端预览脱敏 + reveal toggle；磁盘文件**不变** |

## 2. 依赖图

```
        ┌─ T1: B1 修复 (行交互) ──────────────┐
        │                                       │
spec ──→┤                                       ├→ T4: 回归 + 3 commit
        │                                       │
        ├─ T2: B2 修复 (opencode 下拉) ────────┤
        │                                       │
        └─ T3: B3 修复 (alias 脱敏) ───────────┘
```

- T1 / T2 / T3 互不冲突，可**串行**也可**并行**（此 plan 走串行，便于单 agent 执行）
- T4 依赖 T1+T2+T3 全完

## 3. 任务详情

### T1 — Bug #1 修复：行可点击 + 显式删除

**目标文件**：
- `web/src/components/InstancesTable.tsx`（重写）
- `web/src/routes/InstancesPage.tsx`（微调：删除 onRowClick prop 透传）

**实施步骤**：

1. `InstancesTable.tsx`：
   - 移除 `onRowClick?: (instance: Instance) => void` prop
   - 内部引入 `useNavigate` 和 `useDeleteInstance`（hook）
   - `<TableRow>` 改为 `cursor-pointer hover:bg-muted/50 transition-colors`，onClick 跳 `/instances/${i.id}`
   - 新增 Actions 列：放一个删除按钮（emoji 🗑），onClick 内 `e.stopPropagation()` 避免冒泡触发行跳转
   - 删除按钮 click → 设置 `confirmDelete` state → 渲染 `<ConfirmDialog>`
   - ConfirmDialog `onConfirm` 调 `deleteInst.mutateAsync(i.id)`，成功后 `setConfirmDelete(null)`
2. `InstancesPage.tsx`：
   - 删除 `<InstancesTable onRowClick={navigate} ...>` 调用（不再需要）
   - 保留其余代码（搜索框、Create dialog）

**Acceptance**：
- ✅ `cargo test` pass
- ✅ `npx tsc --noEmit` 0 error
- ✅ 手动：行 hover 高亮；点击进入 `/instances/:id`；点删除按钮弹出确认；确认后从列表消失

**Verify**：
```bash
cd web && npx tsc --noEmit
cargo test --test instances_list_test
```

---

### T2 — Bug #2 修复：OpenCode Model ID 改下拉

**目标文件**：
- `src/api/templates.rs`（扩 `TemplateSummary`）
- `src/api/instances.rs`（不动）
- `web/src/api/types.ts`（同步 TS 类型）
- `web/src/components/InstanceForm.tsx`（opencodeModelId 改下拉）
- `web/src/routes/InstanceDetailPage.tsx`（同上）
- `tests/templates_test.rs`（加 assertion）

**实施步骤**：

1. `src/api/templates.rs`：
   - 新增 `TemplateModelSummary { id, name, opencode_model_id }`
   - `TemplateSummary` 新增 `models: Vec<TemplateModelSummary>` 字段
   - **保留** `available_models: Vec<String>` 字段（标 `#[serde(default)]` 防向前兼容问题）+ 注释 "deprecated, prefer models"
   - `From<&ProviderTemplate>` 实现中把 `t.models` 完整 map
2. `web/src/api/types.ts`：
   - 新增 `TemplateModel { id, name, opencodeModelId }`
   - `Template` 新增 `models: TemplateModel[]`，保留 `availableModels: string[]` 标 deprecated
3. `InstanceForm.tsx`：
   - 把 `Field label="OpenCode Model ID (optional)"` 块从 `<input>` 改为 `<select>`
   - options 来自 `currentTemplate.models.map(m => <option value={m.opencodeModelId}>{m.name} ({m.opencodeModelId})</option>)`
   - 加 "Default (use model id)" 选项 value=""（空字符串走 zsh fallback）
   - useEffect 切换 model 时自动重置 opencodeModelId 为新 model 的 opencode_model_id
4. `InstanceDetailPage.tsx`：同上替换
5. `tests/templates_test.rs`：加 `assert!(body.contains("\"opencodeModelId\""), ...)` 验证新字段透出

**Acceptance**：
- ✅ `cargo test --test templates_test` pass
- ✅ `npx tsc --noEmit` 0 error
- ✅ 手动：新建 minimax-M2.7 instance 时 opencode 下拉默认 `MiniMax-M2.7-highspeed`；切到 M3 时自动变 `MiniMax-M3`

**Verify**：
```bash
cargo test --test templates_test
cd web && npx tsc --noEmit
```

---

### T3 — Bug #3 修复：alias 文件前端脱敏

**目标文件**：
- `web/src/lib/mask.ts`（**新**）
- `web/src/components/AliasesPreview.tsx`（**新**）
- `web/src/routes/AliasesPage.tsx`（替换 `<pre>` 为 `<AliasesPreview>`）
- `web/vitest.config.ts`（**新**，如加 vitest）
- `web/package.json`（加 vitest 依赖）
- `web/src/lib/__tests__/mask.test.ts`（**新**）
- `web/src/components/__tests__/AliasesPreview.test.tsx`（**新**）

**实施步骤**：

1. `web/src/lib/mask.ts`：
   - `SENSITIVE_KEY_PATTERNS = [/KEY/i, /TOKEN/i, /SECRET/i, /PASSWORD/i, /CREDENTIAL/i]`
   - `isSensitiveKey(name: string): boolean` — 匹配任一 pattern
   - `maskValue(value: string): string` — `value.length <= 8` → `'***'`；否则 `${slice(0,3)}***${slice(-4)}`
2. `web/src/components/AliasesPreview.tsx`：
   - props: `{ content: string }`
   - 内部 state `revealed: boolean`
   - 顶部一个 toggle 按钮（"👁 Reveal" / "🙈 Hide"）
   - 主体按 `\n` split 成行；每行调用 `renderLine(line, revealed)`
   - `renderLine`：用正则 `/^(\s*export\s+)([A-Z_][A-Z0-9_]*)=(.+)$/` 拆；非 export 行原样；export 行若 `isSensitiveKey(key)` 且 `!revealed` → 替换 VALUE 部分
3. `web/src/routes/AliasesPage.tsx`：
   - 替换 `<pre>{data}</pre>` 为 `<AliasesPreview content={data ?? ''} />`
4. 决定测试框架（**见风险**）：倾向于**加 Vitest**（一个 dev-dep 装得快）
   - `npm install -D vitest @testing-library/react @testing-library/jest-dom jsdom`
   - 写 2 个 test 文件验证 mask 逻辑和 reveal toggle

**Acceptance**：
- ✅ `npx tsc --noEmit` 0 error
- ✅ 如加 vitest：`npm test` pass
- ✅ 手动：默认打开 /aliases 看不到完整 key；点 reveal 后看到完整；再点隐藏

**Verify**：
```bash
cd web && npm test -- --run
cd web && npx tsc --noEmit
```

---

### T4 — 回归 + 文档 + Commit

**目标文件**：仅 git 操作

**实施步骤**：

1. 跑全量验证：
   ```bash
   cargo fmt --all
   cargo test                          # 期望 ≥ 60 测试 pass
   cd web && npm run typecheck         # 0 error
   cd web && npm run lint              # 0 error
   cd web && npm test -- --run         # 新 vitest 测试 pass
   cd web && npm run build             # 前端构建 OK
   ```
2. 手动 e2e：
   - 启动 `make dev`，开浏览器
   - 验证 B1/B2/B3 三项修复
3. 拆分 commit（按用户习惯分 3 个 commit）：
   - `fix(web): wire row click + add inline delete button (B1)`
   - `fix(web): OpenCode Model ID as dropdown (B2)`
   - `fix(web): mask sensitive vars in aliases preview (B3)`
4. **不发 tag**（按用户"不要发布"要求）

**Acceptance**：
- ✅ 全部验证命令 0 error / 0 fail
- ✅ git log 显示 3 个独立 commit，每个 commit message 含 bug ID
- ✅ 手动 e2e 三项修复均生效

## 4. 检查点

| 检查点 | 触发任务 | 验证内容 |
|---|---|---|
| **CP-A** | T1 完成 | 行可点击 + 删除按钮生效；cargo test pass |
| **CP-B** | T2 完成 | opencode 下拉生效；templates_test 增 assertion pass |
| **CP-C** | T3 完成 | 脱敏生效；vitest pass |
| **CP-D** | T4 完成 | 全部回归 pass + 3 个独立 commit |

每个 CP 后**暂停一下**，让用户验收再继续下一片。

## 5. 风险登记

| 风险 | 可能性 | 影响 | 缓解 |
|---|---|---|---|
| `TemplateSummary.availableModels` 移除破坏旧前端 | 低 | 旧前端 list 空白 | **保留字段** + `#[serde(default)]` + TS 类型标 deprecated；新增 `models` 字段 |
| 行 click + 删除按钮事件冒泡双触发 | 中 | 误删 | 删除按钮 `e.stopPropagation()`；T1 acceptance 明确测 |
| Vitest 引入拖慢安装/构建 | 低 | 首次装 ~10s | vitest 是 dev-dep 不进 dist；如团队不接受可改用 Node 22 内置 `node:test` 跑纯函数（mask.ts） |
| `maskValue` 对短 key（< 8 char）信息泄露 | 低 | 短 key 全部 `***` | 设计上 `value.length <= 8` 全 `***`，无论内容 |
| `aliases.zsh` 含 zsh-specific 语法（多行 export 等）正则漏匹配 | 中 | 漏脱敏 | 正则不匹配的行**原样显示**（zsh 99% 是 `export KEY=VAL`）；可在 T3 测试加注释 |
| B2 切 model 时 opencodeModelId 没重置 → 旧值保留导致错乱 | 中 | 错配 | T2 useEffect 强制重置；写测试覆盖 |
| T1/T2 同时改 `InstanceForm` 区域致 git conflict（不适用本 plan 串行） | 低 | 重复改 | T1 改 InstancesTable，T2 改 InstanceForm（不同文件），不冲突 |
| 截图软件 OCR 抓 emoji 🗑/👁 误读 | 极低 | 误点 | 设计上不影响，emoji 是 UI 装饰；功能上 ConfirmDialog 仍拦截 |
| `useNavigate` 在 `useDeleteInstance` 成功后需刷新列表缓存 | 低 | 删除后列表不立即少一行 | `useDeleteInstance` 的 hook 应已 `invalidateQueries(['instances'])`；T1 验证 |
| `AliasesPage` 加 reveal toggle 改变 DOM 结构影响自动化测试 | 中 | E2E 选不到元素 | 给 reveal 按钮加 `data-testid="reveal-toggle"` 方便后续测试 |

## 6. 验证命令总汇

```bash
# 单步验证
cargo test --test instances_list_test          # T1
cargo test --test templates_test               # T2
cd web && npm test -- --run                     # T3 (vitest)
cd web && npx tsc --noEmit                      # T1/T2/T3 typecheck

# 全部完成时
cargo fmt --all && cargo test                   # 全部后端
cd web && npm run typecheck && npm run lint && npm test -- --run && npm run build

# 手动 e2e
make dev
# 浏览器打开 http://127.0.0.1:7480，验证 B1/B2/B3
```

## 7. 执行顺序（agent follow）

```
T1 → CP-A → T2 → CP-B → T3 → CP-C → T4 → CP-D → done
```

每个 CP 后等用户确认（除非用户已说"不用问我，一次性定下来"）。

## 8. 后续（Out of Scope）

- B3 真实长期方案：磁盘文件用 placeholder（如 `${KIMI_API_KEY:-default}`）→ 启动时由 cc-switch-tui 注入；需要改 zsh 启动逻辑，超出本 spec
- Test Connection 按钮（S3-T5，已 defer）
- Settings 持久化（已 defer）
- 文档更新（README.md、CLAUDE.md 重写为 v0.4.0） — 在用户决定发布时再做
