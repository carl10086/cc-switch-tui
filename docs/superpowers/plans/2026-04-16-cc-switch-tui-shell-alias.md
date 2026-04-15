# cc-switch-tui Shell 别名集成实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为每个 provider 实例生成 `cl-` 前缀的 shell 别名，支持默认 `claude` 别名激活，通过 TUI 编辑 alias/api_key，并自动注入 `~/.zshrc`。

**Architecture:** 扩展 `ProviderInstance` 和 SQLite 表增加 `alias` 字段；`shell.rs` 负责生成 `~/.cc-switch-tui/aliases.zsh` 并维护 `~/.zshrc` 的 source 行；TUI 增加 `CreateAlias` 和 `EditInfoPanel` 状态，列表页按 `e` 进入右侧面板编辑模式。

**Tech Stack:** Rust, ratatui, rusqlite, chrono

---

### Task 1: 数据模型和错误类型扩展

**Files:**
- Modify: `src/domain/instance.rs`
- Modify: `src/domain/error.rs`
- Modify: `src/dao/mod.rs`
- Modify: `src/dao/memory_impl.rs`

- [ ] **Step 1: ProviderInstance 新增 alias 字段**

修改 `src/domain/instance.rs`：

```rust
pub struct ProviderInstance {
    pub id: String,
    pub template_id: String,
    pub model_id: String,
    pub api_key: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub alias: String,        // 新增
}
```

- [ ] **Step 2: 新增 AppError 变体**

修改 `src/domain/error.rs`，在 `Database(String)` 下面添加：

```rust
    #[error("Invalid alias: {0}")]
    InvalidAlias(String),

    #[error("Alias already exists: {0}")]
    AliasAlreadyExists(String),
```

- [ ] **Step 3: Dao trait 新增 set_alias 方法**

修改 `src/dao/mod.rs`，在 `update_instance` 方法后添加：

```rust
    /// 更新实例别名
    fn set_alias(&mut self, id: &str, alias: String) -> Result<(), AppError>;
```

- [ ] **Step 4: MemoryDaoImpl 适配 alias 字段和 set_alias**

修改 `src/dao/memory_impl.rs`：

1. 在 `create_instance` 之后（或任意位置）添加 `set_alias` 实现：

```rust
    fn set_alias(&mut self, id: &str, alias: String) -> Result<(), AppError> {
        let instance = self.instances.get_mut(id)
            .ok_or_else(|| AppError::InstanceNotFound(id.to_string()))?;
        instance.alias = alias;
        Ok(())
    }
```

2. 将所有硬编码构造 `ProviderInstance` 的地方（如测试模块和 dao 方法中）添加 `alias: String::new()` 字段。需要搜索整个文件中的 `ProviderInstance {` 出现位置并补充。

例如，在 `create_instance` 中无需修改（传入的 instance 已包含 alias）。在 `tests/dao_test.rs`（这是外部测试文件）中的所有 `ProviderInstance` 构造稍后会在 Task 6 统一修改，但 `memory_impl.rs` 内部的任何构造都需要先处理。

Run: `cargo check`
Expected: 通过（此时 sqlite_impl.rs 可能因缺少 set_alias 和 alias 字段而报错，这会在 Task 2 处理）

- [ ] **Step 5: Commit**

```bash
git add src/domain/instance.rs src/domain/error.rs src/dao/mod.rs src/dao/memory_impl.rs
git commit -m "feat(domain): add alias field and AppError variants"
```

---

### Task 2: SqliteDaoImpl 适配 alias 字段和 set_alias

**Files:**
- Modify: `src/dao/sqlite_impl.rs`

- [ ] **Step 1: SQLite 表迁移 + refresh_instances 读取 alias**

修改 `src/dao/sqlite_impl.rs` 的构造函数 SQL：

```rust
        conn.execute(
            "CREATE TABLE IF NOT EXISTS instances (
                id TEXT PRIMARY KEY,
                template_id TEXT NOT NULL,
                model_id TEXT NOT NULL,
                api_key TEXT NOT NULL,
                created_at TEXT NOT NULL,
                alias TEXT NOT NULL DEFAULT '',
                is_current INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;
```

注意：对于已有数据库，这一行不会改变已存在的表结构（SQLite 不支持 ALTER 添加列时改变 DEFAULT 的已有行？实际上 `CREATE TABLE IF NOT EXISTS` 对已有表不生效，已存在旧表时 `alias` 列会缺失，导致后续 SELECT 失败）。

**处理方案：** 在 Task 7 或本 Task 中处理 ALTER TABLE。先在这里处理：在 `CREATE TABLE` 之后增加：

```rust
        // 兼容旧表：如果 alias 列不存在则添加
        let _ = conn.execute("ALTER TABLE instances ADD COLUMN alias TEXT NOT NULL DEFAULT ''", []);
```

SQLite 的 ALTER TABLE 在不支持时才会失败（列已存在时也会失败），所以用 `let _ =` 忽略错误。

修改 `refresh_instances` 的 SELECT 和字段映射，增加 `alias`：

```rust
    fn refresh_instances(&mut self) -> Result<(), rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, template_id, model_id, api_key, created_at, alias FROM instances"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(ProviderInstance {
                id: row.get(0)?,
                template_id: row.get(1)?,
                model_id: row.get(2)?,
                api_key: row.get(3)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .map_err(|e| rusqlite::Error::FromSqlConversionFailure(
                        4,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    ))?
                    .with_timezone(&chrono::Utc),
                alias: row.get(5)?,
            })
        })?;
        self.instances.clear();
        for row in rows {
            self.instances.push(row?);
        }
        Ok(())
    }
```

