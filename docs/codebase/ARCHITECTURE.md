# cc-switch-tui 架构文档

## Pattern Overview

Use a **Layered Architecture** with a Rust + Axum backend serving an embedded React SPA, with a transparent HTTP proxy as a sidecar concern and trace capture as a cross-layer capability.

Key characteristics:

1. **Single-binary deployment**: The Rust binary embeds the compiled frontend via `include_dir!` in `src/api/static_fallback.rs`, eliminating separate frontend hosting.
2. **Local-first persistence**: Two SQLite databases in `~/.cc-switch-tui/{db.sqlite, traces.sqlite}` with zero external infrastructure.
3. **Template-instance pattern**: Built-in `ProviderTemplate` definitions in `src/templates.rs` are immutable; users create `ProviderInstance` records that reference templates and override specific fields. ID format: `{template_id}-{alias}`.
4. **Shell integration as core feature**: The system generates zsh function files (`~/.cc-switch-tui/aliases.zsh`) and opencode JSON configs (`~/.cc-switch-tui/opencode/{alias}.json`) rather than mutating a global state. The `ys-proxy` wrapper aliases `cl-*` functions through the local proxy on `http://127.0.0.1:7480/ys-proxy/{alias}/...`.
5. **Transparent proxy with side-channel trace capture**: `/ys-proxy/{alias}/*` routes forward Anthropic-compatible requests to upstream providers; concurrent background tasks parse the SSE stream and persist request/response pairs to the trace store. The frontend reads from the trace store and gets real-time updates via SSE broadcast.
6. **Port agility**: The server binds to a cached port (`src/port.rs::try_bind` scans `start..start+100`) and writes the actual port to `~/.cc-switch-tui/port` for client discovery.

## Layers

### 1. Frontend (Presentation Layer)

- **Location**: `web/`
- **Technology**: React 18.3, Vite 5, Tailwind CSS 3, React Router 6, TanStack Query 5, Zod 3, Vitest 4
- **Responsibilities**:
  - Render the SPA for managing provider instances and inspecting trace sessions
  - Communicate with backend via REST API (`/api/*`) and receive trace events via SSE (`/api/traces/events`)
  - Handle client-side routing and 1-minute cached prefetch of templates on app start
- **Build output**: Compiled to `web-dist/` (Vite build) and embedded into the Rust binary at compile time via `include_dir!("$CARGO_MANIFEST_DIR/web-dist")` in `src/api/static_fallback.rs`
- **Consumed by**: End users via browser (auto-opened on startup by `src/main.rs` when `settings.auto_open_browser` is true)
- **Module structure**:
  - `web/src/main.tsx` — root, wires `QueryClientProvider` + `BrowserRouter`
  - `web/src/App.tsx` — top-level layout, `Routes` declaration, nav
  - `web/src/api/{client,hooks,types,traces}.ts` — typed API client + TanStack Query hooks
  - `web/src/lib/{curl,traceParser,diff,mask,validate}.ts` — pure helpers (curl rebuild, trace parse, message diff, sensitive masking, form validation)
  - `web/src/pages/{Instances,Apply,Config,Settings}Page.tsx` — provider management pages
  - `web/src/pages/traces/{Dashboard,Viewer,DiffView,TokenBadge}.tsx` — trace viewer surface (current main work area on branch `feature/enhanced-trace-viewer-0613`)
  - `web/src/hooks/{useTraceEvents,useTraceSearch}.ts` — SSE subscription + search state
  - `web/src/components/{ui,GlobalSearch,...}/` — shared primitives

### 2. API Layer (Transport / Handler Layer)

- **Location**: `src/api/`
- **Technology**: Axum 0.7, Tower, Tokio
- **Responsibilities**:
  - Define HTTP routes in `src/api/mod.rs::router` and route them to handler functions
  - Extract and validate request parameters (path, query, JSON body)
  - Convert domain errors into HTTP responses via `ApiError::IntoResponse` in `src/api/error.rs`
  - Serve embedded static files with SPA fallback
  - Publish trace events over SSE
