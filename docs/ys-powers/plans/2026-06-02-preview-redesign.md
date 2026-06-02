# Plan: Preview 页面 UI 重新设计

依据 spec: `docs/ys-powers/specs/2026-06-02-preview-redesign-design.md`
Workspace: 直接当前分支

## 任务依赖图

```
T1 删 AliasesPage/OpencodePage
  ↓
T2 新增 CopyButton        (与 T1 并行)
  ↓                        ↓
T3 重写 ApplyPage 布局    (需 T1+T2 完成后)
  ↓
T4 重写 ApplyPage 测试    (RED 必须在 T3 之前；这里 T3 内嵌 TDD 循环)
  ↓
T5 Chrome DevTools MCP 端到端验证
  ↓
T6 code-simplify pass
```

**实施顺序**：T1 → T2 → T3 (内含 T4 测试) → T5 → T6

## 任务清单

### T1: 删除 AliasesPage + OpencodePage
**依赖**：无
**Files**：
- `web/src/routes/AliasesPage.tsx` (删)
- `web/src/routes/OpencodePage.tsx` (删)
- `web/src/routes/__tests__/AliasesPage.test.tsx` (删)
- `web/src/routes/__tests__/OpencodePage.test.tsx` (删)
- `web/src/App.tsx` (改：移除 import + Route + NavLink + 1 个 separator span)

**Acceptance**：
- 4 个文件已删除
- `App.tsx` sidebar 剩下 4 个入口：Instances · Apply · Config · Settings
- `App.tsx` `<Routes>` 剩下 5 个路径：/, /instances/:id, /apply, /config, /settings
- 直接访问 `/aliases` 或 `/opencode` 返回 React Router 404（无害）

**Verify**：
- `grep -r "AliasesPage\|OpencodePage" web/src/` 仅出现在 import 旧 commit（git history），当前工作区无残留
- `cd web && npm run typecheck` 0 error
- `cd web && npm test` 通过（ApplyPage 4 个旧测试若在 T3 之前已删，临时跑 aliases/opencode 相关的 4 个测试就 fail，需 T1 commit 紧接着 T3 commit 修复，否则单独测会红）

**风险**：T1 单独 commit 后 Vitest 会 fail。**缓解**：T1 commit 消息明确写 "BREAKING: removes /aliases and /opencode routes"，与 T3 一起 push 即可。

---

### T2: 新增 CopyButton 组件
**依赖**：无（与 T1 并行）
**Files**：
- `web/src/components/CopyButton.tsx` (新)
- `web/src/components/__tests__/CopyButton.test.tsx` (新)

**Acceptance**：
- 组件签名 `<CopyButton text={string} />`
- 点击调用 `navigator.clipboard.writeText(text)`
- 成功后按钮显示 "Copied!" 1.5s 后恢复 "Copy"
- 失败后按钮显示 "Copy failed" 1.5s 后恢复
- clipboard API 不可用时（mock undefined）走降级路径
- 测试用 `vi.spyOn(navigator.clipboard, 'writeText')` 验证

**Verify**：
- TDD：先写 3 个测试（成功 / 失败 / clipboard 不可用）→ 看红 → 实现 → 看绿
- `cd web && npm test -- CopyButton` 3 个测试 pass

**设计草图**：
```tsx
export function CopyButton({ text, label = 'Copy' }: { text: string; label?: string }) {
  const [state, setState] = useState<'idle' | 'copied' | 'failed'>('idle');
  useEffect(() => {
    if (state === 'idle') return;
    const t = setTimeout(() => setState('idle'), 1500);
    return () => clearTimeout(t);
  }, [state]);
  async function handle() {
    try {
      if (!navigator.clipboard) throw new Error('clipboard unavailable');
      await navigator.clipboard.writeText(text);
      setState('copied');
    } catch {
      setState('failed');
    }
  }
  return <button onClick={handle}>{state === 'copied' ? '✓ Copied' : state === 'failed' ? '✗ Failed' : label}</button>;
}
```

---

### T3: 重新设计 ApplyPage 布局
**依赖**：T1, T2
**Files**：
- `web/src/components/ArtifactCard.tsx` (新)
- `web/src/components/__tests__/ArtifactCard.test.tsx` (新)
- `web/src/routes/ApplyPage.tsx` (改写)
- `web/src/routes/__tests__/ApplyPage.test.tsx` (改写)

**Acceptance**：

#### ArtifactCard
- Props: `{ title, path, size, defaultOpen, children, onCopy? }`
- Header: icon + title + size badge + CopyButton
- Subline: 完整 path（truncate）
- Body: 折叠时 `▸ Click to expand`，展开时渲染 children
- 折叠/展开用 CSS `grid-template-rows` 0fr/1fr + transition

