# Plan: Web 前端 + Rust 后端架构改写（实现计划）

> 配套 spec: `docs/ys-powers/specs/2026-06-02-web-replaces-tui-design.md`
> 分支: `feat/web-replaces-tui`
> 计划原则: **vertical slicing** — 每个 slice 是一个端到端的用户可见功能（DAO → API → React → 验证），不是 spec 里按层划分的 horizontal phase。
> **切片总数：10 个**（含 dark mode / 搜索 / Test Connection / Import-Export 增强）

---

## 1. 依赖图（组件视角）

```
                          ┌────────────────────────┐
                          │     S0: 基础设施         │ ◀── 全部依赖
                          │  Hello World 端到端     │
                          │  (含 dark mode 开关)    │
                          └────────────────────────┘
                                     │
            ┌────────────────────────┼────────────────────────┬──────────────┐
            │                        │                        │              │
            ▼                        ▼                        ▼              ▼
    ┌──────────────┐          ┌──────────────┐          ┌──────────────┐ ┌──────────────┐
    │ S1: 列表读     │          │ S5: Aliases  │          │ S7: 配置       │ │ S10: 进程模型 │
    │ + 搜索/过滤   │          │   预览+Apply │          │ Import/Export │ │ Port/Browser │
    └──────────────┘          └──────────────┘          └──────────────┘ └──────────────┘
            │                        │                        │
            ▼                        ▼                        │
    ┌──────────────┐          ┌──────────────┐                │
    │ S2: 创建      │          │ S6: OpenCode │                │
    │ Instance     │          │   预览+Apply │                │
    └──────────────┘          └──────────────┘                │
            │                                                  │
            ▼                                                  │
    ┌──────────────┐                                          │
    │ S3: 编辑/删除 │  + Test Connection 按钮                  │
    │   /复制       │                                          │
    └──────────────┘                                          │
            │                                                  │
            ▼                                                  │
    ┌──────────────┐                                          │
    │ S4: Templates│                                          │
    │  接入表单     │──────────────────────────────────────────┘
    └──────────────┘

    S1─S4 构成 Phase B（核心 CRUD）
    S5+S6+S7 构成 Phase C（派生内容 + 配置迁移）
    S8: Settings + Diagnostics
    S9: 删除 TUI（必须最后）
```

**关键依赖**：
- **S0 是地基**：所有后续 slice 复用其 build chain / routing / error model / client 框架；S0 同时落 dark mode
- **S1 → S2 → S3 → S4 是有依赖的链**：先读、再写、再编辑删除、再丰富表单
- **S5/S6/S7 互相独立**：都只依赖 S0，可并行
- **S8 独立**：只依赖 S0
- **S9 必须最后**：删除 TUI 后没有回退路径（除非 git reset）
- **S10 可在 S0 后任何时间插入**：进程模型和功能开发正交

---

## 2. Vertical Slices

### S0: Hello World 端到端

**目标**：单文件二进制启动 → 自动开浏览器 → 看到 "Hello cc-switch" 页面 → 页面调用 `/api/health` → 显示 "ok"。同时把 **dark mode 基础设施**落到位（开关可工作，主题跟随系统）。

**触摸点**：
- `Cargo.toml`（加 axum / tokio / include_dir / webbrowser / serde_json 依赖）
- `web/package.json`（vite / react / react-dom / typescript / @tanstack/react-query / react-router-dom / tailwindcss / shadcn 依赖）
- `web/vite.config.ts`（带 `/api` proxy → :7480）
- `web/tsconfig.json`
- `web/tailwind.config.ts` + `web/postcss.config.js`
- `web/index.html` + `web/src/main.tsx` + `web/src/App.tsx`（Hello 占位）
- `Makefile`（dev / web-build / build / release 全部 target）
- `src/main.rs`（重写：找端口 → 启 axum → embed fallback → open browser）
- `src/lib.rs`（加 `pub mod api;`）
- `src/api/mod.rs`（Router 装配）
- `src/api/error.rs`（`ApiError` 枚举 + `IntoResponse` 实现）
- `src/api/health.rs`（`GET /api/health`）
- `src/api/state.rs`（`AppState` 占位）
- `tests/api/mod.rs`（helper: `spawn_test_app`）
- `tests/api/health_test.rs`（一个端点测试）

**任务清单**：
- [ ] **S0-T1**: 初始化前端骨架（`web/` + Vite + React + TS + Tailwind）
  - Acceptance: `cd web && npm install && npm run dev` 启动 Vite，看到 "Hello"
  - Verify: 浏览器访问 5173 显示 "Hello cc-switch"
  - Files: `web/package.json`, `web/vite.config.ts`, `web/tsconfig.json`, `web/tailwind.config.ts`, `web/postcss.config.js`, `web/index.html`, `web/src/main.tsx`, `web/src/App.tsx`, `web/src/styles/globals.css`

