# cc-switch-tui 架构文档

## Pattern Overview

Use a **Layered Architecture** with a Rust backend (Axum) serving an embedded React SPA frontend.

Key characteristics:

1. **Single-binary deployment**: The Rust binary embeds the compiled frontend via `include_dir`, eliminating separate frontend hosting.
2. **Local-first persistence**: SQLite database stored in `~/.cc-switch-tui/db.sqlite` with zero external infrastructure.
3. **Shell integration as core feature**: The system generates zsh alias files and opencode config files rather than providing a daemon.
4. **Template-instance pattern**: Built-in `ProviderTemplate` definitions are immutable; users create `ProviderInstance` records that reference templates and override specific fields.
5. **Port agility**: The server binds to a cached port (default 7480) with automatic fallback scanning to avoid conflicts.

## Layers

### 1. Frontend (Presentation Layer)

- **Location**: `web/`
- **Technology**: React 18, Vite, Tailwind CSS, React Router, TanStack Query
- **Responsibilities**:
  - Render the TUI (web-based UI) for managing provider instances
  - Communicate with backend via REST API (`/api/*`)
  - Handle client-side routing as an SPA
- **Build output**: Compiled to `web-dist/` and embedded into the Rust binary via `include_dir`
- **Consumed by**: End users via browser (auto-opened on startup)

### 2. API Layer (Transport / Handler Layer)

- **Location**: `src/api/`
- **Technology**: Axum 0.7, Tower, Tokio
- **Responsibilities**:
  - Define HTTP routes and route them to handler functions
  - Extract and validate request parameters
  - Convert domain errors into HTTP responses
  - Serve embedded static files via SPA fallback
- **Key files**:
  - `src/api/mod.rs` — Router construction with all `/api/*` routes
  - `src/api/instances.rs` — CRUD handlers for provider instances
  - `src/api/error.rs` — `ApiError` enum with `IntoResponse` for uniform JSON error responses
  - `src/api/state.rs` — `AppState` shared across all handlers (Arc<Mutex<Dao>> + Arc<RwLock<Settings>>)
  - `src/api/static_fallback.rs` — SPA fallback serving embedded `web-dist/` files
- **Depends on**: Domain layer, DAO layer
- **Used by**: External HTTP clients (browser, curl, tests)

### 3. Domain Layer (Business Logic Layer)

- **Location**: `src/domain/`
- **Responsibilities**:
  - Define core data structures and validation rules
  - Contain pure business logic with no I/O dependencies
  - Define the unified error type `AppError`
- **Key files**:
  - `src/domain/instance.rs` — `ProviderInstance` struct and `validate_alias()` function
  - `src/domain/template.rs` — `ProviderTemplate` and `ModelTemplate` structs
  - `src/domain/error.rs` — `AppError` enum for all domain-level errors
- **Depends on**: None (pure Rust + chrono + thiserror)
- **Used by**: API layer, DAO layer, Shell layer

### 4. DAO Layer (Data Access Layer)

- **Location**: `src/dao/`
- **Responsibilities**:
  - Abstract storage operations behind the `Dao` trait
  - Persist instances to SQLite with schema migration support
  - Cache instances in memory for fast reads
- **Key files**:
  - `src/dao/mod.rs` — `Dao` trait defining the contract
  - `src/dao/sqlite_impl.rs` — `SqliteDaoImpl` with full SQLite CRUD and column migration
  - `src/dao/memory_impl.rs` — In-memory implementation for testing
- **Depends on**: Domain layer
- **Used by**: API layer (via `AppState`)

### 5. Infrastructure / Shell Layer

- **Location**: `src/shell.rs`, `src/opencode_config.rs`, `src/port.rs`, `src/templates.rs`
- **Responsibilities**:
  - Generate zsh alias files (`aliases.zsh`)
  - Generate opencode JSON configuration files
  - Manage TCP port binding and graceful shutdown
  - Register built-in provider templates
- **Key files**:
  - `src/shell.rs` — `render_aliases()`, `generate_aliases()`, `shell_escape()`, zshrc integration
  - `src/opencode_config.rs` — `render_opencode_config()`, `write_opencode_config()`, `build_opencode_aliases()`
  - `src/port.rs` — `try_bind()`, port file caching, `wait_for_shutdown()` with Ctrl-C/SIGTERM handling
  - `src/templates.rs` — `register_templates()` returning built-in MiniMax and Kimi templates
- **Depends on**: Domain layer
- **Used by**: API layer (`/api/aliases`, `/api/opencode-config`), main entry point