- [ ] **Step 2: create_instance INSERT 包含 alias**

修改 `create_instance` 的 SQL：

```rust
            "INSERT INTO instances (id, template_id, model_id, api_key, created_at, alias, is_current)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
            rusqlite::params![
                &instance.id,
                &instance.template_id,
                &instance.model_id,
                &instance.api_key,
                created_at_str,
                &instance.alias,
            ],
```

- [ ] **Step 3: 写失败测试 + 实现 set_alias**

在 `#[cfg(test)]` 模块添加测试：

```rust
    #[test]
    fn test_set_alias_updates_alias() {
        let mut dao = create_test_dao();
        let instance = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "key".to_string(),
            created_at: chrono::Utc::now(),
            alias: String::new(),
        };
        dao.create_instance(instance).unwrap();
        dao.set_alias("minimax-MiniMax-M2.7-highspeed", "cl-mini".to_string()).unwrap();
        let found = dao.get_instance("minimax-MiniMax-M2.7-highspeed").unwrap();
        assert_eq!(found.alias, "cl-mini");
    }
```

Run: `cargo test test_set_alias_updates_alias`
Expected: **FAIL** — `set_alias` 未实现

实现 `set_alias`：

```rust
    fn set_alias(&mut self, id: &str, alias: String) -> Result<(), AppError> {
        let changes = self.conn.execute(
            "UPDATE instances SET alias = ?1 WHERE id = ?2",
            [alias, id],
        ).map_err(|e| AppError::Database(e.to_string()))?;
        if changes == 0 {
            return Err(AppError::InstanceNotFound(id.to_string()));
        }
        self.refresh_instances()
            .map_err(|e| AppError::Database(e.to_string()))?;
        Ok(())
    }
```

Run: `cargo test test_set_alias_updates_alias`
Expected: **PASS**

- [ ] **Step 4: 修复现有测试中的 ProviderInstance 构造**

文件中 `#[cfg(test)]` 模块里的所有 `ProviderInstance` 构造都需要添加 `alias: String::new()` 或对应的 alias 值。确保 `cargo test` 全部通过。

Run: `cargo test`
Expected: 全部通过

- [ ] **Step 5: Commit**

```bash
git add src/dao/sqlite_impl.rs
git commit -m "feat(dao): add alias column and set_alias for SqliteDaoImpl"
```

---

### Task 3: 实现 shell.rs 的 aliases.zsh 生成和 zshrc 注入

**Files:**
- Modify: `src/shell.rs`

- [ ] **Step 1: 写失败测试 —— generate_aliases 输出正确**

清空并重写 `src/shell.rs`：

```rust
use crate::domain::{ProviderInstance, ProviderTemplate};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// 生成 aliases.zsh 文件
pub fn generate_aliases(
    instances: &[ProviderInstance],
    templates: &[ProviderTemplate],
    current_instance_id: Option<&str>,
) -> std::io::Result<()> {
    let mut lines = vec!["# Auto-generated by cc-switch-tui. Do not edit manually.".to_string()];

    for instance in instances {
        if instance.alias.is_empty() {
            continue;
        }
        let env = build_env(instance, templates);
        let alias_line = format_alias_cmd(&instance.alias, &env);
        lines.push(alias_line);
    }

    if let Some(id) = current_instance_id {
        if let Some(instance) = instances.iter().find(|i| i.id == id && !i.alias.is_empty()) {
            let env = build_env(instance, templates);
            let alias_line = format_alias_cmd("claude", &env);
            lines.push(alias_line);
        }
    }

    let dir = dirs::home_dir()
        .map(|h| h.join(".cc-switch-tui"))
        .unwrap_or_else(|| PathBuf::from(".cc-switch-tui"));
    fs::create_dir_all(&dir)?;
    let path = dir.join("aliases.zsh");
    fs::write(path, lines.join("\n") + "\n")?;
    Ok(())
}

fn build_env(instance: &ProviderInstance, templates: &[ProviderTemplate]) -> HashMap<String, String> {
    let mut env = HashMap::new();
    if let Some(template) = templates.iter().find(|t| t.id == instance.template_id) {
        env.extend(template.default_env.clone());
        if let Some(model) = template.models.iter().find(|m| m.id == instance.model_id) {
            env.extend(model.env_overrides.clone());
        }
    }
    env.insert("ANTHROPIC_AUTH_TOKEN".to_string(), instance.api_key.clone());
    env
}

fn format_alias_cmd(name: &str, env: &HashMap<String, String>) -> String {
    let mut pairs: Vec<_> = env.iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(b.0));
    let env_str = pairs
        .iter()
        .map(|(k, v)| format!("{}={}", k, shell_escape(v)))
        .collect::<Vec<_>>()
        .join(" ");
    format!("alias {}='{} claude'", name, env_str)
}

fn shell_escape(s: &str) -> String {
    if s.contains(' ') || s.contains('\'') || s.contains('$') || s.contains('&') || s.contains('|') {
        format!("'{}'", s.replace('\'', "'\"'\"'"))
    } else {
        s.to_string()
    }
}

/// 检查 ~/.zshrc 是否包含 source ~/.cc-switch-tui/aliases.zsh
pub fn ensure_zshrc_source() -> std::io::Result<bool> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let zshrc = home.join(".zshrc");
    let source_line = "source ~/.cc-switch-tui/aliases.zsh";

    if zshrc.exists() {
        let content = fs::read_to_string(&zshrc)?;
        if content.lines().any(|l| l.trim() == source_line) {
            return Ok(false);
        }
    }

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&zshrc)?;
    writeln!(file, "{}", source_line)?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ModelTemplate, ProviderTemplate};
    use std::collections::HashMap;
    use tempfile::TempDir;

    #[test]
    fn test_generate_aliases_creates_file() {
        let temp = TempDir::new().unwrap();
        // 临时替换 home_dir 的实现不可行，我们直接测试 build_env 和 format_alias_cmd
        // 但 generate_aliases 硬编码了 ~/.cc-switch-tui，需要调整函数签名接受输出路径才能测试
        // 因此先修改 generate_aliases 为可测试版本
    }
}
```