- [ ] **S0-T2**: Makefile + 全部 target（dev / web-build / build / release / test）
  - Acceptance: `make dev` / `make web-build` / `make release` / `make test` 全部可跑
  - Verify: 跑一遍所有 target 无报错（功能可空）
  - Files: `Makefile`

- [ ] **S0-T3**: Rust 端接入 axum + `include_dir!` + SPA fallback
  - Acceptance: `cargo build` 嵌入 `web-dist/`，`main.rs` 启动 axum 监听 127.0.0.1 + 自动开浏览器
  - Verify: `make release` 跑通，启动二进制，浏览器自动打开看到 "Hello" 页面
  - Files: `Cargo.toml`, `src/main.rs`, `web-dist/index.html`（Vite build 产物占位）

- [ ] **S0-T4**: 实现 `/api/health` + 统一 ApiError
  - Acceptance: `GET /api/health` 返回 `{status: "ok", version: "0.4.0"}` 200；非 `/api/*` 全部走 SPA fallback
  - Verify: `cargo test api::health` 通过 + 浏览器访问二进制端口 `/api/health` 看到 JSON
  - Files: `src/api/mod.rs`, `src/api/error.rs`, `src/api/health.rs`, `src/api/state.rs`, `tests/api/mod.rs`, `tests/api/health_test.rs`

- [ ] **S0-T5**: React 端接入 TanStack Query + 调用 `/api/health` 显示 "ok"
  - Acceptance: 进入页面后 200ms 内看到 "Backend: ok"
  - Verify: 浏览器 DevTools Network 看到 `/api/health` 200，UI 显示 ok
  - Files: `web/src/main.tsx`, `web/src/api/client.ts`, `web/src/api/hooks.ts`, `web/src/api/types.ts`, `web/src/App.tsx`

- [ ] **S0-T6**: Dark mode 基础设施（Tailwind `darkMode: 'class'` + CSS 变量主题 + Sun/Moon 切换按钮）
  - Acceptance: 右上角切换按钮可工作；默认跟随系统 `prefers-color-scheme`；选择持久化到 localStorage；shadcn 组件全跟随主题
  - Verify: 切换后所有 shadcn 组件颜色翻转；刷新页面后选择保留；DevTools `<html>` 元素的 `class` 跟随
  - Files: `web/tailwind.config.ts`, `web/src/styles/globals.css`（CSS 变量）, `web/src/components/ThemeToggle.tsx`, `web/src/hooks/useTheme.ts`, `web/src/App.tsx`

**Slice 验收（合并上面 6 个）**：
- `make release` 成功，产出单文件二进制
- 启动二进制后浏览器 1 秒内自动打开
- 浏览器看到 "Hello cc-switch" + "Backend: ok"
- 右上角主题切换按钮可工作（深色 / 浅色 / 跟随系统）
- 关闭浏览器，二进制仍在跑（Ctrl-C 退出）
- `cargo test` 至少 1 个 health 测试通过

---

### S1: 列出 Instance（只读 + 搜索/过滤）

**目标**：用户能在 `/` 看到一个表格，列出所有 instance，列包括 alias / template / model / isDefault / kvCache 标记 / 操作按钮（占位）。表格上方有搜索框，按 alias / template / model 实时过滤（**客户端过滤**，无 API 改动）。

**触摸点**：
- `src/api/instances.rs`（`GET /api/instances`）
- `tests/api/instances_test.rs`（列表测试）
- `web/src/api/hooks.ts`（`useInstances`）
- `web/src/api/types.ts`（`Instance` 类型）
- `web/src/routes/InstancesPage.tsx`（表格）
- `web/src/components/InstancesTable.tsx`（表格组件）
- `web/src/components/ApiErrorBanner.tsx`（错误条幅）

**任务清单**：
- [ ] **S1-T1**: Rust 端 `GET /api/instances` + 集成测试
  - Acceptance: 返回 instance 列表（apiKey 字段省略），空 DB 返回 `[]`
  - Verify: `cargo test api::instances::test_list` 通过
  - Files: `src/api/instances.rs`, `tests/api/instances_test.rs`

- [ ] **S1-T2**: React 端 `useInstances` hook + Instance 类型
  - Acceptance: 组件 mount 后 200ms 内拉取，loading 态正常
  - Verify: 手动在浏览器看到表格，devtools 看到 `/api/instances` 200
  - Files: `web/src/api/hooks.ts`, `web/src/api/types.ts`, `web/src/api/hooks.test.ts`

- [ ] **S1-T3**: InstancesPage + InstancesTable 组件 + shadcn/ui Table
  - Acceptance: 表格 6 列渲染，isDefault 高亮标记，kvCache 有/无标记，操作按钮占位
  - Verify: 组件测试覆盖空列表、有数据两态；手动看 UI
  - Files: `web/src/routes/InstancesPage.tsx`, `web/src/components/InstancesTable.tsx`, `web/src/components/ui/table.tsx`（shadcn）

