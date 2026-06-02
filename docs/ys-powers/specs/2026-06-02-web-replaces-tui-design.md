# Spec: Web 前端 + Rust 后端架构改写

## Objective

将 cc-switch-tui 从 ratatui TUI 改写为 **React Web 前端 + axum Rust 后端** 的前后端分离架构。Rust 进程以单文件二进制形态发布，启动后监听 `127.0.0.1` 随机端口并自动打开浏览器，用户通过 Web UI 完成所有 Provider Instance 管理。

**为什么这样做**：
- 页面编辑（表单、富文本、剪贴板、多行）比 TUI 友好太多
- 复用现有 Rust 业务层（DAO / domain / shell / opencode_config）零成本
- 单文件二进制 + 本地服务，保留"双击即用"的开箱体验

**关键心智模型（用户最初的核心疑问）**：
> 是否有权限 页面动本地文件呢？

**答：浏览器 JavaScript 永远不能直接访问本地文件。** 这是浏览器沙箱的强制约束。**唯一可行**的架构是：

```
React (浏览器)  ──HTTP/JSON──▶  Rust (axum)  ──▶  本地文件 / SQLite
    沙箱                         唯一出口
```

前端只发请求，所有写盘操作集中到后端。**安全模型天然成立**：绑死 loopback，不开 `0.0.0.0`，没有跨域、没有外部攻击面、不需要鉴权。

**用户故事**：
- 作为用户，我双击 / 命令行启动 `cc-switch-tui`，浏览器自动打开 localhost 上的管理界面
- 作为用户，我在 Web UI 用表单新建 / 编辑 / 删除 Provider Instance，表单比 TUI 友好（可粘贴、可隐藏、可校验）
- 作为用户，我可以在 Web 页面预览 `aliases.zsh` 和 OpenCode 配置内容，满意后点 "Apply" 写入
- 作为用户，我修改 `api_key` 时密码框有"显示"按钮，点错可以"重新输入"
- 作为用户，我关闭浏览器窗口后服务**仍在运行**（避免误关），通过 Ctrl-C 显式退出
- 作为用户，关闭服务后再启动端口能复用（写到 `~/.cc-switch-tui/port`）

**成功标准**：
- [ ] `cargo build --release` 产出单文件二进制，所有 React 资源嵌入
- [ ] 启动后 1 秒内浏览器自动打开到管理页面
- [ ] Instance CRUD（列表 / 详情 / 新建 / 编辑 / 复制 / 删除）功能完整
- [ ] aliases.zsh 内容实时预览 + Apply 写入
- [ ] OpenCode 配置内容实时预览 + Apply 写入
- [ ] Settings（默认 template、auto_open_browser）+ Diagnostics
- [ ] 表单字段级错误（alias 冲突等）有友好提示
- [ ] `api_key` 列表脱敏、详情明文、Password 输入框
- [ ] 旧 TUI 的全部功能在 Web 上有等价路径
- [ ] 后端只 bind `127.0.0.1`，不暴露给局域网
- [ ] 跨平台编译不依赖 npm（`web-dist/` 跟踪到 git）
- [ ] Rust API 集成测试 + React 组件测试 + MSW mock 测试全部通过

## Tech Stack

**后端（Rust）**：
- `axum` 0.7+ — Web 框架（tower 生态，tokio 友好）
- `tokio` 1.x — 异步运行时
- `tower` / `tower-http` — middleware（CORS / tracing）
- `serde` / `serde_json` — 序列化
- `webbrowser` — 跨平台打开默认浏览器
- 现有依赖不动：`rusqlite` / `tracing` / `chrono` / `ureq` / `dirs` / `thiserror`

**前端（TypeScript）**：
- `vite` 5.x — 构建工具
- `react` 18.x + `react-dom` 18.x — UI 框架
- `typescript` 5.x — 类型
- `react-router-dom` 6.x — 路由
- `@tanstack/react-query` 5.x — 数据层（fetch / cache / mutation）
- `tailwindcss` 3.x — 样式
- `shadcn/ui` — 组件（基于 Radix Primitives，可复制可改）
- `zod` — 前端表单 schema 校验
- `vitest` + `@testing-library/react` — 组件测试
- `msw` — HTTP mock（前端 API 层单测）

**构建 / 嵌入**：
- Rust `include_dir!` — 把 `web-dist/` 嵌入二进制
- Vite build 输出到 `web-dist/`（git 跟踪，跨机器编译友好）

**明确删除的依赖**：
- `ratatui` — TUI 渲染
- `crossterm` — 终端控制
- `tools/migrate_instances_id.rs` — 旧迁移工具（v0.3.0 后已无用）

## Commands

