# Spec: 编辑页 Model 下拉框 + App 根预取 templates

> 状态：Spec (Phase 1) — 待实施
> 日期：2026-06-02
> 分支：`feat/web-replaces-tui`
> 关联 spec：`2026-06-02-web-replaces-tui-design.md` (T2 已为创建路径下拉)

## 1. Objective（目标）

修复 v0.4.0 web 替换 TUI 后两个 UX 不一致问题：

| # | 严重度 | 现象 | 根因 |
|---|---|---|---|
| **B1** | 中 | 编辑 instance 时 Model 字段是 `<input>`，但创建时是下拉框 — 用户体验割裂，编辑会丢失模板约束 | `InstanceDetailPage.tsx:138-143` 写死 `<input>`，未复用 `InstanceForm` 的下拉逻辑 |
| **B2** | 低 | "启动时一次性获取下拉框" 期望未实现 — 每次进入表单路由 templates 重新从网络拉，常见 spinner | `useTemplates` 是被动 query hook，没有 app 启动时的预取。`useTemplates` 自己有 60s staleTime 没问题，但**打开表单的瞬间还是要等**第一次请求返回 |

### 用户已确认的修复方向

| Bug | 方向 |
|---|---|
| B1 | 编辑路径下拉框 + 行为和创建完全一致（template.models 非空时下拉，name (id)，为空时 input） |
| B2 | App 根 `prefetchQuery` 把 templates 拉到 QueryClient cache，让所有路由打开表单时直接命中缓存 |

### 非目标（Out of Scope）

- 不动 `useTemplates` 自身的缓存策略（保留 60s staleTime）
- 不重构 `InstanceForm` / `InstanceDetailPage` 的整体结构
- 不改后端 API
- 不引入新依赖

## 2. Commands

延续 v0.4.0 Makefile；本 spec 不增加新命令。

```bash
make dev          # vite + cargo watch
make web-build    # cd web && npm install && npm run build
make test         # cargo test
make typecheck    # cd web && npx tsc --noEmit
make lint         # cd web && npm run lint
```

## 3. Project Structure

### 需要修改的文件

| 文件 | 改动 |
|---|---|
| `web/src/components/ModelSelect.tsx` | **新文件**。提取共享组件：`ModelSelect({ models, value, onChange })`，template.models 非空时 select，为空时 input。和 `OpencodeModelSelect` 形态平行 |
| `web/src/components/InstanceForm.tsx` | Model 字段改为 `<ModelSelect>`，删除内联 select/input 分支 |
| `web/src/routes/InstanceDetailPage.tsx` | Model 字段改为 `<ModelSelect>`，删除内联 `<input>` |
| `web/src/App.tsx` | 顶层调用 `qc.prefetchQuery(['templates'], ...)` 在 QueryClient 上下文内 |
| `web/src/main.tsx` | 把 `queryClient` 实例提到 module 顶层并 export，让 App 可以调用 prefetch（如果不可行，改用 `<App/>` 内 useEffect） |

### 测试

| 文件 | 改动 |
|---|---|
| `web/src/components/__tests__/ModelSelect.test.tsx` | **新文件**。测 select 模式（template.models 非空）、input 模式（空）、change 回调、自定义 placeholder |
| `web/src/routes/__tests__/InstanceDetailPage.test.tsx` | 如已存在则扩展，否则新建。覆盖：Model 字段是 select、切换 model 触发 onChange |

### 决策点

#### 3.1 共享 ModelSelect vs 内联重复？

**选择：共享**。代码形态和 `OpencodeModelSelect` 平行，利于阅读和未来扩展（如果以后需要 `<ModelSelect>` 独立测试或扩展 placeholder 等）。

#### 3.2 App 根预取 vs 路由级 loader？

**选择：App 根 prefetch**。理由：

- React Router 6 没有原生 data loader / route-level prefetch（v6.4+ 有 `loader` 但需要切换到数据路由模式）
- 走 `qc.prefetchQuery` 是 React Query 推荐模式
- 一次 prefetch 缓存，InstanceForm / InstanceDetailPage / ApplyPage（如果未来需要）全部共享
- 失败也不影响功能（页面打开时会自动 retry）