- **Key files**:
  - `src/api/mod.rs` — `router(state)` declares all routes; `/api/*` are JSON, `/ys-proxy/{alias}/*` is the proxy passthrough, all other paths fall back to `spa_fallback`
  - `src/api/instances.rs` — CRUD for provider instances with `InstanceSummary` (no apiKey) and `InstanceDetail` (with apiKey) DTOs
  - `src/api/traces.rs` — session list / get / records / jsonl export / html export / clear / SSE event stream
  - `src/api/aliases.rs` — render `aliases.zsh` content (GET) and write it to disk (POST apply)
  - `src/api/opencode.rs` — render (GET) and write (POST apply) opencode config JSON per instance
  - `src/api/templates.rs` — list built-in provider templates for the Web UI (camelCase DTOs with deprecated `availableModels` + new `models` + `opencodeModels`)
  - `src/api/error.rs` — `ApiError` enum + `IntoResponse` mapping to status codes (404/400/409/500) and structured JSON error body `{error: {code, message, field?}}`
  - `src/api/state.rs` — `AppState` shared across all handlers (Arc'd dao + settings + trace_store + broadcast sender)
  - `src/api/static_fallback.rs` — SPA fallback serving embedded `web-dist/`; non-`/api` paths that don't match a static file fall back to `index.html`
- **Depends on**: Domain layer, DAO layer, Shell/Infrastructure, Proxy
- **Used by**: External HTTP clients (browser, curl, tests, the proxy itself for upstream lookups)

### 3. Domain Layer (Business Logic Layer)

- **Location**: `src/domain/`
- **Responsibilities**:
  - Define core data structures and validation rules
  - Contain pure business logic with no I/O dependencies
  - Define the unified error type `AppError`
- **Key files**:
  - `src/domain/instance.rs` — `ProviderInstance` struct and `validate_alias()` function (lowercase alnum + `-_`, length 1-32, no whitespace)
  - `src/domain/template.rs` — `ProviderTemplate` (immutable built-in definition with `default_env`, `models`, `opencode_*` fields, `opencode_models: Vec<OpenCodeModel>`) and `ModelTemplate` (per-model env overrides; `context_window` field removed — window size now baked into `env_overrides` literal)
  - `src/domain/error.rs` — `AppError` enum: `InstanceAlreadyExists`, `InstanceNotFound`, `TemplateNotFound`, `ModelNotFound`, `Database(String)`, `InvalidAlias(String)`, `AliasAlreadyExists(String)`
- **Depends on**: None (pure Rust + chrono + thiserror)
- **Used by**: API layer, DAO layer, Shell layer, Proxy layer, Trace layer

### 4. DAO Layer (Data Access Layer)

- **Location**: `src/dao/`
- **Responsibilities**:
  - Abstract storage operations behind the `Dao` trait
  - Persist provider instances to SQLite with in-place schema migration
  - Cache all instances in memory for fast reads (refresh on every mutation)
- **Key files**:
  - `src/dao/mod.rs` — `Dao` trait defining the contract (`get_templates`, `list_instances`, `create_instance`, `update_instance`, `set_alias`, `set_opencode_model_id`, `set_kv_cache_enabled`, `rename_instance`; `set_context_window_enabled` removed)
  - `src/dao/sqlite_impl.rs` — `SqliteDaoImpl` with SQLite CRUD; schema created with `CREATE TABLE IF NOT EXISTS`, missing columns added with `PRAGMA table_info` checks followed by `ALTER TABLE`
  - `src/dao/memory_impl.rs` — `MemoryDaoImpl` for tests
- **Depends on**: Domain layer
- **Used by**: API layer via `AppState.dao` (also by the Proxy for upstream lookups)
- **Note**: The `TraceStore` is a separate sibling to `Dao`, not under it — it has its own SQLite database and trait-free API in `src/trace/store.rs`.

### 5. Infrastructure / Shell Layer

- **Location**: `src/shell.rs`, `src/opencode_config.rs`, `src/port.rs`, `src/templates.rs`, `src/data_migration.rs`
- **Responsibilities**:
  - Generate zsh function files (`aliases.zsh`) with `cl-*` aliases for `claude` and `oc-*` aliases for `opencode`
  - Generate opencode JSON configuration files per instance
  - Manage TCP port binding, port file caching, and graceful shutdown
  - Register built-in provider templates (currently `minimax_template()` and `kimi_template()`)
  - Migrate legacy project-local `.cc-switch-tui/` SQLite files into the home directory on first run
- **Key files**:
  - `src/shell.rs` — `render_aliases()` (preview), `generate_aliases()` (write), `shell_escape()`, `ensure_zshrc_source()`
  - `src/opencode_config.rs` — `render_opencode_config()`, `write_opencode_config()` (mode 0o600), `build_opencode_aliases()`, `generate_opencode_configs()`
  - `src/port.rs` — `try_bind()` (start..start+max_attempts scan), `read_cached_port()`, `write_port_file()`, `clear_port_file()`, `wait_for_shutdown()`
  - `src/templates.rs` — `register_templates()` returning the built-in `Vec<ProviderTemplate>` (MiniMax + Kimi)
  - `src/data_migration.rs` — `default_cc_dir()` (returns `~/.cc-switch-tui`), `ensure_data_migrated()` (copies legacy project-local DB on first run)
- **Depends on**: Domain layer
- **Used by**: API layer (`/api/aliases`, `/api/opencode-config`, etc.), `src/main.rs`

### 6. Proxy Layer

- **Location**: `src/proxy/`
- **Responsibilities**:
  - Forward Anthropic-compatible API requests (`/v1/messages`, `/v1/complete`, `/v1/models`) from `claude`/`opencode` invocations to the configured upstream provider
  - Inject the per-instance `Authorization: Bearer <api_key>` and `ANTHROPIC_BASE_URL` from the template
  - Strip hop-by-hop headers (RFC 7230 §6.1) before forwarding
  - Buffer or stream the response back to the client
  - For streaming requests, fork a background task that parses SSE and persists trace records
- **Key files**:
  - `src/proxy/handler.rs` — `proxy_handler()` is the single entry point registered at `/ys-proxy/{alias}/*path`; it inspects method/path/body, looks up the instance + template, and dispatches to either the streaming or non-streaming branch
  - `src/proxy/upstream.rs` — `UpstreamClient` wraps `reqwest::Client`; `forward()` buffers, `forward_streaming()` returns the live `reqwest::Response` for incremental reads
  - `src/proxy/filter.rs` — `filter_headers()` (drops hop-by-hop, optionally redacts `authorization`/`cookie`/`x-api-key` etc. with `Bearer <REDACTED>`), `header_map_to_json()`
  - `src/proxy/parser.rs` — `AnthropicParser::parse_request()` / `parse_response()` / `apply_streaming_event()`; `StreamingAccumulator` joins `content_block_delta` text
  - `src/proxy/sse.rs` — `SseParser` is a streaming SSE byte buffer (handles `\r\n`, partial chunks, `flush()`)
  - `src/proxy/session_extractor.rs` — `extract_claude_session_id()` (parses `metadata.user_id` JSON for `session_id`, max 128 chars) and `redact_user_id_pii()` (replaces `device_id` / `account_uuid` with `***` in-place)
- **Depends on**: DAO layer, Domain layer, Trace layer
- **Used by**: API layer (route registration in `src/api/mod.rs`)

### 7. Trace Layer

- **Location**: `src/trace/`
- **Responsibilities**:
  - Persist Claude Code conversation sessions to `~/.cc-switch-tui/traces.sqlite`
  - Capture request + SSE-derived response pairs, with HTTP envelope (method, path, redacted headers, upstream base URL) and per-session metrics (token counts, duration)
  - Provide CRUD + list + filter + export (JSONL + HTML) + SSE-broadcast of changes
- **Key files**:
  - `src/trace/mod.rs` — module facade (`exporter`, `models`, `store`)
  - `src/trace/models.rs` — `TraceDirection` (Request/Response), `SessionFilter` (date/provider/status/search), `TraceSession` (with extended metrics fields), `TraceRecord` (with HTTP envelope fields added in 0613)
  - `src/trace/store.rs` — `TraceStore` with WAL-mode SQLite connection, `init_schema()` (idempotent `CREATE TABLE IF NOT EXISTS` + `ALTER TABLE` migrations keyed off `duplicate column` errors), `create_session`, `append_record`, `complete_session` (transactional), `list_sessions`, `count_sessions`, `get_records`, `get_all_records`, `delete_session`, `clear_all` (with `VACUUM`), `update_session_metrics`, `finalize_session`. Search uses `LIKE` with `\\` escape for wildcards.
  - `src/trace/exporter.rs` — `export_html()` renders a self-contained HTML report (CSS adapts to dark mode) with HTML-escaped payloads; no external assets
- **Depends on**: Domain layer (`AppError` only)
- **Used by**: Proxy layer (records), API layer (queries + broadcasts), Frontend (viewer)

## Data Flow

### HTTP Request Flow (Provider CRUD)

```
Browser → Axum Router (src/api/mod.rs)
    → Handler (e.g., src/api/instances.rs::create)
        → Validate input (validate_alias, AppError → ApiError)
            → Lock AppState.dao (Arc<Mutex<SqliteDaoImpl>>)
                → Call Dao method (mutates SQLite, refreshes in-memory cache)
                    → Return domain object
        → Map to DTO (InstanceSummary / InstanceDetail)
            → JSON response
```

### Alias Generation Flow

```
POST /api/aliases/apply (src/api/aliases.rs::apply)
    → Lock dao, snapshot instances + templates
        → shell::render_aliases() builds zsh function definitions
            → opencode_config::generate_opencode_configs() writes JSON configs (mode 0o600)
        → Write ~/.cc-switch-tui/aliases.zsh
            → Return { path: ... }
```

The `aliases.zsh` file contains:

- `__cc_switch_print_env <alias>` helper — emits a single `[cc-switch-tui] <alias>: KEY=VALUE ...` line to stderr showing the actual env that will be passed to `claude` (credential-like keys redacted as `<redacted>`, missing keys shown as `<unset>`; suppressed when `CC_SWITCH_QUIET=1`).
- Per-instance `cl-{alias}` function: wraps function body in `(...)` subshell so all `export`s are isolated from the parent shell (POSIX subshell isolation; see `feat/cl-subshell-wrap`). Exports its own env values verbatim from `default_env + env_overrides`, then applies `CC_SWITCH_PROXY_URL` (if set via the zsh command-prefix form) to `ANTHROPIC_BASE_URL`, calls the diagnostic helper, and runs `command claude "$@"`. Context-window related env vars (`CLAUDE_CODE_AUTO_COMPACT_WINDOW` etc.) are now baked into the model template's `env_overrides` literal — no per-instance toggle.
- `ys-proxy` wrapper: uses the zsh command-prefix form `CC_SWITCH_PROXY_URL="http://127.0.0.1:7480/ys-proxy/${alias_name}" $alias_name "$@"` so the sentinel is visible only for the duration of the call (never leaks back to the parent shell) and the underlying `cl-*` function applies it after exporting its own default.
- `oc-{alias}` functions set `OPENCODE_CONFIG` and the provider env var before `command opencode "$@"`.

> Why the sentinel form: a previous design used `${ANTHROPIC_BASE_URL:-default}` to let `ys-proxy` win when its subshell set the URL. That subshell never leaked, so the fall-through branch was unreachable from `ys-proxy` while a *previous* direct `cl-*` call could leak its `ANTHROPIC_BASE_URL` into the parent shell and be picked up incorrectly by the next call. Always exporting the function's own default and using a named sentinel for the proxy override eliminates the leak entirely while keeping the proxy path working. Regression coverage: `src/shell.rs::tests::test_function_body_isolates_previous_alias_export` (real-zsh repro) and `test_ys_proxy_sentinel_overrides_anthropic_base_url` (proxy path).

### Proxy + Trace Capture Flow (Streaming)

```
claude / opencode → POST /ys-proxy/cl-{alias}/v1/messages (src/proxy/handler.rs)
    → Validate alias → look up instance + template via AppState.dao
        → Read + parse request body (extract claude_session_id, redact user_id PII)
            → forward_streaming() to UpstreamClient
                → Return bytes_stream() to client
                → Fork tokio::spawn task that:
                    → SseParser::feed() chunks
                        → On first message_start: TraceStore::create_session() + append_record(Request, …) and broadcast TraceEvent::SessionUpdated
                        → AnthropicParser::apply_streaming_event() updates StreamingAccumulator
                    → On stream end: TraceStore::complete_session() (single transaction inserts response record + updates session metrics + sets status='complete') and broadcast TraceEvent::SessionUpdated
```

### Trace Viewer Real-Time Flow

```
Browser opens /traces/:id (web/src/pages/traces/Viewer.tsx)
    → useSession() + useRecords() TanStack Queries
        → useTraceEvents() subscribes to /api/traces/events (SSE)
            → On TraceEvent::SessionUpdated or RecordAdded: invalidateQuery(['trace-session', id]) + ['trace-records', id]
                → Re-fetch, re-render conversation tab
```

### State Management

- **Server state**:
  - Provider instances: `Arc<Mutex<SqliteDaoImpl>>` in `AppState`; in-memory cache refreshed on every mutation
  - Settings: `Arc<RwLock<Settings>>` in `AppState`; in-memory only, resets on restart
  - Trace store: `Arc<Mutex<TraceStore>>` in `AppState`
  - Trace events: `broadcast::Sender<TraceEvent>` channel (capacity 16) in `AppState` for SSE fan-out
- **Client state**:
  - Server state cached with TanStack Query; templates prefetched on app start with `staleTime: 60_000`
  - UI state (active tab, search query, modal open, copied-curl) is local `useState` in components
  - Theme toggle persists to `localStorage`
- **Persistence**: Only SQLite is durable (`~/.cc-switch-tui/db.sqlite` + `~/.cc-switch-tui/traces.sqlite`); `Settings`, in-memory caches, and the port file are non-durable / non-authoritative

## Key Abstractions

### `Dao` Trait

Located in `src/dao/mod.rs`. The trait abstracts provider configuration storage:

```rust
pub trait Dao {
    fn get_templates(&self) -> Vec<&ProviderTemplate>;
    fn get_template(&self, id: &str) -> Option<&ProviderTemplate>;
    fn list_instances(&self) -> Vec<&ProviderInstance>;
    fn get_instance(&self, id: &str) -> Option<&ProviderInstance>;
    fn create_instance(&mut self, instance: ProviderInstance) -> Result<(), AppError>;
    fn delete_instance(&mut self, id: &str) -> Result<(), AppError>;
    fn update_instance(&mut self, id: &str, model_id: String, alias: String, api_key: String) -> Result<(), AppError>;
    fn set_alias(&mut self, id: &str, alias: String) -> Result<(), AppError>;
    fn rename_instance(&mut self, old_id: &str, new_id: &str, alias: String) -> Result<(), AppError>;
    fn set_opencode_model_id(&mut self, id: &str, opencode_model_id: String) -> Result<(), AppError>;
    fn set_kv_cache_enabled(&mut self, id: &str, enabled: bool) -> Result<(), AppError>;
}
```

Use this trait when adding new storage backends. Every method is `&mut self` because `SqliteDaoImpl` mutates its in-memory cache on every change.

### `AppState`

Located in `src/api/state.rs`. Shared application state injected into all Axum handlers:

```rust
#[derive(Clone)]
pub struct AppState {
    pub dao: Arc<Mutex<SqliteDaoImpl>>,
    pub settings: Arc<RwLock<Settings>>,
    pub trace_store: Arc<Mutex<TraceStore>>,
    pub trace_event_tx: broadcast::Sender<TraceEvent>,
}
```

Clone `AppState` freely — it is reference-counted internally. `TraceEvent` has two variants: `SessionUpdated { session_id }` and `RecordAdded { session_id, record_index }`. The trace HTTP handler subscribes to the broadcast sender in `src/api/traces.rs::events_handler`.

### `AppError`

Located in `src/domain/error.rs`. Use this `thiserror`-derived enum for all domain-level failures. The full set of variants is enumerated above in the Domain Layer section.

### `ApiError`

Located in `src/api/error.rs`. Use this for HTTP transport. Construct with the helper functions, not the variants directly:

```rust
ApiError::not_found("instance {id} not found")
ApiError::validation("alias", "alias cannot be empty")
ApiError::conflict("alias", "alias already exists")
ApiError::internal(e.to_string())
```

HTTP status mapping: `NotFound → 404`, `Validation → 400`, `Conflict → 409`, `Internal → 500`. Error codes in the JSON body: `NOT_FOUND`, `VALIDATION_ERROR`, `ALIAS_CONFLICT`, `INTERNAL_ERROR`.

### `ProviderTemplate` / `ProviderInstance`

Located in `src/domain/template.rs` and `src/domain/instance.rs`.

- `ProviderTemplate` (`src/domain/template.rs`): Immutable built-in definition of a third-party provider. Carries `default_env: HashMap<String, String>`, `models: Vec<ModelTemplate>`, plus `opencode_*` metadata (`opencode_provider_id`, `opencode_npm`, `opencode_base_url`, `opencode_env_var`, `opencode_models: Vec<OpenCodeModel>`). Built by builder functions in `src/templates.rs::register_templates()`.
- `ProviderInstance` (`src/domain/instance.rs`): User-created record referencing a template via `template_id`, with user-specific overrides (`api_key`, `model_id`, `alias`, `opencode_model_id`, `kv_cache_enabled: bool`; `context_window_enabled` removed). ID format: `{template_id}-{alias}`.

### `TraceSession` / `TraceRecord`

Located in `src/trace/models.rs`. Frontend TypeScript counterparts in `web/src/api/traces.ts` (note: Rust `snake_case` ↔ TypeScript `camelCase` mapping is implicit on the Rust side via serde defaults; on the TS side the field names already match the JSON the backend returns).

- `TraceSession`: aggregates one conversation (alias/provider/model, status, record_count, duration_ms, input/output/cache_read/cache_create/total tokens, first_user, last_response, summary_json, date_key, started_at, updated_at).
- `TraceRecord`: a single request/response row with HTTP envelope (method, path, headers_json, upstream_base_url) and `claude_session_id` for cross-session correlation.

## Entry Points

### Main Binary

- **File**: `src/main.rs`
- **Trigger**: `cargo run` or executing the compiled binary `cc-switch-tui`
- **Responsibilities**:
  1. Initialize `tracing` subscriber to `app.log` (append mode, no ANSI, env filter)
  2. Resolve `~/.cc-switch-tui` via `data_migration::default_cc_dir()` and migrate any legacy project-local DB
  3. Load templates via `register_templates()`
  4. Initialize `SqliteDaoImpl` with `~/.cc-switch-tui/db.sqlite` and `TraceStore` with `~/.cc-switch-tui/traces.sqlite`
  5. Construct `AppState` (initializes broadcast channel)
  6. Bind port via `port::try_bind(DEFAULT_PORT=7480, max_attempts=100)` and write actual port to `~/.cc-switch-tui/port`
  7. Optionally auto-open browser when `settings.auto_open_browser` is true
  8. Serve the axum router with `graceful_shutdown` triggered by Ctrl-C / SIGTERM (`port::wait_for_shutdown` cleans up the port file)

### Library Entry

- **File**: `src/lib.rs`
- **Purpose**: Re-exports all public modules for integration tests and external consumers
- **Re-exports**: `api`, `dao`, `data_migration`, `domain`, `opencode_config`, `opencode_fetch`, `port`, `proxy`, `shell`, `templates`, `trace`

### Frontend Entry

- **File**: `web/src/main.tsx`
- **Build command**: `make web-build` (runs `tsc --noEmit && vite build`, outputs to `web-dist/`)
- **Note**: After editing `web/src/**`, run `make web-build` and `make build` so the embedded assets in the next `cargo build` pick up the changes. The Makefile comment on `web-build` is explicit: "web-dist/ 已就绪 → 下次 cargo build 自动 embed".

### Route Registration

- **File**: `src/api/mod.rs::router`
- Mounts:
  - `/api/health` (GET)
  - `/api/instances` (GET, POST), `/api/instances/:id` (GET, PATCH, DELETE), `/api/instances/:id/duplicate` (POST)
  - `/api/templates` (GET)
  - `/api/aliases` (GET), `/api/aliases/apply` (POST)
  - `/api/opencode-config/:id` (GET), `/api/opencode-config/:id/apply` (POST)
  - `/api/config/export` (GET), `/api/config/import` (POST)
  - `/api/settings` (GET, PUT)
  - `/api/diagnostics` (GET)
  - `/api/traces/sessions` (GET, DELETE), `/api/traces/sessions/:id` (GET, DELETE), `/api/traces/sessions/:id/records` (GET), `/api/traces/sessions/:id/export/jsonl` (GET), `/api/traces/sessions/:id/export/html` (GET), `/api/traces/events` (GET, SSE)
  - `/ys-proxy/:alias/*path` (any method → `proxy_handler`)
  - SPA fallback: any unmatched non-`/api` path → `web-dist/index.html`

## Error Handling

### Domain Errors (`AppError`)

Use `AppError` in `src/domain/error.rs` for all domain-level failures. Variants:

- `InstanceAlreadyExists(String)` — primary key conflict on insert
- `InstanceNotFound(String)` — `UPDATE`/`DELETE` affected 0 rows
- `TemplateNotFound(String)` — instance's `template_id` doesn't match a registered template
- `ModelNotFound(String)` — instance's `model_id` doesn't match any model in the template
- `Database(String)` — wraps rusqlite errors; also used as the catch-all for upstream HTTP failures in `src/proxy/upstream.rs`
- `InvalidAlias(String)` — alias validation failure
- `AliasAlreadyExists(String)` — reserved for future rename flows

### API Errors (`ApiError`)

Map `AppError` to `ApiError` in handlers, then rely on `IntoResponse` for uniform JSON:

```rust
// Standard mapping pattern from src/api/instances.rs
dao.create_instance(new_instance).map_err(|e| match e {
    AppError::InstanceAlreadyExists(_) => ApiError::conflict("alias", "alias already exists"),
    AppError::InvalidAlias(msg)        => ApiError::validation("alias", msg),
    AppError::InstanceNotFound(_)      => ApiError::not_found("instance not found"),
    e                                  => ApiError::internal(e.to_string()),
})?;
```

HTTP status codes: `400` (validation), `404` (not found), `409` (alias conflict), `500` (internal). The wire body is `{"error": {"code": "...", "message": "...", "field": "..."}}` — the `field` is omitted for `NotFound` and `Internal`.

### Logging

Use `tracing::{info, warn, error}` (not `println`). Log destination is `app.log` in the working directory (append mode, no ANSI). Set `RUST_LOG` to control verbosity (default `INFO`).

## Cross-Cutting Concerns

### Logging

- **Implementation**: `tracing` + `tracing-subscriber` in `src/main.rs`
- **Configuration**: `RUST_LOG` env var (defaults to `INFO`)
- **Output**: `app.log` (append mode, no ANSI)
- **Pattern**: Use `tracing::info!`, `tracing::warn!`, `tracing::error!` throughout handlers, infrastructure, and the proxy

### Database Schema Migration

- **DAO (`src/dao/sqlite_impl.rs::new`)**: `CREATE TABLE IF NOT EXISTS instances` on init, then `PRAGMA table_info` check followed by `ALTER TABLE instances ADD COLUMN <col> DEFAULT ...` for `alias`, `opencode_model_id`, `kv_cache_enabled`. After schema setup, idempotent migration runs: `UPDATE instances SET model_id = 'MiniMax-M3[1m]' WHERE model_id = 'MiniMax-M3'` (rename old model id) and `ALTER TABLE instances DROP COLUMN context_window_enabled` (SQLite ≥ 3.35). No external migration tool.
- **Trace store (`src/trace/store.rs::init_schema`)**: `CREATE TABLE IF NOT EXISTS` for `sessions` and `records`; for incremental columns, run an `ALTER TABLE` and ignore errors whose message contains `duplicate column` (the SQLite error text); any other error is surfaced as `AppError::Database`. `clear_all()` follows `DELETE FROM sessions` with `VACUUM` to reclaim space.

### Static Asset Embedding

- **Implementation**: `include_dir!` in `src/api/static_fallback.rs::WEB_DIST`
- **Pattern**: `include_dir!("$CARGO_MANIFEST_DIR/web-dist")` is evaluated at compile time; the resulting `Dir` exposes `get_file(path)` for individual assets. `spa_fallback(uri)` looks up the requested path; if not found, it returns `index.html` so React Router handles the client route. `web-dist/index.html` is expected to be present at compile time (the `expect()` message documents this).

### Port Management

- **Implementation**: `src/port.rs`
- **Pattern**: `try_bind(start, max_attempts)` iterates `start..start+max_attempts`; on success returns `(TcpListener, port)`. `write_port_file` creates the parent dir if needed. `clear_port_file` is called by `wait_for_shutdown`'s cleanup closure. `wait_for_shutdown` waits on `Ctrl-C` and (on unix) `SIGTERM` with `tokio::select!`, then sleeps 300 ms before invoking cleanup.

### Input Validation

- **Alias validation** (`src/domain/instance.rs::validate_alias`): lowercase ASCII alnum + `-`/`_`, length 1-32, no whitespace. Called both at the `Dao` layer (e.g. `create_instance`, `update_instance`, `set_alias`, `rename_instance`) and at the API handler layer for fail-fast on `POST /api/instances`.
- **No external validation framework**: Validation is manual function calls. The frontend uses Zod schemas in route components (e.g. `web/src/lib/validate.ts`).

### Header Filtering & Redaction (Proxy)

- **Hop-by-hop headers** (`src/proxy/filter.rs::HOP_BY_HOP`): `connection`, `host`, `keep-alive`, `proxy-authenticate`, `proxy-authorization`, `te`, `trailers`, `transfer-encoding`, `upgrade` — always stripped before forwarding.
- **Sensitive headers** (`SENSITIVE_HEADERS`): `authorization`, `cookie`, `set-cookie`, `set-cookie2`, `x-api-key`, `x-amz-security-token` — redacted to `Bearer <REDACTED>` (or `<REDACTED>` when no scheme) for the trace copy, but preserved verbatim on the live upstream request.
- **Authorization injection** (`src/proxy/upstream.rs::send_request`): the per-instance `api_key` is always re-injected as `Authorization: Bearer <api_key>` regardless of what the client sent, and any incoming `authorization` header is skipped to avoid duplicates.

### PII Redaction

- **`src/proxy/session_extractor.rs`**: when the request body has `metadata.user_id` as a JSON-encoded string, `redact_user_id_pii` replaces `device_id` and `account_uuid` with `***` in place before storing. `extract_claude_session_id` pulls `session_id` from the same nested JSON (max 128 chars). Both no-op gracefully when the structure is missing or malformed.

### Trace Data Integrity

- **`TraceStore::complete_session`** in `src/trace/store.rs` wraps the final response insert + session metrics update + status change in a single `unchecked_transaction()` so the viewer never sees a half-finalized session.
- **WAL mode** is enabled in `TraceStore::new` for concurrent reader/writer friendliness; `foreign_keys` is also enabled so the `ON DELETE CASCADE` on `records.session_id` works.

### Sensitive Display in Web UI

- **`web/src/lib/mask.ts`**: detects env var names ending in `_KEY`/`_TOKEN`/`_SECRET`/`_PASSWORD`/`_CREDENTIAL(S)` and masks their values to `head(3)***tail(4)` (or `***` for ≤8 chars). Used only in the Aliases preview page; the on-disk `aliases.zsh` keeps plaintext `export` lines because zsh needs the real value at runtime.

## Where to Add New Code

### New API Endpoint

1. Add the handler function in the appropriate `src/api/*.rs` module (or create a new module and re-export it in `src/api/mod.rs`)
2. Wire the route in `src/api/mod.rs::router()` — append to the chain, do not reorder existing routes
3. Add request/response DTOs alongside the handler; use `#[serde(rename_all = "camelCase")]` for Rust DTOs that the frontend consumes
4. Map `AppError` → `ApiError` in the handler with the standard match pattern; use `ApiError::validation/not_found/conflict/internal` helpers
5. If the endpoint produces real-time events, broadcast a `TraceEvent` via `AppState.trace_event_tx`

### New Domain Model or Validation

1. Add the struct/enum to the appropriate file in `src/domain/`
2. Add validation as pure functions in the same file
3. Add new error variants to `src/domain/error.rs` if needed; map them in API handlers
4. Write unit tests in the same module under `#[cfg(test)]`

### New Storage Operation

1. Add the method signature to the `Dao` trait in `src/dao/mod.rs`
2. Implement it in `src/dao/sqlite_impl.rs` — remember to call `refresh_instances()` after any mutation
3. Implement it in `src/dao/memory_impl.rs` (for tests)
4. Add unit tests covering the new path

### New Provider Template

1. Add a builder function in `src/templates.rs` (follow the `minimax_template()` / `kimi_template()` pattern)
2. Register it in `register_templates()`
3. Provide `default_env`, `models: Vec<ModelTemplate>`, and the `opencode_*` fields (set empty `opencode_provider_id` to skip opencode config generation for that template)

### New Trace Enrichment

1. Add new columns to `TraceSession` / `TraceRecord` in `src/trace/models.rs`
2. Add corresponding `ALTER TABLE` migrations in `src/trace/store.rs::init_schema` (use the `duplicate column` error-suppression pattern)
3. Update the relevant read functions (`list_sessions`, `get_records`) and `row_to_session` / `row_to_record` mappers
4. If a single transaction is needed, use `complete_session` as the model — call `conn.unchecked_transaction()`
5. Update the TypeScript types in `web/src/api/traces.ts` to match

### New Frontend Component

1. Create the component in `web/src/components/` (shared) or `web/src/pages/` (route-level)
2. Add the route in `web/src/App.tsx` `Routes`; use `useParams` to read `:id` segments
3. Use TanStack Query for server state; add a hook in `web/src/api/hooks.ts` (or a sibling like `traces.ts`) rather than calling `apiGet` directly from a component
4. Add tests in a colocated `*.test.tsx` (Vitest); run with `make test`
5. If the UI is gated by SSE updates, add a `useTraceEvents(enabled)` subscriber

### New Integration Test

1. Create a new file in `tests/` (follow existing `*_test.rs` naming)
2. Use `reqwest` against a running test server, or test the DAO directly with `MemoryDaoImpl`
3. Import via `use cc_switch_tui::*`

### New Shell/Config Generation Feature

1. Add generation logic to `src/shell.rs` (for zsh aliases) or `src/opencode_config.rs` (for opencode JSON)
2. Expose via an API endpoint in `src/api/aliases.rs` or `src/api/opencode.rs`
3. Write generated files into `~/.cc-switch-tui/` with `0o600` permissions when the file may contain an API key
4. Call `shell_escape` on any value interpolated into zsh to avoid injection

### New Proxy Behavior

1. Modify `src/proxy/handler.rs::proxy_handler` (or split it if a branch grows past ~50 lines)
2. Add the new upstream path to the `ALLOWED_PATHS` whitelist constant at the top of the file
3. Add helper parsers in `src/proxy/parser.rs`; SSE event handlers in the same file under `apply_streaming_event`
4. If the new behavior should be observable in trace records, add fields to `TraceRecord` and migrate the schema in `init_schema`