```bash
# --- 开发 ---

# 一键开发：Vite dev server (5173) + cargo run (7480, 带 proxy)
make dev

# 仅启动 Rust（前端用已编译的 web-dist/）
make dev-rust-only
# 等价于: cargo run

# 单独跑 Vite（一般用不到，dev 模式已包含）
cd web && npm run dev

# --- 构建 ---

# 重新构建前端（web/ 改动后必跑）
make web-build
# 等价于: cd web && npm ci && npm run build && rm -rf web-dist && cp -r web/dist web-dist

# 构建 Rust release（单文件二进制）
make build
# 等价于: cargo build --release

# 全量发布构建（前端 + 后端）
make release
# 等价于: make web-build && cargo build --release

# --- 测试 ---

# Rust 全部测试
cargo test

# Rust 特定模块
cargo test api
cargo test dao

# 前端测试
cd web && npm test
# 等价于: cd web && npm run test

# 前端测试 + 覆盖率
cd web && npm run test:coverage

# --- 质量 ---

# Rust 格式化 + clippy
cargo fmt
cargo clippy --all-targets -- -D warnings

# 前端 lint + typecheck
cd web && npm run lint
cd web && npm run typecheck
```

## Project Structure

```
cc-switch-tui/
├── Cargo.toml
├── Makefile                          # 一键 build / dev / test
├── package.json                      # 前端依赖（workdir: web/）
├── package-lock.json
├── tsconfig.json                     # 根 tsconfig（paths 配置）
├── vite.config.ts                    # Vite 配置（带 /api → :7480 proxy）
├── tailwind.config.ts
├── postcss.config.js
├── index.html                        # Vite 入口
├── components.json                   # shadcn/ui 配置
│
├── web/                              # React 源码
│   ├── src/
│   │   ├── main.tsx                  # ReactDOM 挂载 + Provider 嵌套
│   │   ├── App.tsx                   # 根布局：左侧导航 + <Outlet />
│   │   ├── routes/
│   │   │   ├── InstancesPage.tsx     # 列表 + 行内操作
│   │   │   ├── InstanceDetailPage.tsx# /instances/:id 详情/编辑
│   │   │   ├── AliasesPage.tsx       # aliases.zsh 预览 + Apply
│   │   │   ├── OpencodePage.tsx      # OpenCode 配置预览 + Apply
│   │   │   └── SettingsPage.tsx      # 设置 + 诊断信息
│   │   ├── components/
│   │   │   ├── ui/                   # shadcn/ui 生成的组件
│   │   │   ├── InstanceForm.tsx      # 复用于新建/编辑
│   │   │   ├── EnvOverridesEditor.tsx# 键值对编辑器
│   │   │   ├── ConfirmDialog.tsx     # 危险操作确认
│   │   │   ├── ApiErrorBanner.tsx    # 顶层错误条幅
│   │   │   └── SecretInput.tsx       # api_key 输入框（带显示/复制）
│   │   ├── api/
│   │   │   ├── client.ts             # fetch 封装 + 错误统一处理
│   │   │   ├── hooks.ts              # TanStack Query hooks
│   │   │   └── types.ts              # 与 Rust 对齐的 TS 类型
│   │   ├── lib/
│   │   │   ├── format.ts             # 脱敏、长度截断
│   │   │   └── validate.ts           # Zod schema（前端校验）
│   │   └── styles/
│   │       └── globals.css           # Tailwind base + 主题变量
│   └── dist/                         # Vite 产物（git ignore）
│
├── web-dist/                         # 编译后的 React 资源（git 跟踪）
│   ├── index.html
│   └── assets/
│       ├── index-[hash].js
│       └── index-[hash].css
│
├── docs/
│   └── ys-powers/
│       └── specs/
│           ├── 2026-06-01-edit-instance-model-design.md
│           └── 2026-06-02-web-replaces-tui-design.md  # ← 本文件
│
└── src/                              # Rust 源码
    ├── main.rs                       # 入口：找端口 → 启 axum → open browser
    ├── lib.rs                        # 模块声明
    │
    ├── api/                          # 新增：axum 路由层
    │   ├── mod.rs                    # Router 装配 + middleware
    │   ├── state.rs                  # AppState（Arc<Mutex<Dao>> 等）
    │   ├── error.rs                  # ApiError → IntoResponse + JSON 形态
    │   ├── templates.rs              # GET /api/templates
    │   ├── instances.rs              # CRUD 端点
    │   ├── aliases.rs                # GET /api/aliases, POST /api/aliases/apply
    │   ├── opencode.rs               # GET /api/opencode-config/:id, /apply
    │   ├── settings.rs               # GET/PUT /api/settings
    │   └── health.rs                 # GET /api/health, /api/diagnostics
    │
    ├── dao/                          # 现有：不动
    │   ├── mod.rs
    │   ├── memory_impl.rs
    │   └── sqlite_impl.rs
    │
    ├── domain/                       # 现有：不动
    │   ├── mod.rs
    │   ├── error.rs
    │   ├── instance.rs
    │   ├── settings.rs
    │   └── template.rs
    │
    ├── shell.rs                      # 现有：生成 aliases.zsh 内容
    ├── opencode_config.rs            # 现有：生成 OpenCode 配置
    ├── opencode_fetch.rs             # 现有：拉取远端模型
    ├── event.rs                      # 现有：TUI 事件枚举（删除或保留为兼容层）
    │
    └── bin/                          # 删除 migrate_instances_id（已无用）
```