- [ ] **S1-T4**: 顶部 "新建" 按钮（占位，弹 toast "TODO: S2"）+ ApiErrorBanner
  - Acceptance: 点击按钮 toast 提示；API 错误时顶部条幅显示
  - Verify: 手动 + 组件测试
  - Files: `web/src/components/ApiErrorBanner.tsx`, `web/src/routes/InstancesPage.tsx`

- [ ] **S1-T5**: 搜索/过滤框（客户端过滤，匹配 alias / template / model 三个字段，大小写不敏感）
  - Acceptance: 输入 "minimax" 只显示 template 含 "minimax" 的行；输入 "M2.7" 显示所有用 M2.7 模型的行；清空恢复全量
  - Verify: 组件测试覆盖：空查询、命中、部分命中、清空
  - Files: `web/src/components/InstancesTable.tsx`, `web/src/hooks/useInstanceFilter.ts`, `web/src/components/InstancesTable.test.tsx`

**Slice 验收**：
- 浏览器看 DB 里现有的 instance（如果有），列表完整
- 搜索框输入能实时过滤列表
- 没有任何 apiKey 字段显示在网络响应里
- API 错误时（如 500）顶部出现红色条幅
- `cargo test` + `npm test` 全部通过

---

### S2: 创建 Instance

**目标**：用户点 "新建" → 弹 Dialog → 填 alias / template / apiKey 等 → 提交后看到列表里多了一行。

**触摸点**：
- `src/api/instances.rs`（`POST /api/instances` + alias 冲突 409）
- `tests/api/instances_test.rs`（增加创建/冲突测试）
- `web/src/lib/validate.ts`（Zod schema）
- `web/src/api/hooks.ts`（`useCreateInstance`）
- `web/src/components/InstanceForm.tsx`（受控表单）
- `web/src/components/InstanceFormDialog.tsx`（Dialog 包装）
- `web/src/components/SecretInput.tsx`（apiKey 输入）

**任务清单**：
- [ ] **S2-T1**: Rust 端 `POST /api/instances` + alias 冲突 409 + 校验
  - Acceptance: 成功 201 + 完整 instance；alias 重复 409 + 错误 field；alias 不合规 400
  - Verify: `cargo test api::instances` 至少 4 个新测试通过
  - Files: `src/api/instances.rs`, `tests/api/instances_test.rs`

- [ ] **S2-T2**: React Zod schema（alias 复用现有规则 + 必填 + 长度）
  - Acceptance: 大写/空白 alias 提交时阻止
  - Verify: `npm test lib/validate` 至少 5 个测试通过
  - Files: `web/src/lib/validate.ts`, `web/src/lib/validate.test.ts`

- [ ] **S2-T3**: React 端 `useCreateInstance` mutation + 错误处理
  - Acceptance: 成功后 refetch 列表；409 错误回传 `error.field`；500 显示条幅
  - Verify: MSW mock 测试覆盖成功/409/500 三态
  - Files: `web/src/api/hooks.ts`, `web/src/api/hooks.test.ts`

- [ ] **S2-T4**: InstanceForm 组件（alias / template / apiKey / kvCache / model 等字段）
  - Acceptance: 字段全部受控，Zod 校验实时提示，提交按钮在 invalid 时禁用
  - Verify: 组件测试覆盖空状态、填写一半、提交成功三态
  - Files: `web/src/components/InstanceForm.tsx`, `web/src/components/SecretInput.tsx`, `web/src/components/InstanceForm.test.tsx`

- [ ] **S2-T5**: InstanceFormDialog + 接到 InstancesPage 的"新建"按钮
  - Acceptance: 点 "新建" 弹 Dialog；ESC 关闭；提交后 Dialog 关闭 + 列表更新
  - Verify: 组件测试 + 手动端到端（在浏览器真创建一条）
  - Files: `web/src/components/InstanceFormDialog.tsx`, `web/src/routes/InstancesPage.tsx`

**Slice 验收**：
- 浏览器真创建一个 instance，DB 里立即可见
- alias 重复时表单 alias 字段下方显示红字
- apiKey 在网络响应中只出现在 POST 请求体和返回详情中（不出现在 GET 列表）
- Dialog 关闭后列表自动 refresh

---

### S3: 编辑 / 删除 / 复制 Instance

**目标**：用户能编辑已存在的 instance、删除、复制（一键基于现有创建新）。

**触摸点**：
- `src/api/instances.rs`（PATCH / DELETE / POST /:id/duplicate）
- `tests/api/instances_test.rs`（增加对应测试）
- `web/src/api/hooks.ts`（`useUpdateInstance` / `useDeleteInstance` / `useDuplicateInstance`）
- `web/src/routes/InstanceDetailPage.tsx`（新页面 `/instances/:id`）
- `web/src/components/ConfirmDialog.tsx`（删除确认）
- `web/src/routes/InstancesPage.tsx`（行操作按钮接通）

**任务清单**：
- [ ] **S3-T1**: Rust 端 PATCH / DELETE / duplicate + 错误处理
  - Acceptance: PATCH 部分字段成功；DELETE 默认且无替补返 400；duplicate alias 加 `-copy` 后缀冲突时返 409
  - Verify: `cargo test api::instances` 至少 6 个新测试
  - Files: `src/api/instances.rs`, `tests/api/instances_test.rs`

