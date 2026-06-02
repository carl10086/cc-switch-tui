# Spec: OpenCode 模型列表缓存与后台刷新（简化版）

> 日期：2026-06-02
> 关联 spec：`2026-06-02-model-dropdown-and-templates-prefetch-design.md`
> 分支：`feat/web-replaces-tui`

## 1. Objective（目标）

修复 Web 版本 OpenCode Model ID 下拉框数据来源错误的问题。

### 当前问题

Web 版本 `OpencodeModelSelect` 使用 `template.models[].opencodeModelId`（Rust 硬编码），而 TUI 版本使用 `models.dev/api.json` 外部 API + 本地缓存合并。

结果：Web 下拉框只显示 2 个 minimax 模型（M3、M2.7-highspeed），但外部 API 实际有 7 个（M2、M2.1、M2.5、M2.5-highspeed、M2.7、M2.7-highspeed、M3）。新模型发布时必须改 Rust 代码重新编译。

### 正确行为

- OpenCode Model ID 下拉框数据来自 `models.dev/api.json` 外部 API
- 只拉取**关注的 provider**（当前 minimax-cn、kimi-for-coding）
- 启动时先读本地 sqlite cache 立刻渲染，**0 延迟**
- 后台异步刷新，成功则静默更新 cache
- 设置页提供**手动刷新按钮**兜底

### 非目标

- 不缓存全部 138 个 provider（只关注我们用的）
- 不改现有 template 硬编码逻辑（保留作为 fallback）
- 不改 `models.dev/api.json` 的 URL
- 不引入新的网络库（后端继续用 ureq）

## 2. Commands

```bash
make dev          # vite + cargo watch
make web-build    # cd web && npm run build
make test         # cargo test
make typecheck    # cd web && npx tsc --noEmit
```

## 3. Project Structure

### 后端修改

| 文件 | 改动 |
|---|---|
| `src/dao/mod.rs` | DAO trait 新增 `get_opencode_models(provider_id)` / `set_opencode_models(provider_id, models, updated_at)` |
| `src/dao/sqlite_impl.rs` | 新增 `opencode_model_cache` 表；实现 trait 方法 |
| `src/opencode_fetch.rs` | 新增 `fetch_provider_models(provider_id)` 只拉指定 provider；保留现有 `fetch_opencode_models()` 兼容 |
| `src/api/templates.rs` | `TemplateSummary` 新增 `opencode_models: Vec<String>` 字段（从 cache 读取） |
| `src/api/mod.rs` | 新增 `POST /api/opencode-models/refresh` endpoint |
| `src/main.rs` | 启动时 `tokio::spawn` 后台刷新（只刷新关注的 provider） |

### 前端修改

| 文件 | 改动 |
|---|---|
| `web/src/api/types.ts` | `Template` 新增 `opencodeModels: string[]` |
| `web/src/components/OpencodeModelSelect.tsx` | 改为读 `template.opencodeModels` 而不是 `template.models[].opencodeModelId` |
| `web/src/routes/SettingsPage.tsx` | 新增"刷新 OpenCode 模型列表"按钮 + `mutate` 调用 POST `/api/opencode-models/refresh` |
| `web/src/routes/__tests__/SettingsPage.test.tsx` | 新增测试 |

### 数据库迁移

```sql
CREATE TABLE IF NOT EXISTS opencode_model_cache (
    provider_id TEXT PRIMARY KEY,
    model_ids TEXT NOT NULL,  -- JSON array ["MiniMax-M3", ...]
    updated_at TEXT NOT NULL  -- ISO 8601
);
```

## 4. Code Style

### 后端（Rust）

```rust
// src/api/templates.rs
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TemplateSummary {
    pub id: String,
    pub display_name: String,
    pub opencode_provider_id: String,
    pub opencode_base_url: String,
    pub available_models: Vec<String>,
    pub models: Vec<TemplateModelSummary>,
    pub opencode_models: Vec<String>,  // ← 新增：从 cache 读取的该 provider 可用模型
}

pub async fn list_handler(State(state): State<AppState>) -> Result<Json<Vec<TemplateSummary>>, ApiError> {
    let dao = state.dao.lock().await;
    let templates = dao.get_templates();
    
    let mut result = Vec::new();
    for t in templates {
        let opencode_models = dao
            .get_opencode_models(&t.opencode_provider_id)
            .unwrap_or_default()
            .unwrap_or_default();  // cache 不存在时返回空 vec
        
        result.push(TemplateSummary {
            id: t.id.clone(),
            display_name: t.name.clone(),
            opencode_provider_id: t.opencode_provider_id.clone(),
            opencode_base_url: t.opencode_base_url.clone(),
            available_models: t.models.iter().map(|m| m.id.clone()).collect(),
            models: t.models.iter().map(|m| TemplateModelSummary {
                id: m.id.clone(),
                name: m.name.clone(),
                opencode_model_id: m.opencode_model_id.clone(),
            }).collect(),
            opencode_models,
        });
    }
    Ok(Json(result))
}
```