## Code Style

### Rust（保持现有风格，参考 `src/dao/mod.rs`）

```rust
// API handler 示例：返回统一错误、async fn、Routed 风格
pub async fn update_instance(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(patch): Json<InstancePatch>,
) -> Result<Json<Instance>, ApiError> {
    let mut dao = state.dao.lock().await;
    let updated = dao.update_instance(&id, patch)?;
    Ok(Json(updated))
}
```

- 错误处理：`Result<T, ApiError>`，所有 handler 不 panic
- 错误响应：实现 `IntoResponse`，统一 JSON 格式
- 异步：所有 handler 是 `async fn`
- 序列化：`#[derive(Serialize, Deserialize)]` + `#[serde(rename_all = "camelCase")]`
- 字段命名：Rust 用 `snake_case`，自动转 `camelCase` 给前端
- 文档：模块顶部一行 `///` 用途说明

### TypeScript

```typescript
// API hook 示例：TanStack Query
export function useInstances() {
  return useQuery({
    queryKey: ['instances'],
    queryFn: () => api.get<Instance[]>('/instances'),
  });
}

// 类型定义：与 Rust serde camelCase 对齐
export interface Instance {
  id: string;
  templateId: string;
  alias: string;
  apiKey?: string;     // 仅详情返回
  baseUrl?: string;
  modelId: string;
  envOverrides: Record<string, string>;
  kvCacheEnabled: boolean;
  isDefault: boolean;
}
```

- 函数式组件 + hooks
- 命名：组件 `PascalCase`、hooks `useXxx`、工具函数 `camelCase`
- 不用 `any`，unknown + 窄化
- 错误优先用 `Error` 子类（`ApiError`），不抛裸 Error
- 优先用 shadcn/ui 而不是自造组件
- 严格 TS（`strict: true`）

## Core Design

### 1. 进程模型

```
用户执行: cc-switch-tui
  ↓
1. 读 ~/.cc-switch-tui/port，尝试该端口（如果上次进程已死）
  ↓
2. 启动失败则 127.0.0.1:7480 起，被占就 +1 直到找到
  ↓
3. 把端口写回 ~/.cc-switch-tui/port
  ↓
4. tokio::spawn 后台跑 axum server
  ↓
5. webbrowser::open("http://127.0.0.1:{port}")
  ↓
6. 主线程阻塞等待 Ctrl-C
  ↓
7. SIGINT → graceful shutdown（关闭 server、清 port 文件）
```

**关键决策**：
- 不开固定端口（用户可能同时跑多个实例）
- 端口复用 `~/.cc-switch-tui/port` 缓存
- 浏览器关闭 ≠ server 关闭（避免误关丢失状态）
- 显式 Ctrl-C 退出

### 2. 构建管线

```makefile
# Makefile
.PHONY: dev dev-rust-only web-build build release test

dev:
	cd web && npm run dev &  # Vite dev server on 5173
	cargo run                 # Rust on 7480, with proxy from Vite

dev-rust-only:
	cargo run                 # 跑生产模式（用 web-dist/）

web-build:
	cd web && npm ci && npm run build
	rm -rf web-dist
	cp -r web/dist web-dist

build:
	cargo build --release

release: web-build
	cargo build --release
```

**`include_dir!` 集成**（`src/main.rs`）：

```rust
use include_dir::{include_dir, Dir};

static WEB_DIST: Dir = include_dir!("web-dist");

#[tokio::main]
async fn main() {
    let port = pick_port();
    let app = Router::new()
        .nest("/api", api::router())
        .fallback(spa_fallback);  // 非 /api/* 返回 web-dist/index.html

    let listener = tokio::net::TcpListener::bind(
        format!("127.0.0.1:{}", port)
    ).await.unwrap();
    
    webbrowser::open(&format!("http://127.0.0.1:{}", port)).ok();
    
    axum::serve(listener, app).await.unwrap();
}

async fn spa_fallback(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    
    // 先尝试精确文件
    if let Some(file) = WEB_DIST.get_file(path) {
        return Response::from_bytes(file.contents().to_vec())
            .with_header(...);
    }
    
    // 否则返回 index.html（SPA 路由）
    let index = WEB_DIST.get_file("index.html").unwrap();
    Response::from_bytes(index.contents().to_vec())
        .with_header("content-type", "text/html")
}
```

### 3. 完整 API 契约