- [ ] **S3-T2**: React 端 mutations + TanStack Query invalidation
  - Acceptance: 编辑/删除/复制成功后 refetch 列表
  - Verify: MSW mock 测试
  - Files: `web/src/api/hooks.ts`, `web/src/api/hooks.test.ts`

- [ ] **S3-T3**: InstanceDetailPage（编辑表单 + 删除按钮 + 离开拦截）
  - Acceptance: 表单预填，保存 PATCH；点删除弹确认；离开未保存弹确认
  - Verify: 组件测试 + 手动端到端
  - Files: `web/src/routes/InstanceDetailPage.tsx`, `web/src/components/ConfirmDialog.tsx`, `web/src/hooks/useUnsavedGuard.ts`

- [ ] **S3-T4**: InstancesPage 行操作按钮接通（编辑/复制/删除）
  - Acceptance: 三按钮均可点击；删除走 ConfirmDialog；复制成功后 toast 提示
  - Verify: 组件测试 + 手动
  - Files: `web/src/routes/InstancesPage.tsx`, `web/src/components/InstancesTable.tsx`

- [ ] **S3-T5**: Test Connection 按钮（`POST /api/instances/:id/test`，调 provider 健康检查端点，验证 apiKey + baseUrl + 网络）
  - Acceptance:
    - 按钮在 detail 页底部，loading 态显示 spinner
    - 成功：绿色 toast "Connected in 234ms"
    - 失败：红色条幅显示具体错误（401 / 403 / timeout / DNS）
    - 后端用 provider template 的 `test_endpoint`（若无则用 `default_base_url + /v1/messages` 之类通用探测）
  - Verify: 集成测试 mock ureq（成功 / 401 / timeout 三态）；手动用真实 key 验证
  - Files: `src/api/instances.rs`（新增 test handler）, `src/domain/template.rs`（加 `test_endpoint` 字段）, `src/app/templates.rs`（给 minimax/kimi 填 test_endpoint）, `tests/api/instances_test.rs`, `web/src/components/TestConnectionButton.tsx`, `web/src/api/hooks.ts`（useTestConnection）

**Slice 验收**：
- 完整 CRUD 闭环：新建 → 编辑 → 复制 → 删除都能在浏览器完成
- 默认 instance 单独删除被阻止（前端按钮禁用 + 后端 400 兜底）
- 离开未保存页面有确认提示
- Test Connection 按钮能验证 key 有效性，错误有明确提示

---

### S4: Templates 接入表单

**目标**：InstanceForm 的 template / model 下拉从 `/api/templates` 拉取，而不是硬编码。

**触摸点**：
- `src/api/templates.rs`（`GET /api/templates`）
- `tests/api/templates_test.rs`
- `web/src/api/hooks.ts`（`useTemplates`）
- `web/src/components/InstanceForm.tsx`（下拉用 hook 数据）

**任务清单**：
- [ ] **S4-T1**: Rust 端 `/api/templates` + 集成测试
  - Acceptance: 返回 `ProviderTemplate` 列表，camelCase 序列化
  - Verify: `cargo test api::templates` 通过
  - Files: `src/api/templates.rs`, `tests/api/templates_test.rs`

- [ ] **S4-T2**: React `useTemplates` + 接入表单下拉
  - Acceptance: template 切换后 model 下拉自动更新到该 template 的 models
  - Verify: 组件测试 + 手动
  - Files: `web/src/api/hooks.ts`, `web/src/components/InstanceForm.tsx`

**Slice 验收**：
- 表单下拉显示所有真实 templates
- model 下拉根据 template 切换联动
- 修改 template 后 baseUrl 自动用 template 的 default

---

### S5: Aliases 预览 + Apply

**目标**：用户在 `/aliases` 看到一个等宽 textarea 显示完整 `aliases.zsh` 内容，点 "Apply" 写入 `~/.cc-switch-tui/aliases.zsh`。

**触摸点**：
- `src/api/aliases.rs`（`GET /api/aliases` + `POST /api/aliases/apply`）
- `tests/api/aliases_test.rs`
- `web/src/routes/AliasesPage.tsx`（新页面）
- `web/src/api/hooks.ts`（`useAliasesContent` / `useApplyAliases`）
- `web/src/lib/format.ts`（mtime 比较工具）

**任务清单**：
- [ ] **S5-T1**: 重构 `shell::generate_aliases` → 拆为 `render_aliases`（返回 String）和 `write_aliases`（写文件）
  - Acceptance: 现有 `shell::generate_aliases` 调用方不破坏（TUI 阶段可能仍要用），新增两个纯函数
  - Verify: 现有测试 + 新增 render_aliases 单测
  - Files: `src/shell.rs`