```rust
// src/main.rs
#[tokio::main]
async fn main() -> io::Result<()> {
    // ... 初始化 DAO + AppState ...
    
    // 启动时后台刷新 OpenCode 模型缓存（只刷新关注的 provider）
    let dao_for_refresh = state.dao.clone();
    tokio::spawn(async move {
        let providers = vec!["minimax-cn", "kimi-for-coding"];
        for provider_id in providers {
            match opencode_fetch::fetch_provider_models(provider_id).await {
                Ok(models) => {
                    let dao = dao_for_refresh.lock().await;
                    if let Err(e) = dao.set_opencode_models(provider_id, &models) {
                        tracing::warn!("failed to cache opencode models for {}: {}", provider_id, e);
                    }
                }
                Err(e) => {
                    tracing::warn!("failed to fetch opencode models for {}: {}", provider_id, e);
                }
            }
        }
    });
    
    // ... 启动 axum server ...
}
```

```rust
// src/api/mod.rs
pub fn router(state: AppState) -> Router {
    Router::new()
        // ... 现有路由 ...
        .route("/api/templates", get(templates::list))
        .route("/api/opencode-models/refresh", post(opencode_models::refresh))
        // ...
}
```

```rust
// src/api/opencode_models.rs（简化：只有一个 refresh handler）
use axum::{Json, extract::State};
use crate::api::state::AppState;
use crate::api::error::ApiError;

pub async fn refresh(State(state): State<AppState>) -> Result<Json<RefreshResponse>, ApiError> {
    let providers = vec!["minimax-cn", "kimi-for-coding"];
    let mut updated = vec![];
    let mut failed = vec![];
    
    for provider_id in providers {
        match crate::opencode_fetch::fetch_provider_models(provider_id).await {
            Ok(models) => {
                let dao = state.dao.lock().await;
                dao.set_opencode_models(provider_id, &models)
                    .map_err(|e| ApiError::Internal(e.to_string()))?;
                updated.push(provider_id.to_string());
            }
            Err(e) => {
                tracing::warn!("refresh failed for {}: {}", provider_id, e);
                failed.push((provider_id.to_string(), e));
            }
        }
    }
    
    Ok(Json(RefreshResponse { updated, failed }))
}

#[derive(Serialize)]
pub struct RefreshResponse {
    pub updated: Vec<String>,
    pub failed: Vec<(String, String)>,
}
```

### 前端（TypeScript）

```typescript
// web/src/api/types.ts
export interface Template {
  id: string;
  displayName: string;
  opencodeProviderId: string;
  opencodeBaseUrl: string;
  availableModels: string[];
  models: TemplateModel[];
  opencodeModels: string[];  // ← 新增：从 cache 读取的该 provider 可用模型
}
```

```typescript
// web/src/components/OpencodeModelSelect.tsx
interface Props {
  models: string[];  // ← 改为 string[]，外部传入 opencodeModels
  value: string;
  onChange: (value: string) => void;
}

export function OpencodeModelSelect({ models, value, onChange }: Props) {
  if (models.length === 0) {
    return (
      <input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder="defaults to model"
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
      <option value="">— default (use model id) —</option>
      {models.map((id) => (
        <option key={id} value={id}>{id}</option>
      ))}
    </select>
  );
}
```

```typescript
// web/src/components/InstanceForm.tsx
// OpencodeModelSelect 调用点改为：
<OpencodeModelSelect
  models={currentTemplate?.opencodeModels ?? []}  // ← 从 template.opencodeModels 读
  value={values.opencodeModelId ?? ''}
  onChange={(v) => set('opencodeModelId', v)}
/>
```

```typescript
// web/src/routes/SettingsPage.tsx
// 新增刷新按钮
import { useMutation, useQueryClient } from '@tanstack/react-query';

function useRefreshOpencodeModels() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: () => apiPost('/api/opencode-models/refresh', {}),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ['templates'] });
    },
  });
}

// 在 SettingsPage 中：
const refresh = useRefreshOpencodeModels();
<button
  onClick={() => refresh.mutate()}
  disabled={refresh.isPending}
>
  {refresh.isPending ? 'Refreshing…' : 'Refresh OpenCode Models'}
</button>
```

## 5. Testing Strategy

### 后端