```
─────────────────────────────────────────────────────────────
Templates（只读）
─────────────────────────────────────────────────────────────
GET    /api/templates
       → 200 [{ id, displayName, defaultBaseUrl, defaultModel,
                availableModels: [...] }]

─────────────────────────────────────────────────────────────
Instance 资源
─────────────────────────────────────────────────────────────
GET    /api/instances
       → 200 [{ id, templateId, alias, baseUrl, modelId,
                envOverrides, kvCacheEnabled, isDefault }]
       注: apiKey 不在列表中返回

GET    /api/instances/:id
       → 200 { id, templateId, alias, apiKey, baseUrl, modelId,
                envOverrides, kvCacheEnabled, isDefault }

POST   /api/instances
       body: { templateId, alias, apiKey, baseUrl?, modelId?,
               envOverrides?, kvCacheEnabled? }
       → 201 + 完整 instance
       → 409 ALIAS_CONFLICT
       → 400 VALIDATION_ERROR (带 field)

PATCH  /api/instances/:id
       body: 部分字段
       → 200 + 完整 instance
       → 409 ALIAS_CONFLICT

DELETE /api/instances/:id
       → 204
       → 400 if 试图删默认且没其他默认

POST   /api/instances/:id/duplicate
       → 201 + 新 instance（alias 加 -copy 后缀）
       → 409 if 后缀冲突

─────────────────────────────────────────────────────────────
Aliases 文件
─────────────────────────────────────────────────────────────
GET    /api/aliases
       → 200 text/plain (生成的 aliases.zsh 完整内容)
       注: 派生内容，每次调用实时生成

POST   /api/aliases/apply
       → 200 { "path": "~/.cc-switch-tui/aliases.zsh" }
       注: 写入文件；幂等

─────────────────────────────────────────────────────────────
OpenCode 配置
─────────────────────────────────────────────────────────────
GET    /api/opencode-config/:instance_id
       → 200 application/json (生成的配置)
       注: 派生内容

POST   /api/opencode-config/:instance_id/apply
       → 200 { "path": "..." }

─────────────────────────────────────────────────────────────
Settings
─────────────────────────────────────────────────────────────
GET    /api/settings
       → 200 { defaultTemplate, autoOpenBrowser, ... }

PUT    /api/settings
       body: 部分字段
       → 200 + 完整 settings

─────────────────────────────────────────────────────────────
健康 / 诊断
─────────────────────────────────────────────────────────────
GET    /api/health
       → 200 { status: "ok", version: "0.4.0", dbPath: "..." }

GET    /api/diagnostics
       → 200 { dbWritable, zshrcWritable, opencodeConfigPath, ... }
```

### 4. 错误模型

所有错误统一返回：

```json
{
  "error": {
    "code": "ALIAS_CONFLICT",
    "message": "Alias 'mini-prod' already exists",
    "field": "alias"
  }
}
```

**错误码枚举**：
- `VALIDATION_ERROR` (400) — 字段校验失败，带 `field`
- `NOT_FOUND` (404) — 资源不存在
- `ALIAS_CONFLICT` (409) — alias 重复，带 `field: "alias"`
- `CANNOT_DELETE_DEFAULT` (400) — 删默认且没替补
- `INTERNAL_ERROR` (500) — 内部错误，带 `traceId`
- `IO_ERROR` (500) — 文件读写失败，带 `path`

### 5. 敏感数据流

- `api_key` 默认仅在 `GET /api/instances/:id` 详情返回
- 列表里 api_key 字段**省略**（前端显示 `••••••••`）
- 修改 api_key：`PATCH { apiKey: "sk-..." }` → 返回的详情仍带完整 key
- **不**提供"导出全部 api_key"或"查看全部 key"的批量端点
- 不在日志、错误信息、URL 中泄漏 api_key

### 6. 状态同步

- **纯请求/响应**：无 SSE、无 WebSocket、无轮询
- 初始进入页面时 fetch；mutation 后乐观更新或 refetch
- TanStack Query 配置：`staleTime: 0, retry: 1, refetchOnWindowFocus: false`

### 7. 前端关键页面

| 页面 | 路径 | 主要交互 |
|---|---|---|
| Instances 列表 | `/` | 表格 + 新建按钮 + 行内操作 |
| Instance 详情/编辑 | `/instances/:id` | 表单 + 离开拦截 |
| Aliases 预览 | `/aliases` | 等宽 textarea + Apply 按钮 |
| OpenCode 预览 | `/opencode` | 左侧列表 + 右侧 JSON 高亮 + Apply |
| Settings | `/settings` | 全局设置 + 诊断面板 |

**实例表单字段**：
- alias（必填，校验规则同 TUI：小写字母/数字/`-`/`_`）
- template（下拉）
- isDefault（开关）
- kvCacheEnabled（开关）
- baseUrl（可选，默认用 template 的）
- modelId（从 template.availableModels 选 + 自定义）
- apiKey（Password + 显示 + 复制）
- envOverrides（键值对编辑器，+ 添加 / - 删除）

## Testing Strategy

### Rust 后端