## Data Flow

### HTTP Request Flow

```
Browser → Axum Router (src/api/mod.rs)
    → Handler (e.g., src/api/instances.rs::list)
        → Lock AppState.dao (Arc<Mutex<SqliteDaoImpl>>)
            → Call Dao method
                → SQLite query + in-memory cache refresh
                    → Return domain objects
        → Transform to DTO (InstanceSummary / InstanceDetail)
            → JSON response
```

### Alias Generation Flow

```
User triggers alias apply (POST /api/aliases/apply)
    → Handler reads all instances from DAO
        → shell::generate_aliases() builds zsh function definitions
        → opencode_config::generate_opencode_configs() writes JSON configs
        → Write to ~/.cc-switch-tui/aliases.zsh
```

### State Management

- **Server state**: Managed via `tokio::sync::RwLock<Settings>` and `tokio::sync::Mutex<SqliteDaoImpl>` inside `AppState`
- **No client-side state library**: The frontend uses TanStack Query for server state caching; UI state is local React state
- **Persistence**: Only SQLite is durable; `Settings` is in-memory and resets on restart

## Key Abstractions

### Dao Trait

Located in `src/dao/mod.rs`. The `Dao` trait abstracts all storage operations:

```rust
pub trait Dao {
    fn get_templates(&self) -> Vec<&ProviderTemplate>;
    fn get_template(&self, id: &str) -> Option<&ProviderTemplate>;
    fn list_instances(&self) -> Vec<&ProviderInstance>;
    fn get_instance(&self, id: &str) -> Option<&ProviderInstance>;
    fn create_instance(&mut self, instance: ProviderInstance) -> Result<(), AppError>;
    fn delete_instance(&mut self, id: &str) -> Result<(), AppError>;
    fn update_instance(&mut self, id: &str, model_id: String, alias: String, api_key: String) -> Result<(), AppError>;
    // ... additional mutation methods
}
```

Use this trait when adding new storage backends (e.g., a file-based DAO).

### AppState

Located in `src/api/state.rs`. Shared application state injected into all Axum handlers:

```rust
#[derive(Clone)]
pub struct AppState {
    pub dao: Arc<Mutex<SqliteDaoImpl>>,
    pub settings: Arc<RwLock<Settings>>,
}
```

Clone `AppState` freely — it is reference-counted internally.

### ApiError

Located in `src/api/error.rs`. All handlers return `Result<T, ApiError>`, which implements `IntoResponse`:

```rust
pub enum ApiError {
    NotFound(String),
    Validation { field: String, message: String },
    Conflict { field: String, message: String },
    Internal(String),
}
```

Use `ApiError::validation()`, `ApiError::not_found()`, `ApiError::conflict()`, or `ApiError::internal()` to construct errors.

### ProviderTemplate / ProviderInstance

Located in `src/domain/template.rs` and `src/domain/instance.rs`.

- `ProviderTemplate`: Immutable built-in definition of a third-party provider (MiniMax, Kimi) including default env vars, supported models, and opencode metadata.
- `ProviderInstance`: User-created record referencing a template, with user-specific overrides (API key, alias, model selection, feature toggles).

The ID format for instances is `{template_id}-{alias}`. Changing the model does not change the ID.

## Entry Points

### Main Binary

- **File**: `src/main.rs`
- **Trigger**: `cargo run` or executing the compiled binary `cc-switch-tui`
- **Responsibilities**:
  1. Initialize `tracing` logger to `app.log`
  2. Load templates via `register_templates()`
  3. Initialize `SqliteDaoImpl` with `~/.cc-switch-tui/db.sqlite`
  4. Construct `AppState`
  5. Determine port (cached or default 7480, with fallback scanning)
  6. Optionally auto-open browser
  7. Start Axum server with graceful shutdown

### Library Entry

- **File**: `src/lib.rs`
- **Purpose**: Re-exports all public modules for integration tests and external consumers
- **Re-exports**: `api`, `dao`, `domain`, `opencode_config`, `opencode_fetch`, `port`, `shell`, `templates`

### Frontend Entry

- **File**: `web/src/main.tsx`
- **Build command**: `cd web && npm run build` (outputs to `web-dist/`)
- **Note**: The backend must be rebuilt after frontend changes to pick up new embedded assets

## Error Handling

### Domain Errors (`AppError`)

Use `thiserror`-derived `AppError` in `src/domain/error.rs` for all domain-level failures:

