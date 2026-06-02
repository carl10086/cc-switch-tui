# Spec: 修复 v0.4.0 M1 UX 三个高严重度 Bug

> 状态：Spec (Phase 1) — 待实施
> 日期：2026-06-02
> 关联 release：v0.4.0 (commit `cb277dd`)
> 关联 spec：`docs/ys-powers/specs/2026-06-02-web-replaces-tui-design.md`

## 1. Objective（目标）

v0.4.0（Web 替换 TUI）已 ship 但用户试用后报告 **3 个高严重度 bug**。本 spec 修复这 3 个问题，让 v0.4.0 真正达到可发布状态。

### Bug 列表

| # | 严重度 | 现象 | 根因 |
|---|---|---|---|
| **B1** | **阻断** | "instance 不能编辑，不能删除" — 列表页点击行无反应，看不到任何编辑/删除入口 | `InstancesTable` 设计了 `onRowClick` 但 `InstancesPage` 没传；detail page 路由已存在但永远进不去 |
| **B2** | 中 | "OpenCode Model ID (optional) 不是下拉框，还不如之前的 TUI" | `TemplateSummary` 只返回 `available_models: string[]`（model id 列表），未透出每个 model 的 `opencode_model_id`；前端只能用 `<input>` 收字符串 |
| **B3** | **安全** | "alias 文件没有脱敏，KIMI_API_KEY 能被截图软件直接看到" | `AliasesPage` 用 `<pre>{data}</pre>` 原样展示 `aliases.zsh`；文件内容 `export ANTHROPIC_AUTH_TOKEN=sk-...` 全明文；UI 没有任何脱敏 |

### 用户已确认的修复方向

| Bug | 方向 |
|---|---|
| B1 | 行可点击进入 detail page + 每行右侧加显式删除按钮（带 ConfirmDialog） |
| B2 | 后端 `TemplateSummary` 返回每个 model 的 `opencode_model_id`；前端改为下拉，change model 时同步重置 opencodeModelId |
| B3 | 前端预览层脱敏（仅 UI，不改磁盘文件） + reveal 按钮可临时查看明文 |

### 非目标（Out of Scope）

- 不动磁盘文件格式（zsh 必须有明文 export 才能跑；磁盘文件保持不变）
- 不改 React Query、Router、Tailwind 等基础设施
- 不重写 InstancesTable、InstanceForm 的整体结构
- 不引入新依赖（`lucide-react` 用于图标除外 — 可选，也可手写 SVG）

## 2. Commands

延续 v0.4.0 Makefile；本 spec 不增加新命令。

```bash
make dev          # 并行跑 vite + cargo watch
make web-build    # cd web && npm install && npm run build
make test         # cargo test
make typecheck    # cd web && npx tsc --noEmit
make lint         # cd web && npm run lint
make fmt          # cargo fmt --all && cd web && npm run fmt
make release      # 完整 release build（cargo build --release）
```

## 3. Project Structure

### 需要修改的文件

#### Rust 后端

| 文件 | 改动 |
|---|---|
| `src/api/templates.rs` | `TemplateSummary` 增加 `models: Vec<TemplateModelSummary>` 字段（id, name, opencode_model_id）；`availableModels` 标记 deprecated 或保留兼容 |
| `src/api/instances.rs` | `InstanceDetail` 不变（仍返回 `opencode_model_id` 单值） |

#### Web 前端

| 文件 | 改动 |
|---|---|
| `web/src/api/types.ts` | `Template` 增加 `models: TemplateModel[]` 字段（id, name, opencodeModelId）；保留 `availableModels` 兼容 |
| `web/src/components/InstancesTable.tsx` | 行点击跳 detail page（用 `useNavigate` 替代 prop 传递）；每行右侧加显式删除按钮 + 视觉提示 |
| `web/src/routes/InstancesPage.tsx` | 透传删除回调；显示 ConfirmDialog |
| `web/src/components/InstanceForm.tsx` | `opencodeModelId` 改为下拉：列出 `currentTemplate.models[].opencodeModelId`；切换 model 时同步重置 |
| `web/src/routes/InstanceDetailPage.tsx` | 同上，`opencodeModelId` 改为下拉 |
| `web/src/routes/AliasesPage.tsx` | 预览区使用 `AliasesPreview` 组件；reveal/hide toggle；首次进入默认脱敏 |
| `web/src/components/AliasesPreview.tsx` | **新文件**。行级渲染：识别 `export KEY=VALUE` 中的敏感 key，VALUE 替换成掩码；非敏感行原样输出 |
| `web/src/lib/mask.ts` | **新文件**。`isSensitiveKey(name)` + `maskValue(value)` + `reveal` 状态管理 hook |
| `web/src/components/InstanceFormDialog.tsx` | 可能要传 `onSuccess` 回调（如果尚未传） |

