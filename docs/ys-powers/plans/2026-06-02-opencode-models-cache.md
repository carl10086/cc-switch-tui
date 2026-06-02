# Plan: OpenCode 模型列表缓存与后台刷新

> 日期：2026-06-02
> 关联 spec：`2026-06-02-opencode-models-cache-design.md`
> 分支：`feat/web-replaces-tui`（直接基于当前分支）

## 任务总览

```
T1 (后端 DAO + sqlite 表 + fetch_provider_models)
 └──→ T2 (/api/templates 合并 opencode_models + refresh endpoint + main.rs 启动刷新)
        ├─→ T3 (前端 OpencodeModelSelect 改造 + InstanceForm/DetailPage 传值)
        └─→ T4 (前端 SettingsPage 手动刷新按钮)
               ↓
             T5 (Chrome DevTools 端到端验证)
```

## 依赖分析

| 任务 | 依赖 | 说明 |
|---|---|---|
| T1 | 无 | 纯基础设施：DAO trait、sqlite 表、ureq 拉取逻辑 |
| T2 | T1 | 需要 DAO `get/set_opencode_models` 才能读 cache |
| T3 | T2 | 需要 `/api/templates` 返回 `opencodeModels` 字段 |
| T4 | T2 | 需要 `POST /api/opencode-models/refresh` endpoint |
| T5 | T3 + T4 | 端到端验证需要前后端都就绪 |

## T1 — 后端：DAO + sqlite 表 + 单 provider fetch

**Files**:
- `src/opencode_fetch.rs`（修改，新增 `fetch_provider_models(provider_id)` + `async` 改造）
- `src/dao/mod.rs`（修改，DAO trait 新增 2 个方法）
- `src/dao/sqlite_impl.rs`（修改，新增表 + 实现）

**Acceptance**:
- `fetch_provider_models("minimax-cn").await` 返回 `Vec<String>`（7 个 model id）
- `fetch_provider_models("kimi-for-coding").await` 返回 `Vec<String>`（3 个 model id）
- `fetch_provider_models("nonexistent").await` 返回空 vec（不报错）
- DAO `set_opencode_models` → `get_opencode_models` roundtrip 测试通过
- sqlite `opencode_model_cache` 表在 `:memory:` 中创建成功
- `cargo test` pass

**Verify**:
- `cargo test` — 全部 pass（含新增测试）
- `cargo fmt` — 无格式问题

**TDD**:
1. RED：写 `fetch_provider_models` 测试（mock HTTP 或 `#[ignore]` 网络测试）
2. RED：写 DAO cache roundtrip 测试
3. GREEN：实现 `fetch_provider_models` + DAO 方法 + sqlite 表
4. commit: `feat: add opencode model cache DAO + fetch_provider_models`

**注意**：
- `opencode_fetch.rs` 现有 `fetch_opencode_models()` 是**同步阻塞**的（ureq 没有 async）。需要判断：
  - 选项 A：把 `fetch_provider_models` 也做成同步（启动时后台线程跑）
  - 选项 B：引入 `reqwest` 或 `tokio-ureq` 做 async
  - **推荐 A**：保持同步，在 `tokio::spawn(blocking)` 里跑，不引入新依赖

## T2 — 后端：/api/templates 合并 opencode_models + refresh endpoint

**Files**:
- `src/api/templates.rs`（修改，`TemplateSummary` 加字段 + `list` handler 读 cache）
- `src/api/opencode_models.rs`（新建，refresh handler）
- `src/api/mod.rs`（修改，注册路由）
- `src/main.rs`（修改，启动时 spawn 后台刷新）

**Acceptance**:
- `GET /api/templates` 响应包含 `opencodeModels` 字段（每个 template 都有）
- cache 存在时，`opencodeModels` = cache 中的 model ids
- cache 不存在时，`opencodeModels` = 空数组（前端降级为 input）
- `POST /api/opencode-models/refresh` 成功更新 cache，返回 `{ updated, failed }`
- `main.rs` 启动时 `tokio::spawn` 后台刷新，不阻塞 server 启动
- `cargo test` pass

**Verify**:
- `cargo test` pass
- `curl http://127.0.0.1:7480/api/templates | jq '.[0].opencodeModels'` — 有数据
- `curl -X POST http://127.0.0.1:7480/api/opencode-models/refresh` — 返回 200

**commit**: `feat: merge opencode_models into /api/templates + add refresh endpoint`