#### 3.3 prefetch 时机？

**选择：App 组件渲染时（useEffect）**。比 main.tsx 模块加载更早（`main.tsx` 还在配 Provider 时 App 还没 mount），且不会阻塞 React 启动（useEffect 是异步）。

```tsx
// App.tsx
const qc = useQueryClient();
useEffect(() => {
  void qc.prefetchQuery({
    queryKey: ['templates'],
    queryFn: () => apiGet<Template[]>('/api/templates'),
    staleTime: 60_000,
  });
}, [qc]);
```

> 备选方案：把 `queryClient` 提到 module top level，在 main.tsx 中 `await queryClient.prefetchQuery(...)`，但这会阻塞首屏 React 渲染。如果 templates API 慢，会感知到延迟。所以选 useEffect 异步。

## 4. Code Style

### 4.1 新组件 ModelSelect

延续 `OpencodeModelSelect` 风格（props 接受 `models: TemplateModel[]`）：

```typescript
// web/src/components/ModelSelect.tsx
import type { TemplateModel } from '../api/types';

interface Props {
  /** 模板的 models 列表；如果为空则降级为 input */
  models: TemplateModel[];
  value: string;
  onChange: (value: string) => void;
  /** 为空时的占位符（仅 input 模式用） */
  placeholder?: string;
}

/**
 * Model 字段的 select/input 切换组件。
 * 当 template.models 非空时显示下拉（显示 `name (id)`），为空时降级为 input。
 * 和 OpencodeModelSelect 形态平行。
 */
export function ModelSelect({ models, value, onChange, placeholder }: Props) {
  if (models.length === 0) {
    return (
      <input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder ?? 'MiniMax-M3'}
        className="w-full px-3 py-1.5 text-sm rounded border border-input bg-background font-mono"
      />
    );
  }
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="w-full px-3 py-1.5 text-sm rounded border border-input bg-background font-mono"
    >
      {models.map((m) => (
        <option key={m.id} value={m.id}>{m.name} ({m.id})</option>
      ))}
    </select>
  );
}
```

### 4.2 InstanceForm / InstanceDetailPage 改造

把内联 select/input 替换为 `<ModelSelect>`：

```tsx
// InstanceForm.tsx — 替换 line 138-157
<Field label="Model" error={errors.modelId}>
  <ModelSelect
    models={currentTemplate?.models ?? []}
    value={values.modelId}
    onChange={(v) => set('modelId', v)}
  />
</Field>
```

```tsx
// InstanceDetailPage.tsx — 替换 line 137-143
<Field label="Model">
  <ModelSelect
    models={templates?.find((t) => t.id === instance.templateId)?.models ?? []}
    value={draft.modelId ?? ''}
    onChange={(v) => set('modelId', v)}
  />
</Field>
```

### 4.3 App 根 prefetch

```tsx
// web/src/App.tsx
import { useEffect } from 'react';
import { useHealth, useQueryClient, useTemplates } from './api/hooks';
import type { Template } from './api/types';

export default function App() {
  const { data: health } = useHealth();
  const qc = useQueryClient();

  // App 启动时一次性预取 templates，让所有需要下拉的页面（创建/编辑/Apply）打开时直接命中缓存
  useEffect(() => {
    void qc.prefetchQuery({
      queryKey: ['templates'],
      queryFn: () => fetch('/api/templates').then((r) => r.json() as Promise<Template[]>),
      staleTime: 60_000,
    });
  }, [qc]);

  // ...
}
```

> 注：当前代码已经导出了 `useTemplates` hook，本 spec 不复用它（prefetch 需要在 useEffect 内显式发起，避免 useTemplates 触发条件渲染）。也可以直接 `qc.ensureQueryData({ queryKey: ['templates'], queryFn: ... })`。

## 5. Testing Strategy

### 前端