#### 测试

| 文件 | 改动 |
|---|---|
| `tests/templates_test.rs` | 新增 assertion：`TemplateSummary` 包含 `models` 数组，每项含 `opencode_model_id` |
| `tests/instances_list_test.rs` | 不变 |
| `tests/aliases_test.rs` | 不变（后端没改 aliases 渲染） |
| `web/src/lib/__tests__/mask.test.ts` | **新文件**。Vitest 测试 `isSensitiveKey`、`maskValue`、`reveal` 切换 |
| `web/src/components/__tests__/AliasesPreview.test.tsx` | **新文件**。测试：默认脱敏、点 reveal 后显示明文 |

### 文件结构新增

```
web/src/
├── lib/
│   ├── mask.ts                       ← 新
│   └── __tests__/
│       └── mask.test.ts              ← 新
├── components/
│   ├── AliasesPreview.tsx            ← 新
│   └── __tests__/
│       └── AliasesPreview.test.tsx   ← 新
```

## 4. Code Style

### Rust（后端）

延续现有 axum + serde 风格：

```rust
// src/api/templates.rs
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateModelSummary {
    pub id: String,                   // "MiniMax-M2.7-highspeed"
    pub name: String,                 // "MiniMax M2.7 Highspeed"
    pub opencode_model_id: String,    // "MiniMax-M2.7-highspeed"
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSummary {
    pub id: String,
    pub display_name: String,
    pub opencode_provider_id: String,
    pub opencode_base_url: String,
    pub available_models: Vec<String>,    // 保留兼容（M1 旧前端仍可用）
    pub models: Vec<TemplateModelSummary>, // 新增（B2 修复后必用）
}
```

### TypeScript（前端）

延续现有 hook + React Query 模式：

```typescript
// web/src/api/types.ts
export interface TemplateModel {
  id: string;
  name: string;
  opencodeModelId: string;
}

export interface Template {
  id: string;
  displayName: string;
  opencodeProviderId: string;
  opencodeBaseUrl: string;
  availableModels: string[];    // deprecated, 保留兼容
  models: TemplateModel[];      // 新
}
```

```typescript
// web/src/lib/mask.ts
const SENSITIVE_KEY_PATTERNS = [
  /KEY/i, /TOKEN/i, /SECRET/i, /PASSWORD/i, /CREDENTIAL/i,
];

export function isSensitiveKey(name: string): boolean {
  return SENSITIVE_KEY_PATTERNS.some((p) => p.test(name));
}

export function maskValue(value: string): string {
  if (value.length <= 8) return '***';
  // 显示前 3 + 后 4，中间 ***
  return `${value.slice(0, 3)}***${value.slice(-4)}`;
}
```

```typescript
// web/src/components/AliasesPreview.tsx
export function AliasesPreview({ content }: { content: string }) {
  const [revealed, setRevealed] = useState(false);
  const lines = content.split('\n');
  return (
    <div>
      <div className="flex justify-end mb-2">
        <button onClick={() => setRevealed(!revealed)}>
          {revealed ? '🙈 Hide secrets' : '👁 Reveal secrets'}
        </button>
      </div>
      <pre className="...">
        {lines.map((line, i) => renderLine(line, i, revealed))}
      </pre>
    </div>
  );
}

function renderLine(line: string, idx: number, revealed: boolean) {
  const m = line.match(/^(\s*export\s+)([A-Z_][A-Z0-9_]*)=(.+)$/);
  if (!m) return <span key={idx}>{line}\n</span>;
  const [, prefix, key, value] = m;
  if (!isSensitiveKey(key) || revealed) {
    return <span key={idx}>{line}\n</span>;
  }
  return (
    <span key={idx}>
      {prefix}{key}={maskValue(value)}
    </span>
  );
}
```

### 行点击 UX