## T3 — 前端：OpencodeModelSelect 改造

**Files**:
- `web/src/api/types.ts`（修改，`Template` 加 `opencodeModels: string[]`）
- `web/src/components/OpencodeModelSelect.tsx`（修改，props 改 `models: string[]`）
- `web/src/components/InstanceForm.tsx`（修改，传 `template.opencodeModels`）
- `web/src/routes/InstanceDetailPage.tsx`（修改，传 `template.opencodeModels`）

**Acceptance**:
- `Template` type 有 `opencodeModels` 字段
- `OpencodeModelSelect` 接收 `models: string[]`，非空时渲染 select，空时降级 input
- `InstanceForm` 和 `InstanceDetailPage` 都传 `currentTemplate?.opencodeModels ?? []`
- 现有测试 pass；typecheck 0 error
- **不**新增测试（`OpencodeModelSelect` 的行为和之前一致，只是数据来源变了）

**Verify**:
- `cd web && npx tsc --noEmit` 0 error
- `cd web && npx vitest run` 全部 pass

**commit**: `refactor(web): OpencodeModelSelect reads from template.opencodeModels`

## T4 — 前端：SettingsPage 手动刷新按钮

**Files**:
- `web/src/routes/SettingsPage.tsx`（修改，新增刷新按钮 + mutation hook）
- `web/src/routes/__tests__/SettingsPage.test.tsx`（新建或扩展，1 个测试）

**Acceptance**:
- Settings 页面有"Refresh OpenCode Models"按钮
- 点击按钮调用 `POST /api/opencode-models/refresh`
- 成功后 invalidate `['templates']` query，下拉框自动更新
- 按钮有 loading 态（`isPending` 时 disabled + "Refreshing…"）
- 测试：点击按钮 → fetch 被 `/api/opencode-models/refresh` 调用

**Verify**:
- `cd web && npx vitest run SettingsPage.test.tsx` pass
- `cd web && npx tsc --noEmit` 0 error

**commit**: `feat(web): add refresh OpenCode models button in Settings`

## T5 — Chrome DevTools 端到端验证

**Steps**:
1. `make web-build && cp web/dist/* web-dist/ && cargo run`
2. 等启动完成后，打开 `http://127.0.0.1:7480/instances/minimax-cl-mini`
3. `mcp__chrome-devtools__take_snapshot` — 验证 OpenCode Model ID 下拉有 7 个选项
4. `mcp__chrome-devtools__navigate_page` 到 Settings
5. 点击"Refresh OpenCode Models"按钮
6. 验证按钮变 loading → 恢复；network 面板看到 POST `/api/opencode-models/refresh`
7. 回到实例页，验证下拉框数据更新

**Acceptance**:
- OpenCode 下拉有 7 个 minimax 模型（不是之前的 2 个）
- Settings 刷新按钮工作正常
- 截图保存

**不产新 commit**，只在 plan 记录验证结果。

## 检查点

- **CP1** (T1+T2 后)：后端 `cargo test` pass；`curl /api/templates` 看到 `opencodeModels`
- **CP2** (T3+T4 后)：前端 `npx vitest run` 全部 pass；typecheck 0 error
- **CP3** (T5 后)：Chrome DevTools 确认下拉有 7 个模型 + Settings 刷新按钮工作

## 风险与缓解

| 风险 | 缓解 |
|---|---|
| ureq 是同步阻塞的，不能直接 `await` | `tokio::task::spawn_blocking` 包装同步 ureq 调用 |
| models.dev 网络不可达（国内） | 启动时刷新失败只打 log，不阻断；用户可用手动按钮重试；cache 存在时直接用 |
| sqlite 表迁移失败（旧 db 没有新表） | `CREATE TABLE IF NOT EXISTS`，不会失败 |
| `/api/templates` 响应变大 | 只增加 ~200 字节（2 个 provider × 平均 5 个 model id），可忽略 |
| OpencodeModelSelect props 变 `models: string[]`，和之前 `models: TemplateModel[]` 冲突 | TypeScript 编译器会报错，逐个文件修正调用点 |

## 验证

- [ ] T1-T4 全部 commit
- [ ] `cargo test` pass
- [ ] `cd web && npx tsc --noEmit` 0 error
- [ ] `cd web && npx vitest run` 全部 pass
- [ ] Chrome DevTools 端到端：OpenCode 下拉 7 个模型 + Settings 刷新按钮
- [ ] git log 显示 4 个独立 commit