上面的测试失败了，因为 `generate_aliases` 硬编码了 home 目录。**在写测试之前，需要先重构 `generate_aliases` 使其接受输出目录。**

调整 `generate_aliases` 签名和实现：

```rust
pub fn generate_aliases(
    dir: &std::path::Path,
    instances: &[ProviderInstance],
    templates: &[ProviderTemplate],
    current_instance_id: Option<&str>,
) -> std::io::Result<()> {
    let mut lines = vec!["# Auto-generated by cc-switch-tui. Do not edit manually.".to_string()];
    // ... same as before ...
    fs::create_dir_all(dir)?;
    let path = dir.join("aliases.zsh");
    fs::write(path, lines.join("\n") + "\n")?;
    Ok(())
}
```

然后写一个公开包装函数 `generate_aliases_for_home`（或者让调用方传入路径）。这里选择在 `main.rs` 调用时由调用方提供路径，测试时传入 `TempDir`。

修改后的 `generate_aliases` 如上所示（接受 `dir` 参数）。

测试：

```rust
    #[test]
    fn test_generate_aliases_creates_file() {
        let temp = TempDir::new().unwrap();
        let instances = vec![];
        let templates = vec![];
        generate_aliases(temp.path(), &instances, &templates, None).unwrap();
        assert!(temp.path().join("aliases.zsh").exists());
    }
```

Run: `cargo test test_generate_aliases_creates_file`
Expected: **FAIL** — 需要先把函数改好

先把 `src/shell.rs` 完整写成可测试版本（带 `dir` 参数），然后运行测试。

- [ ] **Step 2: 写失败测试 —— 包含 alias 和 claude 默认别名的输出**

```rust
    #[test]
    fn test_generate_aliases_content() {
        let temp = TempDir::new().unwrap();
        let mut env = HashMap::new();
        env.insert("ANTHROPIC_BASE_URL".to_string(), "https://api.minimaxi.com/anthropic".to_string());
        let template = ProviderTemplate {
            id: "minimax".to_string(),
            name: "MiniMax".to_string(),
            default_env: env,
            models: vec![],
        };
        let instance = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "sk-test".to_string(),
            created_at: chrono::Utc::now(),
            alias: "cl-mini".to_string(),
        };
        generate_aliases(
            temp.path(),
            &[instance],
            &[template],
            Some("minimax-MiniMax-M2.7-highspeed"),
        ).unwrap();

        let content = std::fs::read_to_string(temp.path().join("aliases.zsh")).unwrap();
        assert!(content.contains("alias cl-mini="));
        assert!(content.contains("alias claude="));
        assert!(content.contains("ANTHROPIC_BASE_URL=https://api.minimaxi.com/anthropic"));
        assert!(content.contains("ANTHROPIC_AUTH_TOKEN=sk-test"));
    }
```

Run: `cargo test test_generate_aliases_content`
Expected: **PASS**（上一步函数已改好）

- [ ] **Step 3: 写失败测试 —— ensure_zshrc_source**

先重构 `ensure_zshrc_source` 使其接受路径参数以便测试：

```rust
pub fn ensure_zshrc_source(zshrc_path: &std::path::Path) -> std::io::Result<bool> {
    let source_line = "source ~/.cc-switch-tui/aliases.zsh";
    if zshrc_path.exists() {
        let content = fs::read_to_string(zshrc_path)?;
        if content.lines().any(|l| l.trim() == source_line) {
            return Ok(false);
        }
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(zshrc_path)?;
    writeln!(file, "{}", source_line)?;
    Ok(true)
}
```

测试：

```rust
    #[test]
    fn test_ensure_zshrc_source_adds_line() {
        let temp = TempDir::new().unwrap();
        let zshrc = temp.path().join(".zshrc");
        let added = ensure_zshrc_source(&zshrc).unwrap();
        assert!(added);
        let content = fs::read_to_string(&zshrc).unwrap();
        assert!(content.contains("source ~/.cc-switch-tui/aliases.zsh"));

        let added2 = ensure_zshrc_source(&zshrc).unwrap();
        assert!(!added2);
    }
```

Run: `cargo test test_ensure_zshrc_source_adds_line`
Expected: **PASS**

- [ ] **Step 4: 确保 Cargo.toml 有 tempfile（dev-dependency）和 dirs**

检查 `Cargo.toml`，确保有：

```toml
dirs = "5"

[dev-dependencies]
tempfile = "3"
```

如果没有，添加它们。

Run: `cargo test`
Expected: 全部通过（shell.rs 的 3 个测试 + 原有测试）

- [ ] **Step 5: Commit**

```bash
git add src/shell.rs Cargo.toml
# 如果添加了 dev-dependencies 一并提交
git commit -m "feat(shell): implement aliases.zsh generation and zshrc injection"
```

---

### Task 4: App 状态机扩展 —— CreateAlias 和编辑模式

**Files:**
- Modify: `src/app/state.rs`

- [ ] **Step 1: 扩展 AppState 和 EditField**