| 层 | 测试类型 | 工具 | 文件 |
|---|---|---|---|
| domain | 单元 | 标准 | `src/domain/*_test.rs`（现有） |
| dao | 单元（内存 + SQLite） | tempfile | `src/dao/*_test.rs`（现有） |
| **api** | **集成** | `axum::Router::oneshot` | **`tests/api/*.rs`（新增）** |

**API 集成测试结构**：
```
tests/
├── api/
│   ├── mod.rs                 # 公共 helper: spawn_test_app
│   ├── instances_test.rs      # CRUD + alias 冲突
│   ├── templates_test.rs      # 列表
│   ├── aliases_test.rs        # 生成 + apply
│   ├── opencode_test.rs       # 生成 + apply
│   ├── settings_test.rs       # GET + PUT
│   ├── error_test.rs          # 错误响应格式
│   └── health_test.rs         # /health
```

**关键测试用例**：
- `test_create_instance_returns_201`
- `test_create_instance_duplicate_alias_returns_409`
- `test_create_instance_invalid_alias_returns_400`
- `test_list_instances_omits_api_key`
- `test_get_instance_includes_api_key`
- `test_patch_instance_partial`
- `test_delete_default_without_replacement_returns_400`
- `test_aliases_apply_writes_file`
- `test_opencode_config_derived_from_instance`
- `test_health_returns_version`

### React 前端

| 层 | 测试类型 | 工具 | 文件 |
|---|---|---|---|
| 组件 | 组件 | Vitest + Testing Library | `web/src/**/*.test.tsx` |
| API 层 | 单元（mock） | MSW | `web/src/api/*.test.ts` |
| 表单 | 单元（schema） | Zod | `web/src/lib/validate.test.ts` |

**关键测试用例**：
- `InstanceForm` 显示所有字段 / 提交时调 mutation / 字段错误显示
- `useInstances` 拉取后 cache / 错误时抛出
- `validateAlias` 大写/空白被拒 / 合规通过
- `SecretInput` 输入遮蔽 / 显示切换
- `ApiErrorBanner` 渲染 code + message

### 端到端（**M2+ 再做**）

- Playwright 启动 Rust 二进制 → 打开浏览器 → 交互 → 截图对比
- M1 不做，手工验证

## Boundaries

### Always
- 启动时只 bind `127.0.0.1`，**绝不** bind `0.0.0.0`
- 写代码前先写失败测试（TDD）
- API 错误用统一 `ApiError` 类型，handler 不 panic
- `api_key` 不写日志、不进错误消息、不进 URL
- 前端表单提交前 Zod 校验，后端 handler 二次校验
- 所有文件 I/O（zshrc、OpenCode 配置）走 Rust 端
- 改动 `src/domain/` 后同步更新 `web/src/api/types.ts`（CI 加一致性检查）
- 写新依赖前先看是否能用现有依赖
- 涉及删除/重写现有代码前先确认（TUI 代码可一次性删除，但要 review）

### Ask first
- 修改 `ProviderInstance` / `ProviderTemplate` / `Settings` 字段
- 新增 Rust crate 依赖
- 新增 npm 依赖（特别是运行时依赖，不是 devDependency）
- 改端口策略
- 改错误码 / 错误消息风格
- 改 TUI 用户的迁移路径（不保留 TUI 后用户的旧 config 怎么办 — 此处已决策：旧 SQLite 直接复用，业务层零迁移）

### Never
- 暴露服务到 `0.0.0.0` 或 LAN
- 在前端代码里 hardcode `http://localhost:7480` 之类的端口（用相对路径 `/api`）
- 把 `api_key` 写进前端 store / localStorage / cookie
- 用 CORS 解决"前后端分离"问题（loopback 不需要 CORS）
- 删除 TUI 时连带删除 `dao/` `domain/` `shell.rs` `opencode_config.rs`（业务核心必须保留）
- 删 `web-dist/` 跟踪（破坏跨机器编译）
- 在 `serde(rename_all = "camelCase")` 上偷懒改 snake_case（破坏 TS 对齐）
- 把 Vite dev server 端口 (5173) 暴露给用户（用户只该看到 7480）
- 在 `Cargo.lock` 已锁的情况下擅自 `cargo update` 主要依赖

## Implementation Plan

### Phase 1: 仓库骨架与构建链（基础）

1. **创建分支**（已完成）`feat/web-replaces-tui`
2. **初始化前端项目**：
   - `web/` 目录 + `package.json` + `tsconfig.json` + `vite.config.ts`
   - `tailwind.config.ts` + `postcss.config.js` + `index.html`
   - `components.json`（shadcn/ui）
3. **配置 Makefile** + 所有 `make` target
4. **配置 `include_dir!`**：`Cargo.toml` 加 `include_dir = "0.7"`
5. **写最小 `index.html`**（Vite 占位）+ `web-build` 流程跑通
6. **验证**：`make release` 产出单二进制；启动后浏览器能打开一个空白页

**验收**：单文件二进制启动后，浏览器看到一个 "Hello" 页面，地址栏是 `127.0.0.1:随机端口`