- [ ] **S5-T2**: Rust 端 `GET /api/aliases` + `POST /api/aliases/apply` + 集成测试
  - Acceptance: GET 返回 `text/plain`；apply 成功返路径；磁盘只读返 500 IO_ERROR
  - Verify: `cargo test api::aliases` 通过
  - Files: `src/api/aliases.rs`, `tests/api/aliases_test.rs`

- [ ] **S5-T3**: React AliasesPage 页面（textarea + Apply 按钮 + mtime 状态指示）
  - Acceptance: 实时显示内容；点 Apply 写文件 + toast 提示；按钮在 loading 时禁用
  - Verify: 组件测试 + 手动（Apply 后 `cat ~/.cc-switch-tui/aliases.zsh` 验证）
  - Files: `web/src/routes/AliasesPage.tsx`, `web/src/routes/AliasesPage.test.tsx`, `web/src/lib/format.ts`

**Slice 验收**：
- 浏览器看 aliases.zsh 完整内容（与 TUI 输出一致）
- Apply 写入成功，文件 mtime 更新
- 错误情况（zshrc 不可写）顶部条幅显示

---

### S6: OpenCode 预览 + Apply

**目标**：用户在 `/opencode` 左侧选 instance，右侧显示该 instance 的 OpenCode 配置 JSON，点 "Apply" 写入 OpenCode 配置文件。

**触摸点**：
- `src/api/opencode.rs`（`GET /api/opencode-config/:id` + `POST .../apply`）
- `tests/api/opencode_test.rs`
- `web/src/routes/OpencodePage.tsx`（新页面）
- `web/src/components/JsonViewer.tsx`（简单的 JSON 语法高亮）

**任务清单**：
- [ ] **S6-T1**: 重构 `opencode_config::generate_opencode_configs` → 拆为 `render_opencode_config` 和 `write_opencode_config`
  - Acceptance: 现有调用不破坏，新增纯函数
  - Verify: 现有测试 + 新单测
  - Files: `src/opencode_config.rs`

- [ ] **S6-T2**: Rust 端 `GET /api/opencode-config/:id` + `POST .../apply`
  - Acceptance: GET 返回 `application/json`；apply 成功返路径
  - Verify: `cargo test api::opencode` 通过
  - Files: `src/api/opencode.rs`, `tests/api/opencode_test.rs`

- [ ] **S6-T3**: React OpencodePage（左右分栏 + JSON 高亮 + Apply 按钮）
  - Acceptance: 左侧列表点击 → 右侧显示；Apply 写文件 + toast
  - Verify: 组件测试 + 手动
  - Files: `web/src/routes/OpencodePage.tsx`, `web/src/components/JsonViewer.tsx`, `web/src/routes/OpencodePage.test.tsx`

**Slice 验收**：
- 浏览器看每个 instance 的 OpenCode 配置（与 TUI 输出一致）
- Apply 写入成功
- 切换 instance 时右侧内容跟随

---

### S7: 配置 Import / Export

**目标**：用户能在 Settings 页面看到两个按钮：
- **Export**：下载当前所有 instances + settings 为 JSON 文件
- **Import**：上传 JSON 文件合并 / 替换到当前 DB

这是多设备同步、备份恢复、跨用户分享 config 的核心能力。Web 价值比 TUI 高很多。

**导出格式**（versioned 便于未来扩展）：

```json
{
  "version": 1,
  "exported_at": "2026-06-02T10:00:00Z",
  "instances": [
    {
      "id": "minimax-cl-prod",
      "templateId": "minimax",
      "alias": "cl-prod",
      "baseUrl": "https://api.minimax.io/anthropic",
      "modelId": "MiniMax-M2.7",
      "envOverrides": {},
      "kvCacheEnabled": false,
      "isDefault": true
    }
  ],
  "settings": {
    "defaultTemplate": "minimax",
    "autoOpenBrowser": true
  }
}
```

**注**：`apiKey` **不包含**在导出文件里。导入方必须自行填 key（安全考虑 — JSON 文件可能分享到不安全的渠道）。导入时如果某 instance 已存在，弹"保留/覆盖/跳过"三选一。

**触摸点**：
- `src/api/config.rs`（新文件：`GET /api/config/export` + `POST /api/config/import`）
- `tests/api/config_test.rs`
- `web/src/routes/SettingsPage.tsx`（新增 Import/Export 区域）
- `web/src/api/hooks.ts`（`useExportConfig` / `useImportConfig`）
- `web/src/lib/importDialog.tsx`（冲突处理 Dialog：保留 / 覆盖 / 跳过）

**任务清单**：
- [ ] **S7-T1**: Rust 端 `GET /api/config/export` — 返回 versioned JSON（不含 apiKey）
  - Acceptance: 返回 200 + application/json attachment；headers 含 `Content-Disposition: attachment; filename=cc-switch-config-YYYYMMDD.json`
  - Verify: `cargo test api::config::test_export` 通过
  - Files: `src/api/config.rs`, `tests/api/config_test.rs`