```typescript
// web/src/components/InstancesTable.tsx
import { useNavigate } from 'react-router-dom';
import { useState } from 'react';
import { ConfirmDialog } from './ConfirmDialog';
import { useDeleteInstance } from '../api/hooks';

export function InstancesTable({ instances }: { instances: Instance[] }) {
  const navigate = useNavigate();
  const deleteInst = useDeleteInstance();
  const [confirmDelete, setConfirmDelete] = useState<Instance | null>(null);

  if (instances.length === 0) return <EmptyState />;

  return (
    <>
      <Table>
        {/* ... */}
        {instances.map((i) => (
          <TableRow
            key={i.id}
            onClick={() => navigate(`/instances/${i.id}`)}
            className="cursor-pointer hover:bg-muted/50"
          >
            <TableCell>{i.alias}</TableCell>
            {/* ... */}
            <TableCell className="text-right">
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  setConfirmDelete(i);
                }}
                className="p-1 text-red-600 hover:bg-red-50 rounded"
                title="Delete"
              >
                🗑
              </button>
            </TableCell>
          </TableRow>
        ))}
      </Table>
      <ConfirmDialog
        open={!!confirmDelete}
        title="Delete instance?"
        message={`This will permanently delete "${confirmDelete?.alias}".`}
        onConfirm={async () => {
          await deleteInst.mutateAsync(confirmDelete!.id);
          setConfirmDelete(null);
        }}
        onCancel={() => setConfirmDelete(null)}
      />
    </>
  );
}
```

## 5. Testing Strategy

### 后端

| 测试 | 文件 | 验证 |
|---|---|---|
| `test_template_summary_includes_opencode_model_id` | `tests/templates_test.rs` | `TemplateSummary.models[0].opencode_model_id` 非空，且与 `available_models[0]` 不一定相同（验证透出逻辑） |
| `test_aliases_endpoint_unchanged` | `tests/aliases_test.rs` | 回归：aliases 渲染仍为明文（**前端脱敏，不改后端**） |

### 前端

| 测试 | 文件 | 验证 |
|---|---|---|
| `isSensitiveKey` cases | `web/src/lib/__tests__/mask.test.ts` | `ANTHROPIC_AUTH_TOKEN` → true；`ANTHROPIC_BASE_URL` → false；`MINIMAX_API_KEY` → true；`KIMI_API_KEY` → true |
| `maskValue` cases | 同上 | 短字符串全掩；`sk-test-1234` → `sk-***1234` |
| `AliasesPreview default mask` | `web/src/components/__tests__/AliasesPreview.test.tsx` | 默认 render：key=value 替换为 `key=sk-***1234` |
| `AliasesPreview reveal toggle` | 同上 | 点击 reveal 按钮后明文显示；再次点击隐藏 |

### 手动验证（必须）

1. **B1 验证**：打开 `http://127.0.0.1:7480/`，点击行 → 应进入 `/instances/:id`；点删除按钮 → 弹确认 → 确认后从列表消失
2. **B2 验证**：新建 instance，template=minimax，model=M2.7-highspeed，opencode model 下拉应自动选中 `MiniMax-M2.7-highspeed`；切换 model 到 M3，opencode 应自动变为 `MiniMax-M3`
3. **B3 验证**：打开 /aliases 页，搜 `sk-` 找不到完整 key；点 reveal → 明文出现；再点 → 又脱敏
4. **回归测试**：所有现有 `tests/*.rs` 仍 pass；`cargo test --all` 仍 ≥ 60 测试 pass

### 不引入

- 不引入 lucide-react（用 emoji 🗑 / 👁 即可；如团队强烈要求图标库再讨论）
- 不引入新测试框架（前端继续用 Vitest，后端继续用 cargo test）
- 不引入后端 store/secrets crate（磁盘文件明文是 zsh 强制要求，不在本 spec 范围）

## 6. Boundaries

### Always do
- 跑 `cargo test` + `npx tsc --noEmit` + `cargo fmt` 后再 commit
- 保持 spec 中"用户确认的修复方向"完全不动 — 不偷加额外 UX 改版
- 修改 InstanceForm 时同步改 InstanceDetailPage（两处同源）
- 修改后端 types 同步改前端 types + 透传逻辑

### Ask first
- 修改 `shell.rs`（本 spec 不动；如发现 B3 必须改后端才能实现则停下问）
- 修改 `aliases.zsh` 磁盘格式
- 引入新依赖（`lucide-react` / `clsx` 等）
- 删除任何已存在测试