### Phase 2: 后端 API 骨架

1. **新建 `src/api/` 模块**，定义 `ApiError` + 统一响应
2. **实现 `GET /api/health`** + `GET /api/diagnostics`
3. **配置 axum Router** + SPA fallback
4. **写 API 集成测试** 的 helper（spawn_test_app）
5. **验证**：`cargo test` 通过 health/diagnostics 测试

**验收**：浏览器能调通 `/api/health` 看到 JSON 响应

### Phase 3: 业务 API 端点（按 CRUD 顺序）

1. **`/api/templates`** — 调 `domain::templates`
2. **`/api/instances` GET/POST** — 复用 DAO
3. **`/api/instances/:id` GET/PATCH/DELETE** — 复用 DAO
4. **`/api/instances/:id/duplicate`** — 复用 DAO
5. **`/api/aliases` GET + `/api/aliases/apply` POST** — 复用 shell.rs
6. **`/api/opencode-config/:id` GET + /apply POST** — 复用 opencode_config.rs
7. **`/api/settings` GET + PUT** — 新增 domain::settings
8. **每个端点同步写集成测试**

**验收**：所有 API 集成测试通过；Postman / curl 调通完整 CRUD

### Phase 4: 前端骨架

1. **配置 main.tsx** + QueryClientProvider + BrowserRouter
2. **App.tsx 布局** + 左侧导航
3. **API client** + TanStack Query hooks
4. **shadcn/ui 初始化**（加 Button、Input、Dialog、Table、Switch、Card）
5. **类型定义** `web/src/api/types.ts`（与 Rust 对齐）
6. **Zod schema** `web/src/lib/validate.ts`

**验收**：浏览器能加载空壳页面，导航点击有效，能调通 health

### Phase 5: 页面实现（按页面顺序）

1. **Instances 列表页**（读）
2. **Instance 详情/编辑页**（CRUD）
3. **新建/编辑 Dialog**（用 `InstanceForm`）
4. **Aliases 预览页**（textarea + Apply）
5. **OpenCode 预览页**（JSON 高亮 + Apply）
6. **Settings 页**（设置 + 诊断）
7. **每个组件同步写组件测试**

**验收**：所有页面功能完整，操作闭环

### Phase 6: 删除 TUI

1. **删除 `src/ui/`** 全部
2. **删除 `src/app/`** 全部
3. **删除 `src/event.rs`**（如果只有 TUI 用）
4. **`Cargo.toml` 移除** `ratatui` / `crossterm`
5. **删除 `tools/migrate_instances_id.rs`**（v0.3.0 后已无用）
6. **删除 `src/bin/`** 目录（如果已空）
7. **`main.rs` 替换为新入口**
8. **`cargo build` 验证零警告**

**验收**：`cargo build --release` 通过，单文件二进制启动正常

### Phase 7: 端到端 + 抛光

1. **手动跑全流程**：新建 → 编辑 → 复制 → 删除 → Apply aliases → 重启 shell
2. **错误路径测试**：alias 冲突 / api_key 错误 / 磁盘只读
3. **macOS quarantine** 提示更新
4. **README + 使用文档** 改写
5. **CLAUDE.md** 更新（项目状态从 TUI 改为 Web）

**验收**：v0.4.0 发布可用

## Task Breakdown

- [ ] **Task 1**: 初始化前端项目骨架（package.json / vite / tailwind / tsconfig / index.html）
  - Acceptance: `cd web && npm run dev` 能启动 Vite；`npm run build` 产出 dist/
  - Verify: 浏览器访问 5173 看到 "Hello" 页面
  - Files: `web/package.json`, `web/vite.config.ts`, `web/tsconfig.json`, `web/tailwind.config.ts`, `web/postcss.config.js`, `web/index.html`, `web/src/main.tsx`

- [ ] **Task 2**: 配置 Makefile + 全部 target
  - Acceptance: `make dev` / `make web-build` / `make build` / `make release` / `make test` 全部跑通
  - Verify: 跑一遍所有 target 无报错
  - Files: `Makefile`

- [ ] **Task 3**: Rust 端集成 `include_dir!` + SPA fallback
  - Acceptance: `cargo build` 嵌入 `web-dist/`，启动后浏览器能看到嵌入的 index.html
  - Verify: 跑 `make release` 后启动二进制，浏览器打开后看到 "Hello"
  - Files: `Cargo.toml`, `src/main.rs`, `web-dist/index.html`（占位）

- [ ] **Task 4**: 实现 `src/api/` 模块骨架（ApiError + Router 装配 + health）
  - Acceptance: `GET /api/health` 返回 JSON 200，错误响应格式统一
  - Verify: `cargo test api::health` 通过 + 手动 curl 验证
  - Files: `src/api/mod.rs`, `src/api/error.rs`, `src/api/health.rs`, `src/api/state.rs`, `tests/api/mod.rs`, `tests/api/health_test.rs`