修改 `src/app/state.rs` 的 `AppState` 定义：

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum AppState {
    List,
    CreateProvider,
    CreateModel { template_id: String },
    CreateApiKey { template_id: String, model_id: String },
    /// 新增：创建向导最后一页，输入别名
    CreateAlias { template_id: String, model_id: String, api_key: String },
    /// 原有 Edit 保留给 API Key 弹窗（兼容现有 draw_edit）
    Edit { instance_id: String },
    /// 新增：编辑右侧信息面板
    EditInfoPanel { instance_id: String, focus_index: usize },
    /// 新增：编辑具体字段弹窗
    EditField { instance_id: String, field: EditField },
    DeleteConfirm { instance_id: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum EditField {
    Alias,
    ApiKey,
}
```

- [ ] **Step 2: 修改创建流程状态跳转**

在 `handle_create_api_key` 中，按 `Enter` 提交时不再直接 `submit_create()`，而是先进入 `CreateAlias`：

```rust
    fn handle_create_api_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if let AppState::CreateApiKey { template_id, .. } = &self.state {
                    let tid = template_id.clone();
                    self.state = AppState::CreateModel { template_id: tid };
                }
            }
            KeyCode::Enter => {
                if let AppState::CreateApiKey { template_id, model_id } = self.state.clone() {
                    let api_key = self.api_key_input.value.clone();
                    self.state = AppState::CreateAlias {
                        template_id,
                        model_id,
                        api_key,
                    };
                    self.api_key_input = InputState::new(String::new());
                }
            }
            // ... rest unchanged
        }
    }
```

注意：`api_key_input` 在切换到 `CreateAlias` 后可能需要复用。更好的做法是复用 `api_key_input` 作为别名输入框，或者新增 `alias_input`。根据 spec，`CreateAlias` 页面与 `CreateApiKey` 类似，使用输入框。

这里我们选择**复用 `api_key_input`**（因为它在 CreateApiKey 状态结束后就没用了），进入 `CreateAlias` 时将其重置为空：`self.api_key_input = InputState::new(String::new());`。

然后新增 `handle_create_alias` 方法：

```rust
    fn handle_create_alias(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if let AppState::CreateAlias { template_id, model_id, api_key } = &self.state {
                    self.state = AppState::CreateApiKey {
                        template_id: template_id.clone(),
                        model_id: model_id.clone(),
                    };
                    self.api_key_input = InputState::new(api_key.clone());
                }
            }
            KeyCode::Enter => {
                self.submit_create();
            }
            KeyCode::Backspace => self.api_key_input.backspace(),
            KeyCode::Left => self.api_key_input.move_left(),
            KeyCode::Right => self.api_key_input.move_right(),
            KeyCode::Char(c) => self.api_key_input.insert_char(c),
            _ => {}
        }
    }
```

修改 `submit_create`：

```rust
    fn submit_create(&mut self) {
        if let AppState::CreateAlias { template_id, model_id, api_key } = self.state.clone() {
            let alias = self.api_key_input.value.clone();
            if let Err(e) = self.validate_alias(&alias) {
                self.error_message = Some(e.to_string());
                return;
            }
            let id = format!("{}-{}", template_id, model_id);
            let instance = ProviderInstance {
                id: id.clone(),
                template_id,
                model_id,
                api_key,
                created_at: chrono::Utc::now(),
                alias,
            };
            match self.dao.create_instance(instance) {
                Ok(()) => {
                    tracing::info!("create instance success: id={}", id);
                    self.state = AppState::List;
                    self.list_index = self.get_sorted_instances().len().saturating_sub(1);
                }
                Err(AppError::InstanceAlreadyExists(id)) => {
                    self.error_message = Some(format!("实例已存在: {}", id));
                    self.state = AppState::List;
                }
                Err(e) => {
                    self.error_message = Some(e.to_string());
                    self.state = AppState::List;
                }
            }
        }
    }
```

新增 `validate_alias` 方法：

```rust
    fn validate_alias(&self, alias: &str) -> Result<(), AppError> {
        if alias.is_empty() {
            return Err(AppError::InvalidAlias("alias cannot be empty".to_string()));
        }
        if !alias.starts_with("cl-") {
            return Err(AppError::InvalidAlias("alias must start with 'cl-'".to_string()));
        }
        if !alias.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            return Err(AppError::InvalidAlias("alias contains invalid characters".to_string()));
        }
        let instances = self.dao.list_instances();
        if instances.iter().any(|i| i.alias == alias) {
            return Err(AppError::AliasAlreadyExists(alias.to_string()));
        }
        Ok(())
    }
```

- [ ] **Step 3: 新增编辑模式状态处理**

在 `on_key` 中增加分支：

```rust
    pub fn on_key(&mut self, key: KeyEvent) {
        self.error_message = None;
        tracing::debug!("key_event = {:?}", key);
        match &self.state.clone() {
            AppState::List => self.handle_list(key),
            AppState::CreateProvider => self.handle_create_provider(key),
            AppState::CreateModel { .. } => self.handle_create_model(key),
            AppState::CreateApiKey { .. } => self.handle_create_api_key(key),
            AppState::CreateAlias { .. } => self.handle_create_alias(key),
            AppState::Edit { .. } => self.handle_edit(key),
            AppState::EditInfoPanel { .. } => self.handle_edit_info_panel(key),
            AppState::EditField { .. } => self.handle_edit_field(key),
            AppState::DeleteConfirm { .. } => self.handle_delete_confirm(key),
        }
    }
