# Plan: One-shot Apply 工作流

## 依赖图

```
T1 抽出 OpencodeConfigPreview 共享组件
   ↓
T2 新增 ApplyPage 基础骨架（路由 + 列表 + 总数）
   ↓
T3 ApplyPage 接入 apply 按钮 + 成功 / 失败 toast
   ↓
T4 AliasesPage 剥掉 Apply 按钮，加 "→ Apply" 链接
   ↓
T5 OpencodePage 剥掉 Apply 按钮，加 "→ Apply" 链接
   ↓
T6 App.tsx 加 /apply 路由 + sidebar Apply 入口
   ↓
T7 端到端验证（typecheck + vitest + build + cargo test）
```

T1 → T2 → T3（顺序：先有共享预览组件 → 才有 ApplyPage → 才有按钮行为）
T4 / T5 互相独立，可与 T3 并行
T6 必须在 T2 之后（ApplyPage 存在才有路由）

## Task 列表

### T1. 抽出 OpencodeConfigPreview 共享组件

**目标**：复用 `OpencodePage` 中的 `<pre>{JSON.stringify(config, null, 2)}</pre>` 模式到 `ApplyPage`，避免重复。

**改动**：
- 新增 `web/src/components/OpencodeConfigPreview.tsx`
- Props: `{ config: Record<string, unknown>; collapsed?: boolean }`
- 用现有 `<pre>` + max-height + overflow 样式（从 `OpencodePage.tsx:102` 复制）
- 折叠状态由外部传入（`ApplyPage` 控制）

**验收**：
- 组件能独立 render，给 `config` prop 显示 JSON
- 折叠 prop 为 true 时显示 "[click to expand]" 提示而非 JSON

**验证**：
- `npm test -- OpencodeConfigPreview` 通过
- 手动：`make dev` 打开 /opencode 验证视觉无变化

**文件**：`web/src/components/OpencodeConfigPreview.tsx`（新）

---

### T2. 新增 ApplyPage 基础骨架

**目标**：路由 `/apply` 可访问，显示所有产物列表 + 总数。

**改动**：
- 新增 `web/src/routes/ApplyPage.tsx`
- 列表展示：
  - `~/.cc-switch-tui/aliases.zsh`（默认展开，引用 `AliasesPreview`）
  - `~/.cc-switch-tui/opencode/{alias}.json` × N（默认折叠，引用 `OpencodeConfigPreview`）
- 数据获取：
  - `useAliasesContent()` → aliases 文本
  - `useInstances()` → 拿 instance id 列表
  - 每个 instance 调 `useOpencodeConfig(id)` 拉 config
- 顶部头部："Apply 全部产物" + 副标题 "即将写入 N 个文件到 ~/.cc-switch-tui/"

**验收**：
- 打开 `/apply` 能看到所有产物名
- 展开 / 折叠 opencode JSON 可用
- aliases.zsh 默认展开（带 Reveal secrets 按钮）

**验证**：
- `npm test -- ApplyPage` 至少 1 个测试通过（render 测试）
- 手动：访问 `/apply` 确认渲染

**文件**：`web/src/routes/ApplyPage.tsx`（新）

---

### T3. ApplyPage 接入 Apply 按钮 + 成功 / 失败反馈

**目标**：单按钮 apply 全部产物，结果可视化。

**改动**：
- 复用 `useApplyAliases()` hook
- 按钮：底部 sticky / 普通定位，文案 `Apply All (N files)`，pending 时 `Applying…`
- 成功后：
  - 顶部绿色 toast：`✓ Wrote N files`（3 秒后自动消失）
  - 调 `qc.invalidateQueries({ queryKey: ['aliases', 'content'] })` + `['opencode-config']`
- 失败后：
  - 红色 banner：`✗ Apply failed: {err.message}`
  - 按钮恢复可点
- `serverError` 状态用 `useState<unknown>(null)` 临时存

**验收**：
- 点击按钮 → 调 POST /api/aliases/apply
- 成功 → 看到 toast → 3 秒消失
- 失败 → 看到 banner，按钮可重试

**验证**：
- `npm test -- ApplyPage` 4 个测试：render / apply success / apply failure / collapsed default
- 手动：`make dev` + DevTools Network 看到 POST 请求

**文件**：`web/src/routes/ApplyPage.tsx`（改）

---

### T4. AliasesPage 剥掉 Apply 按钮（只读预览）

**目标**：AliasesPage 顶部 Apply 按钮移除，改为底部跳 ApplyPage 链接。

**改动**：
- 删 `<button onClick={handleApply}>Apply</button>` + `useApplyAliases()` 引用 + `lastApplyResult` state
- 底部加链接："想写入所有产物？→ Apply"
- `useAliasesContent()` 保留（预览仍需要）
- 保留 `AliasesPreview` 组件 + Reveal secrets 逻辑

**验收**：
- AliasesPage 无 Apply 按钮
- 链接点击跳 `/apply`
- aliases.zsh 文本预览功能不变

**验证**：
- `npm test -- AliasesPage` 1 个测试：queryByRole('button', { name: /apply/i }) 为 null
- 手动：访问 /aliases 确认无 Apply 按钮