| 测试 | 文件 | 验证 |
|---|---|---|
| `ModelSelect: input 模式 (空 models)` | `__tests__/ModelSelect.test.tsx` | 渲染 `<input>`，change 触发 onChange |
| `ModelSelect: select 模式 (非空 models)` | 同上 | 渲染 `<select>`，每项 `name (id)`，change 触发 onChange |
| `ModelSelect: 受控值同步` | 同上 | value prop 更新时下拉 / input 反映新值 |
| `InstanceDetailPage: Model 是 select 不是 input` | `__tests__/InstanceDetailPage.test.tsx` | 加载后 `screen.getByRole('combobox')` 存在，且 options 数量 = template.models.length |
| `InstanceDetailPage: 切换 model 触发 opencodeModelId 同步` | 同上 | 改 select 后，opencodeModelId 字段自动更新（如有自动同步逻辑） |
| `App: 启动时调用 prefetchQuery` | `__tests__/App.test.tsx` 或 `main.test.tsx` | 渲染 `<App/>` 后，fetch 被 `/api/templates` 调用 |

### 手动验证（必须）

1. **B1 验证**：打开 `http://127.0.0.1:7480/`，点任一行进 detail page；Model 字段是下拉不是 input；切换 model 时 opencodeModelId 自动同步
2. **B2 验证**：刷新页面 → 等 ≤100ms（prefetch 已完成）→ 进入 detail page → Model 下拉无 spinner，直接显示 options
3. **回归测试**：创建路径仍正常；切 template 时下拉更新；network 面板 `/api/templates` 只在刷新后调用 1 次（cache 命中）

## 6. Boundaries

### Always do
- 跑 `npx tsc --noEmit` + `npx vitest run` 再 commit
- 保持 `ModelSelect` 和 `OpencodeModelSelect` 形态平行（同样的 props 接口风格）
- `prefetchQuery` 失败用 `void` + 静默吞（页面打开时会自动重试）

### Ask first
- 修改 `useTemplates` 的 staleTime
- 引入新依赖
- 把 prefetch 改为同步阻塞（await 在 main.tsx 顶层）

### Never do
- 把 templates API 改成必须 token / 鉴权（如需鉴权先确认后端接口）
- 把 prefetch 写进 render body（会导致每次渲染都触发）
- 在 prefetch 失败时弹 toast（应静默，让页面正常打开再 retry）

## 7. 任务分解（详见 plan 文件）

| Task | 描述 | 验收 | 估时 |
|---|---|---|---|
| **T1** | 新建 `ModelSelect` 组件 + 测试 | 测试 4 个 case pass | 15 min |
| **T2** | `InstanceForm` 改造为用 `ModelSelect`（删除内联 select/input） | 现有测试 pass；行为不变 | 5 min |
| **T3** | `InstanceDetailPage` 改造为用 `ModelSelect`（删除内联 input） | 现有测试 pass；UI 显示下拉 | 10 min |
| **T4** | App 根 prefetch templates + 测试 | 启动后 fetch 被调用 1 次；切换路由无 spinner | 15 min |
| **T5** | Chrome DevTools 端到端验证 | 进 detail 页 ≤ 100ms 显示下拉 options；切 model 时 opencode 同步 | 10 min |

### 依赖关系

```
T1 ──→ T2 ──→ T5
T1 ──→ T3 ──→ T5
T1 ──→ T4 ──→ T5
```

T1 是基础（被 T2/T3 依赖）；T2/T3/T4 可并行；T5 验证全流程。

### Risk Register

| 风险 | 缓解 |
|---|---|
| `ModelSelect` 和 `OpencodeModelSelect` 太像，被认为过度抽象 | 形态平行是设计意图；未来加 Model 字段专属属性（如 `description`）时共享基础更省事 |
| `prefetchQuery` 重复触发 | `useEffect` 依赖 `qc`（稳定），只在 mount 时触发 1 次；QueryClient 自己也有 dedupe |
| 测试中 fetch stub 无法检测 prefetch | 改用 spy 模式：`vi.fn()` 包 fetch，检查调用次数和 URL |
| InstanceDetailPage 现有 useTemplates 也调用了（line 21） | 保留 — `useTemplates` 是消费 hook，`prefetchQuery` 是预热，两者并存没问题；staleTime 内共用 cache |

## 8. Open Questions

无。用户已对 3 个核心设计决策表态（下拉形态、prefetch 时机、工作分支），本 spec 可直接进入实施。