### Never do
- 改 v0.4.0 已 ship 的 API endpoint 路径（向后兼容）
- 把 `apiKey` 写入前端 console / log / state 持久化（Redux/LocalStorage）
- 暴露明文 apiKey 到任何 GET 端点的 summary 响应（继续走 detail 端点）
- 把 `KIMI_API_KEY`、`ANTHROPIC_AUTH_TOKEN` 等敏感变量保留在 React 组件 props 中跨组件传

## 7. 实施计划（任务分解）

按 vertical slice 拆分；每片交付可测的完整功能。

| Task | 描述 | 验收 | 估时 |
|---|---|---|---|
| **T1** | **B1: 行可点击 + 显式删除按钮**<br/>- `InstancesTable` 用 `useNavigate` 跳 detail<br/>- 加 row 级 hover 样式<br/>- 加删除按钮 + ConfirmDialog<br/>- `InstancesPage` 移除对 `onRowClick` prop 的依赖 | `cargo test` pass；手动：行点击进入 /instances/:id；点删除按钮可删除 | 30 min |
| **T2** | **B2: OpenCode Model ID 改下拉**<br/>- 后端 `TemplateSummary` 加 `models: Vec<TemplateModelSummary>`<br/>- 前端 `Template` type 同步<br/>- `InstanceForm` opencodeModelId 改下拉<br/>- `InstanceDetailPage` opencodeModelId 改下拉<br/>- 切换 model 时自动重置 opencodeModelId | `cargo test` pass；`npx tsc --noEmit` pass；手动：切换 model 时 opencode id 同步 | 45 min |
| **T3** | **B3: alias 文件前端脱敏 + reveal**<br/>- 新增 `web/src/lib/mask.ts` + 单测<br/>- 新增 `web/src/components/AliasesPreview.tsx` + 单测<br/>- `AliasesPage` 用 `AliasesPreview` 替换 `<pre>` 直显 | Vitest pass；手动：默认脱敏；reveal toggle 正常；非敏感行原样显示 | 45 min |
| **T4** | **回归 + 文档**<br/>- 跑 `make test` + `make typecheck` + `cargo fmt`<br/>- 更新 CHANGELOG/CLAUDE.md（如有）<br/>- `git commit` 三个 fix（按 T1/T2/T3 拆分 commit）<br/>- 不发 tag（按用户要求"不要发布"） | 全部测试 pass；git log 干净 | 15 min |

### 依赖关系

```
T1 ──→ T2 ──→ T3 ──→ T4
                  ↑──────┘
```

T1/T2/T3 可并行；T4 依赖前三个。T1 与 T2 都改 InstancesTable/InstanceForm 区域，但 T1 改的是 `<table>` 行交互，T2 改的是 form 字段；不冲突。

### Risk Register

| 风险 | 缓解 |
|---|---|
| `TemplateSummary.availableModels` 移除会破坏旧前端 | 保留字段但加 `#[serde(default)]`；前端 type 标 `availableModels: string[]` 保留兼容 |
| `maskValue` 对非 ASCII key（如 base64 with `/`、`+`）不显示 | 现有 KIMI/MINIMAX/ANTHROPIC key 都是 `sk-` 前缀 ASCII；非 ASCII 由 `value.length > 8` 兜底成 `***` |
| 行点击 + 删除按钮事件冒泡冲突 | 删除按钮 `onClick` 内 `e.stopPropagation()` |
| `useNavigate` 在 `<button>` 内嵌套触发两次跳转 | T1 验证；用 `onClick` 而非 `<a>`，避免 link nesting |

### 验证检查点

- [ ] T1 完成后：手动验证 B1；git diff 限制在 `web/src/components/InstancesTable.tsx` 和 `web/src/routes/InstancesPage.tsx`
- [ ] T2 完成后：手动验证 B2；git diff 包含 `src/api/templates.rs` + 前端 2 文件
- [ ] T3 完成后：手动验证 B3；git diff 包含 2 新文件 + 1 改文件
- [ ] T4 完成后：`cargo test` ≥ 60 pass；`npx tsc --noEmit` 0 error；git log 显示 3 个独立 commit

## 8. Open Questions

无。用户已对 3 个核心设计决策表态（行交互方式 + opencode 字段形态 + 脱敏策略），且选择"当前分支直接干"，本 spec 可直接进入实施。