- [ ] **S7-T2**: Rust 端 `POST /api/config/import` — 接受 JSON body，按 strategy 处理冲突
  - Acceptance:
    - 接受 `{strategy: "merge" | "replace"}` 两种模式
    - merge: 已存在 alias 跳过，新 alias 创建
    - replace: 全量替换（先备份当前 DB 文件到 `db.sqlite.bak.{timestamp}`）
    - 缺 apiKey 字段合法，导入后用户需手动补填
    - version 字段不识别时返 400
  - Verify: 集成测试覆盖：merge / replace / 未知 version / 缺字段
  - Files: `src/api/config.rs`, `src/dao/sqlite_impl.rs`（可能需要 bulk_insert）, `tests/api/config_test.rs`

- [ ] **S7-T3**: React 端 Export 按钮（调 hook 触发下载）
  - Acceptance: 点击后浏览器下载 JSON 文件；loading 态显示
  - Verify: 组件测试 + 手动
  - Files: `web/src/api/hooks.ts`（useExportConfig）, `web/src/components/ExportConfigButton.tsx`

- [ ] **S7-T4**: React 端 Import 按钮（file picker + 解析预览 + 冲突 Dialog）
  - Acceptance:
    - 上传后显示预览（实例数量、settings 摘要）
    - 选 merge / replace 策略
    - merge 模式下，每个冲突 alias 弹"保留/覆盖/跳过"对话框
    - 提交后 refetch 列表 + toast 提示（X 创建 / Y 跳过 / Z 覆盖）
  - Verify: 组件测试覆盖：file upload、预览、merge 冲突、replace 流程
  - Files: `web/src/components/ImportConfigButton.tsx`, `web/src/components/ImportPreviewDialog.tsx`, `web/src/components/ConflictDialog.tsx`, `web/src/api/hooks.ts`（useImportConfig）

**Slice 验收**：
- 用户点 Export 下载到 JSON 文件，可读且不含 apiKey
- 用户在另一台机器 Import 该文件，merge 模式下跳过已存在 alias，新增不存在的
- replace 模式全量替换，旧 DB 自动备份为 `db.sqlite.bak.{timestamp}`
- 导入未知 version 时报错明确

---

### S8: Settings + Diagnostics

**目标**：用户在 `/settings` 看到全局设置（default template、auto_open_browser）+ 系统诊断（DB 路径、写入权限、OpenCode 路径）。

**触摸点**：
- `src/domain/settings.rs`（如果不存在则新增）
- `src/api/settings.rs`（`GET / PUT /api/settings`）
- `src/api/diagnostics.rs`（`GET /api/diagnostics`）
- `tests/api/settings_test.rs`, `tests/api/diagnostics_test.rs`
- `web/src/routes/SettingsPage.tsx`（新页面）

**任务清单**：
- [ ] **S8-T1**: 评估 `src/domain/settings.rs` 现状（已有则用，没有则最小实现）
  - Acceptance: `Settings` 结构体 + 持久化
  - Verify: 现有测试 + 新单测
  - Files: `src/domain/settings.rs`（可能新增）, `src/dao/sqlite_impl.rs`, `src/dao/memory_impl.rs`

- [ ] **S8-T2**: Rust 端 `GET / PUT /api/settings` + `GET /api/diagnostics`
  - Acceptance: settings CRUD 工作；diagnostics 返回 4 个布尔 + 2 个路径字段
  - Verify: `cargo test api::settings` + `cargo test api::diagnostics` 通过
  - Files: `src/api/settings.rs`, `src/api/diagnostics.rs`, `tests/api/settings_test.rs`, `tests/api/diagnostics_test.rs`

- [ ] **S8-T3**: React SettingsPage（设置表单 + 诊断面板）
  - Acceptance: 改设置后 PUT 成功；诊断信息只读展示
  - Verify: 组件测试 + 手动
  - Files: `web/src/routes/SettingsPage.tsx`, `web/src/routes/SettingsPage.test.tsx`

**Slice 验收**：
- 改 default template 后保存，刷新页面仍生效
- 诊断信息准确反映当前环境（DB 路径、zshrc 路径、权限）

---

### S9: 删除 TUI（cleanup）

**目标**：删除 `ratatui` / `crossterm` 依赖 + `src/app/` + `src/ui/` + `src/event.rs` + `tools/migrate_instances_id.rs`。`main.rs` 完全重写为 web 入口。

**触摸点**：
- `Cargo.toml`（移除 `ratatui` / `crossterm`）
- `src/main.rs`（重写）
- `src/lib.rs`（移除 `pub mod app;` / `pub mod ui;` / `pub mod event;`）
- `src/app/`（删除整个目录）
- `src/ui/`（删除整个目录）
- `src/event.rs`（删除）
- `tools/migrate_instances_id.rs`（删除）

**任务清单**：
- [ ] **S9-T1**: `Cargo.toml` 移除 ratatui / crossterm
  - Acceptance: `cargo build` 成功（即使还有 TUI 代码）
  - Verify: 编译过
  - Files: `Cargo.toml`