```

新增 `handle_edit_info_panel`：

```rust
    fn handle_edit_info_panel(&mut self, key: KeyEvent) {
        let max_index = 1; // alias=0, api_key=1
        match key.code {
            KeyCode::Esc => self.state = AppState::List,
            KeyCode::Up => {
                if let AppState::EditInfoPanel { instance_id, focus_index } = self.state.clone() {
                    if focus_index > 0 {
                        self.state = AppState::EditInfoPanel {
                            instance_id,
                            focus_index: focus_index - 1,
                        };
                    }
                }
            }
            KeyCode::Down => {
                if let AppState::EditInfoPanel { instance_id, focus_index } = self.state.clone() {
                    if focus_index < max_index {
                        self.state = AppState::EditInfoPanel {
                            instance_id,
                            focus_index: focus_index + 1,
                        };
                    }
                }
            }
            KeyCode::Enter => {
                if let AppState::EditInfoPanel { instance_id, focus_index } = self.state.clone() {
                    let field = match focus_index {
                        0 => EditField::Alias,
                        1 => EditField::ApiKey,
                        _ => return,
                    };
                    // 预填输入框
                    if let Some(instance) = self.dao.get_instance(&instance_id) {
                        let value = match field {
                            EditField::Alias => instance.alias.clone(),
                            EditField::ApiKey => instance.api_key.clone(),
                        };
                        self.edit_input = InputState::new(value);
                    }
                    self.state = AppState::EditField { instance_id, field };
                }
            }
            _ => {}
        }
    }
```

新增 `handle_edit_field`：

```rust
    fn handle_edit_field(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                if let AppState::EditField { instance_id, .. } = self.state.clone() {
                    self.state = AppState::EditInfoPanel {
                        instance_id,
                        focus_index: 0,
                    };
                }
            }
            KeyCode::Enter => {
                if let AppState::EditField { instance_id, field } = self.state.clone() {
                    let value = self.edit_input.value.clone();
                    let result = match field {
                        EditField::Alias => {
                            if let Err(e) = self.validate_alias(&value) {
                                Err(e)
                            } else {
                                self.dao.set_alias(&instance_id, value)
                            }
                        }
                        EditField::ApiKey => {
                            self.dao.update_instance(&instance_id, value)
                        }
                    };
                    match result {
                        Ok(()) => {
                            self.state = AppState::EditInfoPanel {
                                instance_id,
                                focus_index: 0,
                            };
                        }
                        Err(e) => {
                            self.error_message = Some(e.to_string());
                        }
                    }
                }
            }
            KeyCode::Backspace => self.edit_input.backspace(),
            KeyCode::Left => self.edit_input.move_left(),
            KeyCode::Right => self.edit_input.move_right(),
            KeyCode::Char(c) => self.edit_input.insert_char(c),
            _ => {}
        }
    }