- `InstanceAlreadyExists(String)` — primary key conflict
- `InstanceNotFound(String)` — missing resource
- `InvalidAlias(String)` — validation failure
- `Database(String)` — SQLite error wrapper

### API Errors (`ApiError`)

Map domain errors to `ApiError` in handlers, then rely on `IntoResponse` for uniform JSON:

```rust
// Example mapping pattern from src/api/instances.rs
dao.create_instance(new_instance).map_err(|e| match e {
    AppError::InstanceAlreadyExists(_) => ApiError::conflict("alias", "alias already exists"),
    AppError::InvalidAlias(msg) => ApiError::validation("alias", msg),
    e => ApiError::internal(e.to_string()),
})?;
```

HTTP status codes:
- `400` — Validation errors
- `404` — Not found
- `409` — Conflict (duplicate alias)
- `500` — Internal errors

### Logging

Use `tracing` (not `println`). Log file is `app.log` in the working directory. Set `RUST_LOG` environment variable to control verbosity.

## Cross-Cutting Concerns

### Logging

- **Implementation**: `tracing` + `tracing-subscriber` with `fmt` and `env-filter`
- **Configuration**: Environment variable `RUST_LOG` (defaults to `INFO`)
- **Output**: `app.log` (append mode, no ANSI)
- **Pattern**: Use `tracing::info!`, `tracing::warn!`, `tracing::error!` throughout handlers and infrastructure

### Database Schema Migration

- **Implementation**: `SqliteDaoImpl::new()` in `src/dao/sqlite_impl.rs`
- **Pattern**: `CREATE TABLE IF NOT EXISTS` on init, followed by `PRAGMA table_info` checks to add missing columns (alias, opencode_model_id, kv_cache_enabled, context_window_enabled)
- **No external migration tool**: Schema evolution is handled imperatively in code

### Static Asset Embedding

- **Implementation**: `include_dir` crate in `src/api/static_fallback.rs`
- **Pattern**: The `web-dist/` directory is embedded at compile time via `include_dir!("$CARGO_MANIFEST_DIR/web-dist")`
- **SPA routing**: Any non-API path not matching a static file falls back to `index.html`

### Port Management

- **Implementation**: `src/port.rs`
- **Pattern**: Read cached port from `~/.cc-switch-tui/port`; if unavailable or occupied, scan `start_port..start_port+100` for an available port; write the actual port back to the file; clean up on graceful shutdown

### Input Validation

- **Alias validation**: `validate_alias()` in `src/domain/instance.rs` enforces lowercase alphanumeric + `-`/`_`, length 1-32
- **No external validation framework**: Validation is manual function calls in handlers

## Where to Add New Code

### New API Endpoint

1. Add handler function in the appropriate `src/api/*.rs` module (or create a new module)
2. Wire the route in `src/api/mod.rs::router()`
3. Add request/response DTOs alongside the handler (follow `instances.rs` pattern)
4. Map domain errors to `ApiError` in the handler

### New Domain Model or Validation

1. Add the struct/enum to the appropriate file in `src/domain/`
2. Add any validation logic as pure functions in the same file
3. Add new error variants to `src/domain/error.rs` if needed
4. Write unit tests in the same module under `#[cfg(test)]`

### New Storage Operation

1. Add the method signature to the `Dao` trait in `src/dao/mod.rs`
2. Implement it in `src/dao/sqlite_impl.rs`
3. Implement it in `src/dao/memory_impl.rs` (for tests)
4. Add unit tests in `src/dao/sqlite_impl.rs` under `#[cfg(test)]`

### New Provider Template

1. Add a builder function in `src/templates.rs` (follow `minimax_template()` / `kimi_template()` pattern)
2. Register it in `register_templates()`
3. Update any affected tests

### New Frontend Component

1. Create the component in `web/src/components/`
2. Add it to the appropriate page/route in `web/src/App.tsx`
3. Use TanStack Query for server state, local `useState` for UI state
4. Add tests in `web/src/App.test.tsx` or a new `*.test.tsx` file

### New Integration Test

1. Create a new file in `tests/` (follow existing `*_test.rs` naming)
2. Use `reqwest` to hit the running server, or test DAO directly
3. Import the crate via `use cc_switch_tui::*`

### New Shell/Config Generation Feature

1. Add generation logic to `src/shell.rs` (for zsh aliases) or `src/opencode_config.rs` (for opencode JSON)
2. Expose via an API endpoint in `src/api/aliases.rs` or `src/api/opencode.rs`
3. Ensure generated files are written to `~/.cc-switch-tui/` with correct permissions (600 for configs containing API keys)