- [ ] **Task 5**: 实现 `/api/templates` 端点
  - Acceptance: 返回 `domain::templates` 列表，camelCase 序列化
  - Verify: `cargo test api::templates` 通过
  - Files: `src/api/templates.rs`, `tests/api/templates_test.rs`

- [ ] **Task 6**: 实现 `/api/instances` CRUD（GET 列表 / GET 详情 / POST / PATCH / DELETE / duplicate）
  - Acceptance: 全部端点工作，列表脱敏 api_key，alias 冲突 409
  - Verify: `cargo test api::instances` 全部通过（至少 8 个测试用例）
  - Files: `src/api/instances.rs`, `tests/api/instances_test.rs`

- [ ] **Task 7**: 实现 `/api/aliases` 端点（GET 内容 + POST apply）
  - Acceptance: GET 返回 shell.rs 生成的文本，apply 写入 `~/.cc-switch-tui/aliases.zsh`
  - Verify: 集成测试 + 手动 Apply 后 cat 文件验证
  - Files: `src/api/aliases.rs`, `tests/api/aliases_test.rs`

- [ ] **Task 8**: 实现 `/api/opencode-config/:id` 端点
  - Acceptance: GET 返回 opencode_config.rs 生成的 JSON，apply 写入 OpenCode 配置
  - Verify: 集成测试 + 手动 Apply 后 cat 配置验证
  - Files: `src/api/opencode.rs`, `tests/api/opencode_test.rs`

- [ ] **Task 9**: 实现 `/api/settings` 端点
  - Acceptance: GET / PUT 正常工作
  - Verify: 集成测试
  - Files: `src/api/settings.rs`, `src/domain/settings.rs`（如果之前没有）, `tests/api/settings_test.rs`

- [ ] **Task 10**: 实现 `/api/diagnostics` 端点
  - Acceptance: 返回 DB 路径、zshrc 路径、写入权限等
  - Verify: 集成测试
  - Files: `src/api/health.rs`（扩展）或 `src/api/diagnostics.rs`, `tests/api/diagnostics_test.rs`

- [ ] **Task 11**: 进程模型实现（找端口 + 写 port 文件 + open 浏览器 + Ctrl-C 处理）
  - Acceptance: 启动后浏览器自动开，端口写文件，二次启动复用端口
  - Verify: 手动跑两次看效果
  - Files: `src/main.rs`（重写）, `src/port.rs`（新增）

- [ ] **Task 12**: 前端 API client + TanStack Query hooks
  - Acceptance: `useInstances` / `useInstance` / `useTemplates` / 等 hooks 全部就绪，错误统一处理
  - Verify: 组件测试 + MSW mock 测试
  - Files: `web/src/api/client.ts`, `web/src/api/hooks.ts`, `web/src/api/types.ts`, `web/src/api/*.test.ts`

- [ ] **Task 13**: 前端 Zod schema + 校验工具
  - Acceptance: alias 校验、必填校验、长度校验全部 Zod 化
  - Verify: 单元测试覆盖正反例
  - Files: `web/src/lib/validate.ts`, `web/src/lib/validate.test.ts`

- [ ] **Task 14**: 初始化 shadcn/ui + 基础组件
  - Acceptance: Button / Input / Dialog / Table / Switch / Card / Textarea / Select 全部可用
  - Verify: Storybook 或示例页面
  - Files: `web/src/components/ui/*`（shadcn 生成）

- [ ] **Task 15**: App 布局 + 路由 + 导航
  - Acceptance: 左侧导航 + 右侧 Outlet，5 个页面占位可点
  - Verify: 浏览器跳转正常
  - Files: `web/src/main.tsx`, `web/src/App.tsx`, `web/src/routes/*.tsx`

- [ ] **Task 16**: Instances 列表页实现
  - Acceptance: 表格渲染、isDefault 标记、kvCache 标记、行操作按钮
  - Verify: 组件测试 + 手动验证
  - Files: `web/src/routes/InstancesPage.tsx`, `web/src/components/InstancesTable.tsx`, `web/src/routes/InstancesPage.test.tsx`

- [ ] **Task 17**: Instance 详情/编辑页 + InstanceForm
  - Acceptance: 全部字段可编辑、离开拦截、错误显示、SecretInput
  - Verify: 组件测试 + 手动
  - Files: `web/src/routes/InstanceDetailPage.tsx`, `web/src/components/InstanceForm.tsx`, `web/src/components/SecretInput.tsx`, `web/src/components/EnvOverridesEditor.tsx`

- [ ] **Task 18**: 新建/编辑 Dialog
  - Acceptance: 弹出对话框、字段校验、提交后 refetch
  - Verify: 组件测试
  - Files: `web/src/components/InstanceFormDialog.tsx`, `web/src/components/ConfirmDialog.tsx`