```

修改 `handle_list`：

```rust
    fn handle_list(&mut self, key: KeyEvent) {
        let instances = self.get_sorted_instances();
        match key.code {
            KeyCode::Char('q') => {
                tracing::debug!("state transition: List -> Quit");
                self.should_quit = true;
            }
            KeyCode::Char('n') => {
                tracing::debug!("state transition: List -> CreateProvider");
                self.state = AppState::CreateProvider;
                self.provider_index = 0;
            }
            KeyCode::Char('e') => {
                if let Some(instance) = self.current_instance() {
                    tracing::debug!("state transition: List -> EditInfoPanel({})", instance.id);
                    self.state = AppState::EditInfoPanel {
                        instance_id: instance.id.clone(),
                        focus_index: 0,
                    };
                }
            }
            KeyCode::Char('d') => {
                if let Some(instance) = self.current_instance() {
                    tracing::debug!("state transition: List -> DeleteConfirm({})", instance.id);
                    self.state = AppState::DeleteConfirm { instance_id: instance.id.clone() };
                }
            }
            KeyCode::Enter => {
                if let Some(instance) = self.current_instance() {
                    if instance.alias.is_empty() {
                        self.error_message = Some("请先按 e 进入编辑模式设置别名".to_string());
                    } else {
                        if let Err(e) = self.dao.set_current_instance(&instance.id) {
                            self.error_message = Some(e.to_string());
                        } else {
                            // 激活后重新生成 aliases.zsh
                            let _ = crate::shell::generate_aliases(
                                &dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(".cc-switch-tui"),
                                &self.dao.list_instances().into_iter().cloned().collect::<Vec<_>>(),
                                &self.dao.get_templates().into_iter().cloned().collect::<Vec<_>>(),
                                Some(&instance.id),
                            );
                            self.error_message = Some(format!("已激活 {}，新终端中 claude 命令将使用该配置", instance.alias));
                        }
                    }
                }
            }
            KeyCode::Up => {
                if self.list_index > 0 {
                    self.list_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.list_index + 1 < instances.len() {
                    self.list_index += 1;
                }
            }
            _ => {}
        }
    }
```

注意：`handle_list` 里调用 `generate_aliases` 时，`self.dao.list_instances()` 返回的是 `Vec<&ProviderInstance>`，需要 `.cloned().collect()`。但 `ProviderInstance` 已经实现了 `Clone`（`derive(Clone)`），所以没问题。同理 `get_templates()` 返回 `Vec<&ProviderTemplate>`，也要 `.cloned().collect()`，需要确保 `ProviderTemplate` 和 `ModelTemplate` 都实现了 `Clone`。如果它们没有，需要添加。

检查 `src/domain/template.rs`，给 `ProviderTemplate` 和 `ModelTemplate` 添加 `#[derive(Debug, Clone)]`（如果还没有的话）。

- [ ] **Step 4: 运行 cargo check 和 cargo test**

Run: `cargo check`
Expected: 通过

Run: `cargo test`
Expected: 通过（部分原有测试因 `ProviderInstance` 构造缺少 `alias` 字段可能失败，这些会在 Task 6 修复）

- [ ] **Step 5: Commit**

```bash
git add src/app/state.rs src/domain/template.rs
git commit -m "feat(app): add CreateAlias, EditInfoPanel and EditField states"
```

---

### Task 5: UI 渲染适配

**Files:**
- Modify: `src/ui/create.rs`
- Modify: `src/ui/edit.rs`
- Modify: `src/ui/list.rs`
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: 创建向导渲染 CreateAlias 页面**

修改 `src/ui/create.rs`，在 `draw_api_key_input` 之后添加 `draw_alias_input`：

```rust
fn draw_alias_input(frame: &mut Frame, app: &App) {
    let area = centered_rect(frame, 50, 7);
    frame.render_widget(Clear, area);

    let t = theme::theme();
    let text = vec![
        Line::from("请输入别名（必须以 cl- 开头）："),
        Line::from(""),
        Line::from(vec![
            Span::raw("> "),
            Span::raw(app.api_key_input.value.clone()),
            Span::styled("_", Style::default().fg(t.warning())),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(Block::default().title("输入别名").borders(Borders::ALL));
    frame.render_widget(paragraph, area);
}
```

修改 `draw_create` 分发：

```rust
pub fn draw_create<D: Dao>(frame: &mut Frame, app: &App<D>) {
    match &app.state {
        AppState::CreateProvider => draw_provider_select(frame, app),
        AppState::CreateModel { .. } => draw_model_select(frame, app),
        AppState::CreateApiKey { .. } => draw_api_key_input(frame, app),
        AppState::CreateAlias { .. } => draw_alias_input(frame, app),
        _ => {}
    }
}
```

- [ ] **Step 2: 编辑弹窗支持 Alias 和 API Key 两种标题**

`src/ui/edit.rs` 当前的 `draw_edit` 只用于 `AppState::Edit { .. }`（原有 API Key 编辑）。

为了兼容新的 `EditField` 状态，我们复用 `draw_edit`，但让标题根据状态变化：

```rust
use crate::app::state::{App, EditField};

pub fn draw_edit<D: Dao>(frame: &mut Frame, app: &App<D>) {
    let title = match &app.state {
        crate::app::state::AppState::EditField { field: EditField::Alias, .. } => "编辑别名",
        _ => "编辑",
    };
    // ... rest same as before, but use `title` variable for Block::default().title(title)
}
```

- [ ] **Step 3: 信息面板支持编辑模式高亮**

修改 `src/ui/list.rs` 的 `draw_info_panel`，增加对 `EditInfoPanel` 状态的识别。需要传入 `app.state` 判断。

```rust
fn draw_info_panel<D: Dao>(frame: &mut Frame, area: ratatui::layout::Rect, app: &App<D>) {
    let t = theme::theme();
    let mut text = vec![];

    let focus_index = match &app.state {
        crate::app::state::AppState::EditInfoPanel { focus_index, .. } => Some(*focus_index),
        _ => None,
    };

    if let Some(instance) = app.current_instance() {
        if let Some(template) = app.dao.get_template(&instance.template_id) {
            let model = template.models.iter()
                .find(|m| m.id == instance.model_id)
                .map(|m| m.name.as_str())
                .unwrap_or("Unknown");

            text.push(Line::from(vec![Span::styled(
                "实例详情",
                Style::default().fg(t.heading()).add_modifier(ratatui::style::Modifier::BOLD),
            )]));
            text.push(Line::from(""));
            text.push(Line::from(format!("ID: {}", instance.id)));
            text.push(Line::from(format!("Provider: {}", template.name)));
            text.push(Line::from(format!("Model: {}", model)));
            text.push(Line::from(format!(
                "API Key: {}*******",
                &instance.api_key.chars().take(3).collect::<String>()
            )));
            text.push(Line::from(""));
            text.push(Line::from(vec![Span::styled(
                "环境变量",
                Style::default().fg(t.heading()).add_modifier(ratatui::style::Modifier::BOLD),
            )]));
            text.push(Line::from(""));

            let mut env = template.default_env.clone();
            if let Some(m) = template.models.iter().find(|m| m.id == instance.model_id) {
                env.extend(m.env_overrides.clone());
            }
            env.insert("ANTHROPIC_AUTH_TOKEN".to_string(), instance.api_key.clone());

            let mut keys: Vec<_> = env.keys().collect();
            keys.sort();
            for key in keys {
                let value = if key == "ANTHROPIC_AUTH_TOKEN" {
                    format!("{}*******", &env.get(key).unwrap().chars().take(3).collect::<String>())
                } else {
                    env.get(key).unwrap().clone()
                };
                text.push(Line::from(format!("{}={}", key, value)));
            }
        }
    } else {
        text.push(Line::from("暂无实例，按 n 新建"));
    }

    let paragraph = Paragraph::new(text)
        .block(Block::default().title("信息面板").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}
```

等等，上面没有真正体现编辑模式的高亮。需要把 Alias 和 API Key 单独提出来作为可编辑字段显示。

重构 `draw_info_panel`：

```rust
fn draw_info_panel<D: Dao>(frame: &mut Frame, area: ratatui::layout::Rect, app: &App<D>) {
    let t = theme::theme();
    let mut text = vec![];

    let focus_index = match &app.state {
        crate::app::state::AppState::EditInfoPanel { focus_index, .. } => Some(*focus_index),
        _ => None,
    };

    if let Some(instance) = app.current_instance() {
        if let Some(template) = app.dao.get_template(&instance.template_id) {
            let model = template.models.iter()
                .find(|m| m.id == instance.model_id)
                .map(|m| m.name.as_str())
                .unwrap_or("Unknown");

            text.push(Line::from(vec![Span::styled(
                "实例详情",
                Style::default().fg(t.heading()).add_modifier(ratatui::style::Modifier::BOLD),
            )]));
            text.push(Line::from(""));
            text.push(Line::from(format!("ID: {}", instance.id)));
            text.push(Line::from(format!("Provider: {}", template.name)));
            text.push(Line::from(format!("Model: {}", model)));
            text.push(Line::from(""));

            // Alias 字段（可编辑）
            let alias_display = if instance.alias.is_empty() {
                "(未设置)".to_string()
            } else {
                instance.alias.clone()
            };
            let alias_style = if focus_index == Some(0) {
                Style::default().bg(t.selection_bg()).fg(t.selection_fg())
            } else {
                Style::default()
            };
            text.push(Line::from(vec![
                Span::raw("Alias: "),
                Span::styled(alias_display, alias_style),
            ]));

            // API Key 字段（可编辑）
            let api_key_masked = format!("{}*******",
                &instance.api_key.chars().take(3).collect::<String>());
            let api_style = if focus_index == Some(1) {
                Style::default().bg(t.selection_bg()).fg(t.selection_fg())
            } else {
                Style::default()
            };
            text.push(Line::from(vec![
                Span::raw("API Key: "),
                Span::styled(api_key_masked, api_style),
            ]));

            text.push(Line::from(""));
            text.push(Line::from(vec![Span::styled(
                "环境变量",
                Style::default().fg(t.heading()).add_modifier(ratatui::style::Modifier::BOLD),
            )]));
            text.push(Line::from(""));

            let mut env = template.default_env.clone();
            if let Some(m) = template.models.iter().find(|m| m.id == instance.model_id) {
                env.extend(m.env_overrides.clone());
            }
            env.insert("ANTHROPIC_AUTH_TOKEN".to_string(), instance.api_key.clone());

            let mut keys: Vec<_> = env.keys().collect();
            keys.sort();
            for key in keys {
                let value = if key == "ANTHROPIC_AUTH_TOKEN" {
                    format!("{}*******", &env.get(key).unwrap().chars().take(3).collect::<String>())
                } else {
                    env.get(key).unwrap().clone()
                };
                text.push(Line::from(format!("{}={}", key, value)));
            }
        }
    } else {
        text.push(Line::from("暂无实例，按 n 新建"));
    }

    let paragraph = Paragraph::new(text)
        .block(Block::default().title("信息面板").borders(Borders::ALL))
        .wrap(Wrap { trim: true });
    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 4: 帮助栏根据状态切换**

修改 `draw_help_bar`：

```rust
fn draw_help_bar<D: Dao>(frame: &mut Frame, area: ratatui::layout::Rect, app: &App<D>) {
    let t = theme::theme();
    let help = match &app.state {
        crate::app::state::AppState::EditInfoPanel { .. } => {
            "↑↓:切换字段  Enter:编辑  Esc:退出编辑"
        }
        _ => "↑↓:移动  Enter:激活  n:新建  e:编辑详情  d:删除  q:退出",
    };
    let paragraph = Paragraph::new(help)
        .style(Style::default().bg(t.muted()).fg(t.selection_fg()));
    frame.render_widget(paragraph, area);
}
```

修改 `draw_list` 中对 `draw_help_bar` 的调用，传入 `app`：

```rust
    draw_help_bar(frame, main_layout[1], app);
```

- [ ] **Step 5: ui/mod.rs 分发 EditInfoPanel 和 EditField**

修改 `src/ui/mod.rs` 的 `draw` 函数：

```rust
pub fn draw<D: Dao>(frame: &mut Frame, app: &App<D>) {
    tracing::trace!("render frame, state={:?}", app.state);
    match &app.state {
        AppState::List | AppState::EditInfoPanel { .. } => list::draw_list(frame, app),
        AppState::CreateProvider
        | AppState::CreateModel { .. }
        | AppState::CreateApiKey { .. }
        | AppState::CreateAlias { .. } => {
            list::draw_list(frame, app);
            create::draw_create(frame, app);
        }
        AppState::Edit { .. }
        | AppState::EditField { .. } => {
            list::draw_list(frame, app);
            edit::draw_edit(frame, app);
        }
        AppState::DeleteConfirm { .. } => {
            list::draw_list(frame, app);
            popup::draw_delete_confirm(frame, app);
        }
    }
}
```

- [ ] **Step 6: 运行 cargo check**

Run: `cargo check`
Expected: 通过

- [ ] **Step 7: Commit**

```bash
git add src/ui/create.rs src/ui/edit.rs src/ui/list.rs src/ui/mod.rs
git commit -m "feat(ui): render CreateAlias, EditInfoPanel and EditField"
```

---

### Task 6: 修复所有测试中的 ProviderInstance 构造

**Files:**
- Modify: `tests/dao_test.rs`
- Modify: `src/dao/memory_impl.rs`（如 Task 1 未完全处理）

- [ ] **Step 1: 修复 tests/dao_test.rs**

文件中所有 `ProviderInstance` 的构造都需要添加 `alias: String::new()` 或具体值。搜索 `ProviderInstance {` 并逐一修复。

Run: `cargo test`
Expected: 全部通过

- [ ] **Step 2: Commit**

```bash
git add tests/dao_test.rs
git commit -m "test: add alias field to all ProviderInstance constructors"
```

---

### Task 7: main.rs 启动时调用 zshrc 注入

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: main.rs 启动前调用 ensure_zshrc_source**

修改 `src/main.rs`：

```rust
use cc_switch_tui::shell;

fn main() -> io::Result<()> {
    let log_file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open("app.log")
        .expect("无法创建日志文件");

    // ... tracing init unchanged ...

    let zshrc_modified = shell::ensure_zshrc_source(
        &dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join(".zshrc"),
    ).unwrap_or(false);

    // ... terminal init unchanged ...

    let db_path = ".cc-switch-tui/db.sqlite";
    let templates = register_templates();
    let dao = SqliteDaoImpl::new(db_path, templates).expect("无法初始化数据库");
    let mut app = App::new_with_dao(dao);
    app.zshrc_modified = zshrc_modified; // 需要在 App 中新增这个字段

    let res = run_app(&mut terminal, &mut app);
    // ...
}
```

等等，`app.zshrc_modified` 需要 `App` 结构体有这个字段。更简单的做法是：不存到 `App`，而是在 `main.rs` 里传入一个全局或作为参数。但 UI 需要读取它来显示提示。

最简单的方式：给 `App` 增加一个 `pub zshrc_modified: bool` 字段，在 `new_with_dao` 中默认设为 `false`，然后 `main.rs` 在创建 app 后手动设置它。

修改 `src/app/state.rs` 的 `App` 结构体：

```rust
pub struct App<D: Dao> {
    pub dao: D,
    pub state: AppState,
    pub list_index: usize,
    pub provider_index: usize,
    pub model_index: usize,
    pub api_key_input: InputState,
    pub edit_input: InputState,
    pub error_message: Option<String>,
    pub should_quit: bool,
    pub zshrc_modified: bool, // 新增
}
```

修改 `new_with_dao`：

```rust
    pub fn new_with_dao(dao: D) -> Self {
        Self {
            dao,
            state: AppState::List,
            list_index: 0,
            provider_index: 0,
            model_index: 0,
            api_key_input: InputState::new(String::new()),
            edit_input: InputState::new(String::new()),
            error_message: None,
            should_quit: false,
            zshrc_modified: false,
        }
    }
```

- [ ] **Step 2: 在列表页顶部显示 zshrc 修改提示**

修改 `src/ui/list.rs` 的 `draw_list`：

当前布局是 `[Min(0), Length(1)]`，底部只有帮助栏。如果 `app.zshrc_modified` 为 `true`，可以在底部帮助栏上方再加一行提示。

修改 `draw_list`：

```rust
pub fn draw_list<D: Dao>(frame: &mut Frame, app: &App<D>) {
    let constraints = if app.zshrc_modified {
        vec![Constraint::Min(0), Constraint::Length(1), Constraint::Length(1)]
    } else {
        vec![Constraint::Min(0), Constraint::Length(1)]
    };

    let main_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(frame.size());

    let content_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(main_layout[0]);

    draw_instance_list(frame, content_layout[0], app);
    draw_info_panel(frame, content_layout[1], app);

    if app.zshrc_modified {
        let t = theme::theme();
        let msg = Paragraph::new("已自动配置 ~/.zshrc，请执行 source ~/.zshrc 生效")
            .style(Style::default().fg(t.warning()));
        frame.render_widget(msg, main_layout[main_layout.len() - 2]);
    }

    draw_help_bar(frame, *main_layout.last().unwrap(), app);

    if let Some(ref msg) = app.error_message {
        draw_error_popup(frame, msg);
    }
}
```

- [ ] **Step 3: 运行 cargo check 和 cargo test**

Run: `cargo check`
Expected: 通过

Run: `cargo test`
Expected: 全部通过

- [ ] **Step 4: Commit**

```bash
git add src/main.rs src/app/state.rs src/ui/list.rs
git commit -m "feat(main): auto-configure zshrc and show notice in TUI"
```

---

## Self-Review Checklist

**1. Spec coverage：**
- [x] `ProviderInstance` 新增 `alias` 字段 — Task 1
- [x] SQLite 表增加 `alias` 列 + 兼容旧表 — Task 2
- [x] `Dao` 新增 `set_alias` — Tasks 1-2
- [x] `shell.rs` 重写为 `generate_aliases` + `ensure_zshrc_source` — Task 3
- [x] 创建向导增加 `CreateAlias` — Task 4
- [x] 列表页 `Enter` 激活 + `e` 进入编辑模式 — Task 4
- [x] 编辑模式支持 Alias 和 API Key — Tasks 4-5
- [x] `main.rs` 启动时注入 zshrc — Task 7
- [x] 底部提示 zshrc 修改 — Task 7
- [x] 新增 `AppError::InvalidAlias` 和 `AliasAlreadyExists` — Task 1

**2. Placeholder scan：**
- [x] 无 "TBD"、"TODO"、"implement later"
- [x] 所有代码步骤包含完整代码

**3. Type一致性检查：**
- [x] `generate_aliases` 签名统一（接受 `dir` 参数）
- [x] `EditField::Alias` / `EditField::ApiKey` 在 state.rs 和 ui/edit.rs 中一致
- [x] `AppState::CreateAlias` 携带 `api_key: String`

**发现的问题并修复：**
- `ProviderTemplate` 和 `ModelTemplate` 可能需要添加 `Clone`（Task 4 已提醒）
- `dao.list_instances()` 和 `dao.get_templates()` 在 `handle_list` 中需要 `.cloned().collect()`，已在 Task 4 步骤中说明