| 测试 | 文件 | 验证 |
|---|---|---|
| `test_opencode_models_cache_roundtrip` | `tests/opencode_models_test.rs` | `set_opencode_models` → `get_opencode_models` 能读回 |
| `test_templates_list_includes_opencode_models` | `tests/templates_test.rs` | `/api/templates` 响应包含 `opencodeModels` 字段 |
| `test_fetch_provider_models_filters_by_provider` | `tests/opencode_fetch_test.rs` | `fetch_provider_models("minimax-cn")` 只返回 minimax 模型 |
| `test_refresh_endpoint_updates_cache` | `tests/opencode_models_test.rs` | POST `/api/opencode-models/refresh` 成功更新 cache |

### 前端

| 测试 | 文件 | 验证 |
|---|---|---|
| `OpencodeModelSelect: renders options from opencodeModels` | `__tests__/OpencodeModelSelect.test.tsx` | 传入 `models={["a", "b"]}` 渲染 2 个 options |
| `OpencodeModelSelect: fallback to input when empty` | 同上 | `models={[]}` 时降级为 input |
| `SettingsPage: refresh button triggers POST` | `__tests__/SettingsPage.test.tsx` | 点击刷新按钮调 `/api/opencode-models/refresh` |

## 6. Boundaries

### Always do
- 跑 `cargo test` + `npx tsc --noEmit` 再 commit
- 只缓存 `templates.rs` 中实际出现的 `opencode_provider_id`
- 后台刷新失败时静默（只打 log，不阻断启动）
- 前端 `opencodeModels` 为空时降级为 input（和现有行为一致）

### Ask first
- 改 DAO trait（需要新增方法）
- 改数据库 schema（新增表）
- 引入新的 HTTP 客户端库（如 reqwest）替代 ureq

### Never do
- 启动时同步拉取 models.dev（会阻塞启动）
- 缓存全部 138 个 provider（浪费存储）
- 把 `models.dev/api.json` 完整内容存 sqlite（我们只存关注的 provider 的 model ids）
- 新增 `/api/opencode-models/:providerId` 独立 GET endpoint（直接合并进 `/api/templates`）

## 7. 任务分解

| Task | 描述 | 验收 | 估时 |
|---|---|---|---|
| **T1** | **后端：DAO + 表 + 单 provider fetch**<br/>- `opencode_fetch.rs` 新增 `fetch_provider_models(provider_id)`<br/>- DAO trait 新增 `get/set_opencode_models`<br/>- sqlite 新增 `opencode_model_cache` 表 | `cargo test` pass；cache roundtrip 测试通过 | 25 min |
| **T2** | **后端：/api/templates 合并 opencode_models + refresh endpoint**<br/>- `templates::list` handler 读 cache 合并到 `TemplateSummary`<br/>- 新增 `POST /api/opencode-models/refresh`<br/>- `main.rs` 启动时 `tokio::spawn` 后台刷新 | `cargo test` pass；`curl /api/templates` 看到 `opencodeModels` | 20 min |
| **T3** | **前端：OpencodeModelSelect 改读 opencodeModels**<br/>- `Template` type 加 `opencodeModels`<br/>- `OpencodeModelSelect` props 改为 `models: string[]`<br/>- `InstanceForm` + `InstanceDetailPage` 传 `template.opencodeModels` | 现有测试 pass；typecheck 0 error | 15 min |
| **T4** | **前端：SettingsPage 手动刷新按钮**<br/>- 新增 `useRefreshOpencodeModels()` mutation hook<br/>- SettingsPage 加按钮 + 刷新后 invalidate templates query | 测试覆盖；手动验证按钮刷新 | 15 min |
| **T5** | **Chrome DevTools 端到端验证**<br/>- 验证 OpenCode 下拉有 7 个 minimax 模型<br/>- 验证 Settings 刷新按钮工作 | 截图 + 网络面板确认 | 10 min |

### 依赖关系

```
T1 ──→ T2 ──→ T3 ──→ T5
            └─→ T4 ──→ T5
```

T1/T2 是后端基础；T3/T4 可并行；T5 验证。

## 8. 与 TUI 版本的关键差异

| | TUI (main 分支) | Web (简化方案) |
|---|---|---|
| 缓存位置 | 内存 (`app.opencode_model_cache`) | sqlite (`opencode_model_cache` 表) |
| 缓存持久化 | ❌ 进程退出即丢失 | ✅ 重启后仍在 |
| 刷新时机 | 启动时同步拉取（阻塞） | 启动时后台 spawn（非阻塞） |
| 手动刷新 | ❌ 无 | ✅ Settings 页面按钮 |
| 数据合并 | `template.models + cache` 合并去重 | 只使用 cache（template 硬编码作为 fallback） |
| API | 内存直接读 | `/api/templates` 响应包含 `opencodeModels` |

## 9. Open Questions

无。用户已确认：
- 只缓存关注的 provider（minimax-cn、kimi-for-coding）
- 启动时读 cache + 后台刷新 + 手动按钮兜底
- 缓存放 sqlite（现有 db.sqlite）
- 不新增独立 API，合并进 `/api/templates`