**文件**：`web/src/routes/AliasesPage.tsx`（改）

---

### T5. OpencodePage 剥掉 Apply 按钮（只读预览 + 跳 Apply）

**目标**：OpencodePage 右侧 Apply 按钮移除，改为底部跳 ApplyPage 链接。

**改动**：
- 删 `<button onClick={handleApply}>Apply</button>` + `useApplyOpencodeConfig()` 引用 + `lastApplyResult` state
- 保留 instance 列表（左侧可点击切换预览）
- 保留 JSON 预览
- 底部加链接："想写入所有产物？→ Apply"
- **额外**：替换 inline `<pre>` 为新的 `<OpencodeConfigPreview>` 组件（如果 T1 抽出来了）

**验收**：
- OpencodePage 无 Apply 按钮
- 链接点击跳 `/apply`
- instance 切换预览功能不变

**验证**：
- `npm test -- OpencodePage` 1 个测试：queryByRole('button', { name: /apply/i }) 为 null
- 手动：访问 /opencode 确认无 Apply 按钮

**文件**：`web/src/routes/OpencodePage.tsx`（改）

---

### T6. App.tsx 加 /apply 路由 + sidebar Apply 入口

**目标**：导航可达 ApplyPage。

**改动**：
- `import { ApplyPage } from './routes/ApplyPage'`
- `<Route path="/apply" element={<ApplyPage />} />`
- `<StyledNavLink to="/apply">Apply</StyledNavLink>` 放在 Opencode 之后、Config 之前

**验收**：
- 访问 `/apply` 能进入 ApplyPage
- sidebar 看到 Apply 入口
- 点击 Apply 高亮当前路由

**验证**：
- 手动 `make dev` 访问 `/apply` + 点击 sidebar 链接
- typecheck 0 error

**文件**：`web/src/App.tsx`（改）

---

### T7. 端到端验证

**目标**：所有质量门 green。

**命令**：
```bash
cd web && npx tsc --noEmit              # 0 error
cd web && npm test                       # 全部 pass
cd web && npm run build                  # 成功
cargo test --workspace                   # 64 / 64 pass（零后端改动）
```

**手动验证清单**（在 `make dev` 启动后）：
1. 打开 http://localhost:5173/apply → 看到所有产物列表
2. 点击 Apply → 看到绿色 toast
3. 终端 `ls ~/.cc-switch-tui/opencode/` → 看到 N 个 .json
4. `cat ~/.cc-switch-tui/aliases.zsh` → 看到明文 KEY
5. /aliases 页面 → 顶部无 Apply 按钮
6. /opencode 页面 → 右侧无 Apply 按钮
7. 折叠展开 opencode JSON → 状态切换正确
8. 失败路径：mock 失败时 ApplyPage 红色 banner 显示

**验收**：上面 8 步全部通过。

---

## Checkpoints

| 节点 | 完成 | 风险 |
|---|---|---|
| **CP1**（T1 + T2 完成） | 共享组件 + ApplyPage 骨架 | 视觉差异：JSON 折叠展开与 OpencodePage 现有预览对齐 |
| **CP2**（T3 + T4 + T5 完成） | Apply 工作流闭环 | toast 自动消失：3s 时长合理；mock 测试不要 over-specify |
| **CP3**（T6 + T7 完成） | 路由 + 全绿 | 零后端改动 = cargo test 应该无 diff |

## 风险登记表

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| ApplyPage 调 N 次 useOpencodeConfig(id) 触发 N+1 请求 | 高 | 低（GET 可缓存 + TanStack 自动 dedup） | 接受；如有性能问题可加 batch endpoint（M1+） |
| OpencodeConfigPreview 组件 props 选错 | 中 | 中 | 写组件时与现有 OpencodePage 的 `<pre>` 样式 1:1 复制 |
| 折叠状态丢失（切路由后） | 中 | 低 | 接受；用户体验"切走再回来看默认折叠"反而是好事 |
| AliasesPreview 在 ApplyPage 中复用，prop 冲突 | 低 | 中 | 组件只接受 `content` prop，无冲突 |

## 文件清单

**新增（2）**：
- `web/src/routes/ApplyPage.tsx`
- `web/src/components/OpencodeConfigPreview.tsx`

**修改（4）**：
- `web/src/routes/AliasesPage.tsx`
- `web/src/routes/OpencodePage.tsx`
- `web/src/App.tsx`
- `web/src/components/__tests__/` （新增测试文件）

**测试新增（3）**：
- `web/src/routes/__tests__/ApplyPage.test.tsx`
- `web/src/routes/__tests__/AliasesPage.test.tsx`
- `web/src/routes/__tests__/OpencodePage.test.tsx`

**零后端改动**。

## 实施顺序

1. T1（共享组件）
2. T2（ApplyPage 骨架）
3. T6（路由 + 入口）— 让 ApplyPage 可访问
4. T3（Apply 按钮 + 反馈）
5. T4 + T5（剥掉旧 Apply 按钮，可并行）
6. T7（端到端验证）