#### ApplyPage
- 顶部 sticky 顶栏：`sticky top-0 z-10 bg-background/80 backdrop-blur`
  - 内含：Apply 按钮（4 态）+ 文件数 + 滚动阴影
- 主体：1 + N 张 ArtifactCard（aliases + 每个 instance 一个）
- aliases 卡片：默认展开
- opencode 卡片：默认折叠
- Apply 按钮状态机：
  - `idle`: 蓝色 + "Apply all · 4 files"
  - `loading`: spinner + "Writing…"
  - `success`: 绿色勾 + "✓ Wrote 4 files" 1.5s
  - `error`: 红色边框 + "Apply failed" + `[Retry]`
- Cmd/Ctrl+Enter 触发 apply
- 成功后 invalidate aliases + opencode-config queries

**Verify**：
- 视觉：Chrome DevTools MCP 截图验证
- TDD：测试覆盖
  - ApplyPage 渲染 N+1 卡片
  - 顶部 sticky 顶栏存在
  - aliases 卡片默认展开
  - Apply 4 态切换
  - 错误显示 Retry
  - Cmd+Enter 触发
- `cd web && npm run typecheck` 0 error
- `cd web && npm test` 全绿
- `cd web && npm run build` 成功
- Chrome DevTools MCP 验证 10 步手动清单（spec 末尾）

---

### T5: Chrome DevTools MCP 端到端验证
**依赖**：T3 完成
**Files**：无（验证步骤）

**Acceptance**：
- 启 dev server (`make dev` 或 `cd web && npm run dev`)
- 用 Chrome DevTools MCP 走完 spec 末尾 10 步验证
- 截屏 3 张：sticky 顶栏、展开 JSON 卡片、错误 retry 状态
- 报告结论：哪些通过、哪些需要补救

**Verify**：
- 截图存在
- 所有交互均按 spec 工作

---

### T6: code-simplify pass
**依赖**：T5
**Files**：本轮所有新增 / 改写文件

**Acceptance**：
- ApplyPage、ArtifactCard、CopyButton 三个组件 + 测试
- 扫描代码气味：长函数、嵌套三元、重复逻辑
- 简化机会评估：是否值得抽 hook、是否值得合并子组件
- 应用 1-2 个有意义的 simplify（不是无脑重写）
- 全测试 + build 仍通过

**Verify**：
- `cd web && npm run typecheck && npm test && npm run build` 全部通过
- 至少 1 个 simplify commit 留下

---

## 检查点 (Checkpoints)

### CP1: T1+T2 完成
- [x] AliasesPage/OpencodePage 已删
- [x] CopyButton 3 测试 pass
- **门槛**：Vitest 全绿（ApplyPage 测试暂时随 ApplyPage 重写一起恢复可接受，但理想情况 T1 commit 应独立可测；这里选择 T1→T3 连续 commit 避免中间红）

### CP2: T3 完成
- [x] ApplyPage 重写完成，sticky + ArtifactCard + 4 态 + Cmd+Enter
- [x] 所有测试 pass，build 成功
- **门槛**：可以本地起 dev server 看到效果

### CP3: T5+T6 完成
- [x] 手动 10 步验证通过
- [x] code-simplify commit
- **门槛**：可以 PR / merge

## 风险登记表

| 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|
| T1 单独 commit 后 Vitest 红 | 高 | 低 | T1→T3 连续 commit，不单独 push |
| Sticky 顶栏在老 Safari 不支持 backdrop-blur | 低 | 中 | 加 fallback `bg-background/95`，无 backdrop 时仍是 95% 不透明 |
| 键盘 Cmd+Enter 与浏览器冲突（如 Cmd+Enter 已有默认行为） | 低 | 低 | 监听器加 `e.preventDefault()`；React 不会触发到 textarea 等 |
| Apply 按钮 4 态机容易写出"成功后又变 loading"的 flicker | 中 | 中 | success 1.5s 内 disable apply，timeout 后再回 idle |
| ArtifactCard 内嵌 AliasesPreview 后折叠时仍渲染 preview（性能） | 中 | 低 | 用 `grid-template-rows: 0fr` 真正折叠，DOM 仍渲染但不可见 |
| 卡片路径过长截断后用户看不到完整路径 | 低 | 低 | 加 `title={path}` tooltip 兜底 |

## 验证总结

```bash
# 每次 commit 前
cd web && npm run typecheck && npm test

# 每个 phase 结束
cd web && npm run build

# 整体结束
cd web && npm test              # 35-40 tests pass
cargo test                      # 64/64 pass（零后端改动）
```

## 总工作量估算

- T1: 5 min（删文件 + 改 App.tsx）
- T2: 15 min（CopyButton 简单）
- T3: 60-90 min（重头戏）
- T5: 15 min（MCP 验证）
- T6: 10-20 min

**总计**：~2-2.5 小时，1 个 session 可完成。