- [ ] **Task 19**: Aliases 预览页
  - Acceptance: 等宽 textarea、Apply 按钮、状态指示（已应用 / 未应用 / mtime 差异）
  - Verify: 组件测试 + 手动 Apply 验证
  - Files: `web/src/routes/AliasesPage.tsx`, `web/src/routes/AliasesPage.test.tsx`

- [ ] **Task 20**: OpenCode 预览页
  - Acceptance: 左侧列表 + 右侧 JSON 高亮、Apply 按钮
  - Verify: 组件测试 + 手动 Apply 验证
  - Files: `web/src/routes/OpencodePage.tsx`, `web/src/routes/OpencodePage.test.tsx`

- [ ] **Task 21**: Settings 页 + 诊断面板
  - Acceptance: 全局设置表单 + 诊断信息展示
  - Verify: 组件测试
  - Files: `web/src/routes/SettingsPage.tsx`, `web/src/routes/SettingsPage.test.tsx`

- [ ] **Task 22**: ApiErrorBanner + 全局错误处理
  - Acceptance: 顶层错误条幅、字段级错误定位
  - Verify: 组件测试
  - Files: `web/src/components/ApiErrorBanner.tsx`, `web/src/api/client.ts`（扩展）

- [ ] **Task 23**: 删除 TUI 全部代码
  - Acceptance: `src/ui/` `src/app/` `src/event.rs` 全部删除，`Cargo.toml` 移除 ratatui/crossterm
  - Verify: `cargo build` 通过 + 全部测试通过
  - Files: `src/ui/*` (删), `src/app/*` (删), `src/event.rs` (删), `Cargo.toml`, `src/main.rs`

- [ ] **Task 24**: 全量验证（端到端 + 质量门）
  - Acceptance: `cargo test` + `cargo clippy` + `cd web && npm test` + `cargo build --release` 全过
  - Verify: 跑一遍所有命令
  - Files: 无

- [ ] **Task 25**: 文档更新
  - Acceptance: README.md 改写为 Web 版说明，CLAUDE.md 项目状态更新
  - Verify: 阅读检查
  - Files: `README.md`, `CLAUDE.md`

## Resolved Decisions

以下问题在 Phase 1（explore-then-ask）已与用户确认，记录在此供回溯：

1. **访问模型**：本地起服务 + 自动开浏览器。Rust 绑死 `127.0.0.1:随机端口`，不开 `0.0.0.0`，不需要鉴权。
2. **取代关系**：Web 完全取代 TUI。删除 `ratatui` / `crossterm` 依赖和 `src/ui/` `src/app/` 目录。不保留 TUI 入口。
3. **发布形态**：单文件二进制，React 编译产物走 `include_dir!` 嵌入。`web-dist/` 跟踪到 git（跨机器编译不需要 npm）。
4. **MVP 范围**：全量 1:1 复刻 TUI 功能（Instance CRUD、aliases、OpenCode、Settings、诊断），不做功能裁剪。
5. **前端栈**：Tailwind + shadcn/ui + Vite + React 18 + TypeScript + React Router + TanStack Query。
6. **后端栈**：axum + tokio（不复用 TUI 的 ratatui 状态机）。
7. **API 风格**：标准 REST 资源（方案 A），非 RPC。`PATCH` 处理"设默认"等部分更新。
8. **状态同步**：纯请求/响应，无 SSE/WebSocket/轮询。Web UI 是配置的唯一来源。
9. **数据模型**：完全复用 `src/domain/` 现有结构，零业务迁移。SQLite 文件格式不变。
10. **敏感数据**：`api_key` 仅在 `GET /api/instances/:id` 详情返回，列表脱敏。不提供批量导出。
11. **错误模型**：统一 `{error: {code, message, field?}}` JSON 格式，HTTP 状态码语义化。
12. **CORS**：不需要（loopback 同源），开发模式 Vite proxy 解决。
13. **数据库**：继续用 SQLite（`rusqlite`），文件位置 `~/.cc-switch-tui/db.sqlite` 不变。
14. **shell alias 机制**：保持现状（`~/.cc-switch-tui/aliases.zsh` + `source ~/.zshrc`），Web 只是编辑入口。
15. **OpenCode 配置机制**：保持现状（写 OpenCode 配置文件），Web 只是预览 + Apply 入口。
16. **端口策略**：启动时尝试 `~/.cc-switch-tui/port` 缓存的端口，失败再 +1 扫描找到空闲端口，写回缓存。
17. **进程退出**：浏览器关闭 ≠ 服务关闭，Ctrl-C 显式退出（避免误关）。
18. **TUI 用户数据兼容**：v0.3.x 用户升级到 v0.4.0 后 SQLite 文件直接复用，业务层零迁移。配置文件位置不变。
19. **CI 策略（M1 暂不做）**：M2+ 加 Rust clippy + 前端 lint + 字段一致性检查（Rust domain vs TS types）。
20. **端到端测试（M1 暂不做）**：M2+ 加 Playwright，启动二进制 + 浏览器交互。