- [ ] **S9-T2**: 删除 `src/app/` `src/ui/` `src/event.rs` `tools/migrate_instances_id.rs`
  - Acceptance: 文件已删除；`src/lib.rs` 移除对应 mod
  - Verify: `cargo build` 成功
  - Files: `src/lib.rs`, 删除目录

- [ ] **S9-T3**: `main.rs` 重写为 web 入口（之前是 S0 的占位版）
  - Acceptance: 启动 → 找端口 → 启 axum → open browser → 阻塞 Ctrl-C
  - Verify: 手动跑二进制，UI 正常加载
  - Files: `src/main.rs`

- [ ] **S9-T4**: 全量质量门
  - Acceptance: `cargo test` + `cargo clippy --all-targets -- -D warnings` + `cargo build --release` + `cd web && npm run lint` + `cd web && npm run typecheck` + `cd web && npm test` 全过
  - Verify: 跑一遍所有命令
  - Files: 无（验证步骤）

**Slice 验收**：
- `cargo build --release` 产出干净的单文件二进制
- 二进制启动后 TUI 不可达（设计如此）
- 没有任何 ratatui/crossterm 痕迹在 dependency tree

---

### S10: 进程模型打磨

**目标**：端口策略实现、port 文件写入、Ctrl-C graceful shutdown、启动时间 < 1s。

**触摸点**：
- `src/port.rs`（新增：找空闲端口）
- `src/main.rs`（接入）

**任务清单**：
- [ ] **S10-T1**: `src/port.rs` — `pick_port(cached: Option<u16>) -> Result<u16, AppError>`，从 cached 开始尝试，被占就 +1 扫描最多 100 个端口
  - Acceptance: 优先复用缓存端口；缓存被占就 +1；100 个都失败返错
  - Verify: 单元测试覆盖：可用/被占/缓存命中/全失败
  - Files: `src/port.rs`, `src/port_test.rs`（或 `#[cfg(test)]` 内嵌）

- [ ] **S10-T2**: `main.rs` 写 port 到 `~/.cc-switch-tui/port` + Ctrl-C graceful shutdown
  - Acceptance: 启动后 `~/.cc-switch-tui/port` 有值；Ctrl-C 后进程退出且 port 文件删除
  - Verify: 手动跑两次，看 port 文件复用
  - Files: `src/main.rs`

- [ ] **S10-T3**: 启动性能 — 二进制从执行到浏览器加载完成 < 1s
  - Acceptance: 计时验证
  - Verify: `time cc-switch-tui & sleep 1 && curl http://127.0.0.1:PORT/api/health` 看 200 响应时间
  - Files: 无

**Slice 验收**：
- 连续启动两次，二进制都从同一端口起（除非中间有别的进程占了）
- Ctrl-C 后 0.5s 内进程退出
- 启动到 UI 可见 < 1s

---

## 3. Phase Checkpoints

| Phase | Slices | 目标 | Checkpoint 验证 |
|---|---|---|---|
| **A. 基础设施** | S0 + S10 | 单文件二进制能起 + 健康检查 + 进程模型 + dark mode | `make release` 成功；启动后浏览器 1s 内打开 + "ok"；dark mode 切换工作；连续启动复用端口；Ctrl-C clean exit |
| **B. 核心 CRUD** | S1 + S2 + S3 + S4 | Instance 全功能管理（含 Test Connection + 搜索/过滤） | 浏览器里能完成 列表（可搜索）→ 新建 → 编辑（含 Test Connection） → 复制 → 删除；数据落 SQLite；alias 冲突友好提示；apiKey 列表脱敏 |
| **C. 派生内容 + 配置迁移** | S5 + S6 + S7 | Aliases + OpenCode 预览/Apply + Import/Export | 两个预览/Apply 页面工作；Export 出的 JSON 不含 apiKey；Import merge/replace 两种策略工作；冲突时弹"保留/覆盖/跳过" |
| **D. Settings + 清理** | S8 + S9 | 设置 + 诊断 + 删除 TUI | Settings 持久化；诊断准确；`cargo build --release` 单文件；TUI 代码全删；全套质量门通过 |

**Checkpoint 形式**：
- 每个 Phase 结束时暂停，**用户手动端到端走一遍**核心功能
- 走通后写一句 "Phase X ✅" 提交
- 走不通则记录问题、回到对应 Slice 修

---

## 4. 任务依赖矩阵

| 任务 | 依赖 | 阻塞 | 可并行 |
|---|---|---|---|
| S0 全部 | — | 所有后续 slice | S0-T1 / S0-T2 / S0-T6 内部可并行 |
| S1 | S0 | S2 | — |
| S2 | S1 | S3 | — |
| S3 | S2 | S4 | S3 内的 T3 / T4 / T5 可并行 |
| S4 | S3 | — | 可与 S5 / S6 / S7 并行 |
| S5 | S0 | — | 与 S6 / S7 完全并行 |
| S6 | S0 | — | 与 S5 / S7 完全并行 |
| S7 | S0 | — | 与 S5 / S6 完全并行 |
| S8 | S0 | — | 与 S5 / S6 / S7 并行 |
| S9 | S0 + S1-S8 全部 | — | 必须在所有其他 slice 完成后 |
| S10 | S0 | — | 任何时候 |

