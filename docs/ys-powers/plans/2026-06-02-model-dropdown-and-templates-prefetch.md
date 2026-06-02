# Plan: 编辑页 Model 下拉框 + App 根预取 templates

> 日期：2026-06-02
> 关联 spec：`2026-06-02-model-dropdown-and-templates-prefetch-design.md`
> 工作分支：`feat/web-replaces-tui`（直接基于当前分支）

## 任务总览

```
T1 (ModelSelect 组件 + 测试)
 ├─→ T2 (InstanceForm 改用 ModelSelect)
 ├─→ T3 (InstanceDetailPage 改用 ModelSelect)
 └─→ T4 (App 根 prefetch templates + 测试)
          ↓
        T5 (Chrome DevTools 端到端验证)
```

## T1 — 新建 ModelSelect 组件 + 测试

**Files**:
- `web/src/components/ModelSelect.tsx`（新建，~32 行）
- `web/src/components/__tests__/ModelSelect.test.tsx`（新建，4 个测试）

**Acceptance**:
- `ModelSelect({ models: [], value, onChange })` → 渲染 `<input>`，change 触发 onChange
- `ModelSelect({ models: [{id, name, opencodeModelId}], value, onChange })` → 渲染 `<select>`，每个 option 显示 `${name} (${id})`
- value 变化时 select / input 反映新值
- 4 个测试全 pass

**Verify**:
- `cd web && npx vitest run ModelSelect.test.tsx` — 4 pass
- `cd web && npx tsc --noEmit` — 0 error

**TDD**:
1. RED：写 4 个测试
2. GREEN：实现 ModelSelect
3. commit: `feat(web): add ModelSelect component`

## T2 — InstanceForm 改用 ModelSelect

**Files**:
- `web/src/components/InstanceForm.tsx`（修改 line 138-157 → ~5 行）

**Acceptance**:
- 删除内联的 `currentTemplate.models.length > 0 ? select : input` 分支
- 改为 `<ModelSelect models={...} value={...} onChange={...} />`
- 行为完全等价：option 显示 `name (id)`；空 models 时降级为 input
- `InstanceForm.test.tsx`（如存在）仍 pass

**Verify**:
- `cd web && npx vitest run InstanceForm.test.tsx`（如存在）— pass
- `cd web && npx tsc --noEmit` — 0 error

**commit**: `refactor(web): use ModelSelect in InstanceForm`

## T3 — InstanceDetailPage 改用 ModelSelect

**Files**:
- `web/src/routes/InstanceDetailPage.tsx`（修改 line 137-143 → ~5 行）

**Acceptance**:
- 删除 `<input>` 硬编码
- 改为 `<ModelSelect models={...} value={...} onChange={...} />`
- 行为：template.models 非空时显示下拉（option 为 `name (id)`），为空时降级为 input
- 现有测试 pass（如有 `InstanceDetailPage.test.tsx`）
- 手动验证：编辑页 Model 字段是 select 不是 input

**Verify**:
- `cd web && npx vitest run InstanceDetailPage.test.tsx`（如存在）— pass
- `cd web && npx tsc --noEmit` — 0 error

**commit**: `fix(web): use ModelSelect dropdown in edit page`

## T4 — App 根 prefetch templates + 测试

**Files**:
- `web/src/App.tsx`（修改，加 useQueryClient + useEffect prefetch）
- `web/src/__tests__/App.test.tsx` 或 `web/src/App.test.tsx`（新建，如不存在）

**Acceptance**:
- App 渲染后立即触发 `/api/templates` 请求（1 次）
- prefetch 失败静默（`void` + catch 兜底）
- 进入 `/instances/:id` 或创建对话框时 templates 已在 cache（不会再次触发 fetch）
- 测试：渲染 `<App/>` → 等待 → `expect(fetchStub).toHaveBeenCalledWith('/api/templates', ...)` 至少 1 次

**Verify**:
- `cd web && npx vitest run` — 全部 pass
- `cd web && npx tsc --noEmit` — 0 error

**commit**: `feat(web): prefetch templates at App root`

## T5 — Chrome DevTools MCP 端到端验证

**Steps**:
1. `pkill -f "vite|cargo run" 2>/dev/null; make dev &` — 启动 dev
2. `mcp__chrome-devtools__new_page({ url: "http://127.0.0.1:7480/instances/<id>" })`
3. `mcp__chrome-devtools__list_network_requests` — 验证启动后 `/api/templates` 已被调用
4. `mcp__chrome-devtools__take_snapshot` — 验证 Model 字段是 `<select>`（不是 `<input>`）
5. `mcp__chrome-devtools__click` 切 model → 验证 opencodeModelId 自动更新
6. `mcp__chrome-devtools__take_screenshot` — 保存验证截图

**Acceptance**:
- B1 验证：编辑页 Model 是 select，options 数量 = template.models.length
- B2 验证：刷新页面后到下拉显示 ≤ 100ms（无 spinner）
- B2 验证：network 面板 `/api/templates` 只在刷新时调用 1 次

**commit**: 验证不产生新 commit（仅在 plan 里记录）

## 检查点

- **CP1** (T1+T2+T3+T4 后)：typecheck 0 error + vitest 全 pass + `npx vitest run ModelSelect.test.tsx` 4 pass
- **CP2** (T5 后)：Chrome DevTools 截图显示下拉 + network 面板确认只 1 次 templates 请求

## 风险与缓解

| 风险 | 缓解 |
|---|---|
| prefetch 触发了双请求（prefetch + useTemplates 自身） | `useTemplates` 命中 cache 后不发请求；QueryClient 内部 dedupe |
| InstanceForm 已经在 useTemplates，prefetc 是冗余 | 接受冗余 — 启动时 prefetch 是显式优化，hooks 调用是消费。两者复用 cache 不重复发请求 |
| ModelSelect 形态和 OpencodeModelSelect 重复 → 抽象过度 | 形态平行是设计选择（一个选 model id，一个选 opencode model id）；不再上提一层（2 实例不需要 YAGNI） |
| 测试中 fetch mock 不够快导致 prefetch 失败 | 测试用 `vi.waitFor` 等待 prefetch 完成；不需要修改 fetch stub |

## 验证

- [ ] T1-T4 全部 commit
- [ ] `cd web && npx tsc --noEmit` 0 error
- [ ] `cd web && npx vitest run` 全部 pass
- [ ] Chrome DevTools 端到端：B1 + B2 都验证通过
- [ ] git log 显示 4 个独立 commit（一个不漏，也不多）
