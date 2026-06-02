# Spec: Preview 页面 UI 重新设计

## Objective

**问题**：上一轮 spec `2026-06-02-oneshot-apply-design.md` 把"一键 apply"做出来了，但页面本身还是工程原型级别的 UI——基础 `<details>` 折叠、扁平列表、Apply 按钮只有 2 态。

**用户诉求**（来自对话）：
- 删除 Aliases / OpenCode 2 个子 tab（内容已被 ApplyPage 包含，冗余）
- 把 Apply 页面从"工程原型"提升为"看着舒服、用着顺手"
- "不用考虑成本，UI 重新设计，让交互更舒服"

**目标**：
1. Apply 页面作为**唯一预览+执行入口**，承担所有产物的展示和写入
2. UI 视觉从"能跑"提升到"可发布"——卡片化、icon 化、状态化
3. 交互从"点击 = 反馈"提升为"操作有动画、状态有色彩、错误可恢复"
4. 移除 sidebar 上 Aliases / OpenCode 2 个入口；连带删除路由和页面文件

**非目标**（明确不做）：
- 不改后端 API 形状（继续用 `POST /api/aliases/apply` 一次写完）
- 不引入 framer-motion 等动画库（CSS transition 够用）
- 不引入组件库（shadcn/radix 不引）
- 不做 apply 进度、apply 历史、apply 撤销
- 不改 AliasesPreview / OpencodeConfigPreview 内部实现（只调 API）

## Tech Stack

不变：React 18 + TypeScript 5 + Vite 5 + TanStack Query 5；Tailwind CSS 4。
后端 Rust 1 + axum 0.7 零改动。

可选新增：浏览器 API `navigator.clipboard.writeText`（零依赖，现代浏览器全支持）。

## Commands

不变：
- 开发：`cd web && npm run dev`
- Typecheck：`cd web && npm run typecheck`
- 测试：`cd web && npm test`
- Build：`cd web && npm run build`
- Rust：`cargo test`（零改动，全部继续 pass）

## Project Structure

### 删除文件

```
web/src/routes/AliasesPage.tsx             ← 删
web/src/routes/OpencodePage.tsx            ← 删
web/src/routes/__tests__/AliasesPage.test.tsx        ← 删
web/src/routes/__tests__/OpencodePage.test.tsx       ← 删
```

### 新增文件

```
web/src/components/CopyButton.tsx          ← 新（带复制反馈）
web/src/components/ArtifactCard.tsx        ← 新（产物卡片：icon/path/size/copy/折叠）
```

### 改写文件

```
web/src/routes/ApplyPage.tsx               ← 重新设计：单列 + sticky apply 顶部
web/src/routes/__tests__/ApplyPage.test.tsx        ← 重写测试覆盖新组件
web/src/App.tsx                            ← 移除 2 个 import + 2 个 Route + 2 个 NavLink
```

### 不动

- `web/src/components/AliasesPreview.tsx`（ApplyPage 内部继续复用）
- `web/src/components/OpencodeConfigPreview.tsx`（同上）
- `web/src/components/ApiErrorBanner.tsx`（继续用）
- 所有后端文件
- 所有 api hooks

## 设计

### Sidebar 导航（删除后）

```
┌─ Sidebar ──────────────┐
│  • Instances           │
│  ──────────────────    │
│  • Apply               │
│  ──────────────────    │
│  • Config              │
│  • Settings            │
└────────────────────────┘
```

### Apply 页面（重新设计）

```
┌────────────────────────────────────────────────────────┐
│ [sticky 顶栏 - backdrop-blur]                          │
│   ⚡ Apply all · 4 files                                │
│   ──────────────────────────────────                    │
│   [ ⚡ Apply ]   last run: never  |  next: write       │
│                                                        │
│ 说明: 即将写入 4 个文件到 ~/.cc-switch-tui/。           │
│       点 Apply 后 source ~/.zshrc 生效。                │
├────────────────────────────────────────────────────────┤
│                                                        │
│ ┌─ Card: aliases.zsh (默认展开) ────────────────┐    │
│ │  📄 aliases.zsh             240 B  [👁][📋]   │    │
│ │  ~/.cc-switch-tui/aliases.zsh                  │    │
│ │ ┌────────────────────────────────────────────┐ │    │
│ │ │ export KIMI_API_KEY=sk-****                │ │    │
│ │ │ export ANTHROPIC_BASE_URL=…                │ │    │
│ │ │ alias cl-mini='…'                          │ │    │
│ │ └────────────────────────────────────────────┘ │    │
│ └────────────────────────────────────────────────┘    │
│                                                        │
│ ┌─ Card: cl-mini/opencode.json (默认折叠) ─────┐    │
│ │  📄 cl-mini.json              512 B  [📋]    │    │
│ │  ~/.cc-switch-tui/opencode/cl-mini.json        │    │
│ │ ▸ Click to expand                              │    │
│ └────────────────────────────────────────────────┘    │
│                                                        │
│ ┌─ Card: cl-pro/opencode.json ────────────────┐    │
│ │  📄 cl-pro.json               480 B  [📋]    │    │
│ │  ~/.cc-switch-tui/opencode/cl-pro.json         │    │
│ │ ▸ Click to expand                              │    │
│ └────────────────────────────────────────────────┘    │
│                                                        │
│ (Cmd+Enter 触发 apply)                                │
└────────────────────────────────────────────────────────┘
```

### Apply 按钮 4 态