**最优开发顺序**：
```
S0 (基础 + dark mode)
  ↓
S1 → S2 → S3 → S4  (核心 CRUD 链，含 Test Connection + 搜索)
  ↓
S5 + S6 + S7 + S8 + S10  (并行：派生内容 + 配置迁移 + Settings + 进程)
  ↓
S9  (清理 TUI)
```

---

## 5. 风险登记

| # | 风险 | 影响 | 缓解 |
|---|---|---|---|
| R1 | `include_dir!` 嵌入大体积前端导致二进制膨胀 | 二进制可能从 5MB 涨到 20+MB | 启用 Vite minify + gzip；如太大考虑 zstd 压缩嵌入（`include_dir!` 0.7+ 支持） |
| R2 | 浏览器缓存导致前端更新不生效 | 用户更新后看到旧 UI | 给 Vite 输出加 `?v={cargo_pkg_version}` query；或者在 SPA fallback handler 强制 `Cache-Control: no-cache` |
| R3 | TypeScript 类型与 Rust serde 不一致 | 编译过但运行时空字段 | S4 完成后加 CI 脚本：对比 `web/src/api/types.ts` 与 `src/domain/*.rs` 字段（不严格一致就 fail） |
| R4 | 单 binary 启动后用户改 `web-dist/` 想重载 | 不支持（设计如此） | README 明确写"前端更新需要重 build"；提供 `make web-build && make build` 一行 |
| R5 | macOS 首次运行 quarantine | 与 TUI 版本同样问题 | 沿用 `xattr -d com.apple.quarantine` 提示，不重新解决 |
| R6 | 并发启动（脚本里调两次） | 第二个会找新端口，但 port 文件被覆盖 | S9 实现里加 file lock（`flock`）防并发；M1 可不做，挂 P2 |
| R7 | Vite dev server 与 Rust 端口冲突 7480 | 启动顺序问题 | dev 模式强制 Rust 监听 7480，Vite 5173，互不冲突 |
| R8 | Import 文件破坏现有 DB | replace 模式下错误的 JSON 覆盖一切 → 丢数据 | replace 前自动备份 `db.sqlite.bak.{timestamp}`；导入后显式列出 "X 创建 / Y 跳过 / Z 覆盖" 让用户能反查 |
| R9 | Test Connection 调用真实 provider 可能慢 / 超时 | 单次连接拖到 10s+ 阻塞 UI | 后端 5s timeout；前端 loading 态可中断；用 ureq 的 timeout config；测试用 mock 避免依赖网络 |

---

## 6. 验证工具清单

每个 Slice 完成后必须跑：

```bash
# Rust
cargo test                                # 单元 + 集成
cargo clippy --all-targets -- -D warnings
cargo build --release                     # 单文件二进制
make release                              # 完整 release 构建

# 前端
cd web && npm run lint
cd web && npm run typecheck
cd web && npm test                        # 组件 + MSW
cd web && npm run build                   # 产出 dist/

# 手动
./target/release/cc-switch-tui &          # 后台启动
sleep 2 && curl http://127.0.0.1:$(cat ~/.cc-switch-tui/port)/api/health
kill %1                                   # 关掉
```

---

## 7. Resolved Questions（M1 范围决策）

| # | 问题 | 决策 | 落点 |
|---|---|---|---|
| Q1 | Dark mode | **M1 做**（S0-T6，shadcn/ui 默认支持几乎零成本） | S0 |
| Q2 | 搜索/过滤 instance | **M1 做**（S1-T5，客户端过滤无 API 改动） | S1 |
| Q3 | Import/Export config | **M1 做**（S7 整个 slice） | S7 |
| Q4 | Test Connection 按钮 | **M1 做**（S3-T5，新增 `/api/instances/:id/test`） | S3 |
| Q5 | macOS .app bundle | **M1 不做**（用户体验更好但非核心；先发裸二进制收集反馈） | M2+ |

---

## 8. 文档与提交

每个 Phase 结束做：
- 一次 git commit，message 含 `Phase X: <一句话>`
- Phase D 完成后：
  - 更新 `README.md`（改写为 Web 版说明 + 截图）
  - 更新 `CLAUDE.md`（项目状态 v0.4.0）
  - 打 tag `v0.4.0`

---

## 9. Plan 完成度自检

- [x] 10 个 vertical slices 定义完整
- [x] 每个 slice 有 touch points / 任务 / 验收 / 验证
- [x] 4 个 Phase checkpoint 明确
- [x] 依赖图绘制
- [x] 风险登记 9 条
- [x] 验证工具清单
- [x] 5 条 Open Questions 全部决策完成

**待你审阅**：plan 整体方向、slice 划分、phase 切点是否符合你的预期。如果有调整（比如把 S7 拆细 / 把 dark mode 移到 S8 抛光阶段 / 加新 slice），现在改最便宜。