| 状态 | 视觉 | 文案 | 触发 |
|---|---|---|---|
| idle | 主色 + 静止 | "Apply all · 4 files" | 初始 |
| loading | 旋转 spinner | "Writing…" | apply.mutateAsync() |
| success | 绿色勾 + 短暂 1.5s | "✓ Wrote 4 files" | 200 OK |
| error | 红色边框 + retry 子按钮 | "Apply failed" + "[Retry]" | catch |

### ArtifactCard 视觉

| 区域 | 元素 |
|---|---|
| Header (可点击折叠) | icon + 文件名 + 字节数 + CopyButton + (aliases) RevealButton |
| Meta | 完整路径（truncate + title tooltip） |
| Content (折叠时) | ▸ "Click to expand" |
| Content (展开时) | monospace code block，max-h-96 overflow-y-auto |

### 交互细节

| 行为 | 实现 |
|---|---|
| 折叠/展开 | CSS `transition: grid-template-rows 200ms ease`（grid-rows trick 配合 `grid-template-rows: 0fr / 1fr`） |
| Apply 按钮状态切换 | useState 4 态机 + useEffect 重置 |
| 复制成功反馈 | CopyButton 内部 1.5s "Copied!" 状态 |
| Sticky 顶栏 | `sticky top-0 z-10 bg-background/80 backdrop-blur` |
| 键盘快捷键 | Cmd/Ctrl+Enter 触发 apply，document keydown listener |
| 滚动 | `scroll-behavior: smooth` 在 `<html>` 上 |
| Apply 成功滚动 | `scrollTo({ top: 0, behavior: 'smooth' })` 让用户看到顶部状态 |
| Hover | 卡片 `hover:shadow-md transition-shadow` |
| Loading skeleton | `<div className="animate-pulse bg-muted h-4 rounded" />` |

## Code Style

不变：React 18 函数组件 + hooks + Tailwind className + 现有 ESM 命名导出。

新约定：
- 组件内不上 prop-types（TypeScript 已经覆盖）
- 不引第三方库（除 lucide-react 如有）
- 状态机用 discriminated union：`type ApplyState = 'idle' | 'loading' | 'success' | 'error'`
- 时间相关不引 date-fns（直接 `new Date().toLocaleString()`）

## Testing Strategy

### Vitest + @testing-library/react

**`web/src/components/__tests__/CopyButton.test.tsx`**（新）：
- 点击调用 `navigator.clipboard.writeText`，按钮文字变 "Copied!"
- clipboard API 抛错时按钮文字变 "Copy failed"
- 1.5s 后恢复 idle
- 需要 mock `navigator.clipboard`

**`web/src/components/__tests__/ArtifactCard.test.tsx`**（新）：
- 默认折叠时只显示 header + path
- 点击 header 展开显示 children
- CopyButton 点击触发复制
- (aliases 类型) Reveal 按钮切换 reveal 状态

**`web/src/routes/__tests__/ApplyPage.test.tsx`**（重写）：
- 渲染：4 个 artifact 卡片（1 aliases + 3 opencode）
- 顶部 sticky Apply 按钮显示文件数
- aliases 卡片默认展开，opencode 卡片默认折叠
- 点击 Apply 调 POST /api/aliases/apply，loading → success 状态切换
- Apply 失败时显示 retry 按钮，点击重试
- Cmd+Enter 触发 apply（不需点击按钮）
- Sticky 顶栏在滚动时仍可见（无需测试，纯 CSS）

### 验证

```bash
cd web && npm run typecheck     # 0 error
cd web && npm test              # 全部 pass（预估 35-40 tests）
cd web && npm run build         # 成功
cargo test                      # 现有 64/64 继续 pass
```

### 手动验证清单

1. `make dev` → 打开 http://localhost:5173/apply
2. 看到 sticky 顶栏 + 4 张卡片
3. 滚动：顶栏 backdrop-blur 始终可见
4. 点击 aliases 卡片：折叠/展开平滑
5. 点击 opencode 卡片：展开 JSON
6. 点击 CopyButton：icon 变 ✓ + 文案变 "Copied!"
7. 点 Apply：按钮变 "Writing…" → 1.5s 后 "✓ Wrote 4 files"
8. 故意 mock 失败：按钮变红 + 显示 "Retry" 子按钮
9. 按 Cmd+Enter：触发 apply（同点击）
10. sidebar 看不到 Aliases / OpenCode 入口

## Boundaries

- **Always**: 改动前跑 typecheck + test；commit 前 build；TDD（红→绿→重构）；新组件先写测试
- **Ask first**: 引入新依赖（计划零新增）；改后端 API 形状；改 sidebar 顺序；改 apply endpoint
- **Never**: 删 ApplyPage 已有测试（重写可，删不行）；改后端 Rust 代码；改 AliasesPreview / OpencodeConfigPreview 内部实现

## Success Criteria

1. ✅ Apply 页面是预览+执行的唯一入口，sidebar 移除 Aliases / OpenCode 子 tab
2. ✅ AliasesPage.tsx、OpencodePage.tsx 及对应测试文件全部删除
3. ✅ ApplyPage 顶部 sticky 顶栏（backdrop-blur + 滚动时阴影），始终可见
4. ✅ 每个产物渲染为 ArtifactCard：icon + 路径 + 字节数 + CopyButton
5. ✅ aliases 卡片默认展开；opencode 卡片默认折叠
6. ✅ Apply 按钮 4 态切换：idle → loading → success/error
7. ✅ 错误时显示 retry 按钮
8. ✅ Cmd/Ctrl+Enter 触发 apply
9. ✅ CopyButton 复制成功显示 "Copied!" 反馈
10. ✅ TypeScript 0 error；Vitest 全绿（新增 8+ 测试）；Rust 64/64 继续 pass；build 成功

## Open Questions

无（已通过 explore-then-ask 阶段锁定）。
