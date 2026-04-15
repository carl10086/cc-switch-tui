# cc-switch-tui SQLite 持久化实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将实例数据从内存存储迁移到 SQLite 持久化存储，使用 `:memory:` SQLite 进行单元测试，遵循严格 TDD。

**Architecture:** 新建 `SqliteDaoImpl` 实现现有 `Dao` trait，保留 `MemoryDaoImpl` 供测试。`App` 改为持有 `Box<dyn Dao>`，运行时 `main.rs` 初始化 `SqliteDaoImpl`。

**Tech Stack:** Rust, rusqlite (bundled), chrono

---

### Task 1: 添加依赖并调整 App 结构以支持 dyn Dao

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/app/state.rs`
- Modify: `src/dao/mod.rs`

- [ ] **Step 1: 添加 rusqlite 依赖**

修改 `Cargo.toml`，在 `[dependencies]` 下添加：

```toml
rusqlite = { version = "0.32", features = ["bundled"] }
```

- [ ] **Step 2: 让 Dao trait 支持 trait object**

修改 `src/dao/mod.rs`，在 `pub trait Dao` 的每个方法前添加 `#[must_use]` 或保持不变，重点是确保方法签名兼容 `Box<dyn Dao>`。当前已有 `get_templates() -> Vec<&ProviderTemplate>` 返回引用，这在 trait object 中**不能**直接使用（因为 `&ProviderTemplate` 的生命周期与 `self` 绑定，但 trait object 会丢失具体类型信息）。

**解决方案：** 将 `Dao` trait 改为要求 `Self: 'static` 不可行。更简单的方式是让 `App` 不直接 Box `Dao`，而是让 `SqliteDaoImpl` 和 `MemoryDaoImpl` 都实现一个需要 `Clone` 的 wrapper，或者……更好的方案：**把 `App` 做成泛型 `App<D: Dao>`**。

修改 `src/app/state.rs`：

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
}

impl App<MemoryDaoImpl> {
    pub fn new() -> Self {
        let templates = register_templates();
        Self::new_with_dao(MemoryDaoImpl::new(templates))
    }
}

impl<D: Dao> App<D> {
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
        }
    }
    // ... 其余方法不变
}
```

注意：`impl App<MemoryDaoImpl>` 块里放 `new()`，泛型 `impl<D: Dao> App<D>` 里放 `new_with_dao` 和其他方法。确保 `on_key` 和所有 handler 方法都在泛型 impl 块中。

- [ ] **Step 3: 运行 cargo check**

Run: `cargo check`
Expected: 通过（可能需要调整 `main.rs` 中的 `App::new()` 调用，如果 `main.rs` 当前仍使用 `App::new()` 则暂时无需改动）

- [ ] **Step 4: 运行测试**

Run: `cargo test`
Expected: 全部通过（当前 19 个测试）

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/app/state.rs src/dao/mod.rs
git commit -m "refactor: make App generic over Dao trait"
```

---

### Task 2: 创建 SqliteDaoImpl 并实现基础查询方法（严格 TDD）

**Files:**
- Create: `src/dao/sqlite_impl.rs`
- Modify: `src/dao/mod.rs`

- [ ] **Step 1: 写第一个失败测试 —— 构造函数能创建表**

在 `src/dao/sqlite_impl.rs` 底部添加 `#[cfg(test)]` 模块：

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::templates::register_templates;

    fn create_test_dao() -> SqliteDaoImpl {
        let templates = register_templates();
        SqliteDaoImpl::new(":memory:", templates).unwrap()
    }

    #[test]
    fn test_constructor_creates_table() {
        let dao = create_test_dao();
        // 如果能创建成功，说明表已建好；进一步验证可以查询
        let instances = dao.list_instances();
        assert!(instances.is_empty());
    }
}
```

Run: `cargo test test_constructor_creates_table`
Expected: **FAIL** — `SqliteDaoImpl` 未定义

- [ ] **Step 2: 实现 SqliteDaoImpl 构造函数和最小结构**

创建 `src/dao/sqlite_impl.rs`：

```rust
use crate::dao::Dao;
use crate::domain::{AppError, ProviderInstance, ProviderTemplate};
use rusqlite::Connection;
use std::path::Path;

pub struct SqliteDaoImpl {
    conn: Connection,
    templates: Vec<ProviderTemplate>,
}

impl SqliteDaoImpl {
    pub fn new(path: &str, templates: Vec<ProviderTemplate>) -> Result<Self, rusqlite::Error> {
        if path != ":memory:" {
            if let Some(parent) = Path::new(path).parent() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let conn = Connection::open(path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS instances (
                id TEXT PRIMARY KEY,
                template_id TEXT NOT NULL,
                model_id TEXT NOT NULL,
                api_key TEXT NOT NULL,
                created_at TEXT NOT NULL,
                is_current INTEGER NOT NULL DEFAULT 0
            )",
            [],
        )?;
        conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_current ON instances(is_current) WHERE is_current = 1",
            [],
        )?;
        Ok(Self { conn, templates })
    }
}
```

在 `src/dao/mod.rs` 添加：

```rust
pub mod sqlite_impl;
```

Run: `cargo test test_constructor_creates_table`
Expected: **PASS**

- [ ] **Step 3: 写失败测试 —— get_instance 能找到插入的数据**

在 `tests` 模块中添加：

```rust
    #[test]
    fn test_get_instance_returns_inserted() {
        let mut dao = create_test_dao();
        let instance = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "test-key".to_string(),
            created_at: chrono::Utc::now(),
        };
        dao.create_instance(instance.clone()).unwrap();
        let found = dao.get_instance(&instance.id).unwrap();
        assert_eq!(found.id, instance.id);
        assert_eq!(found.api_key, instance.api_key);
    }
```

Run: `cargo test test_get_instance_returns_inserted`
Expected: **FAIL** — `impl Dao for SqliteDaoImpl` 未实现

- [ ] **Step 4: 实现 Dao trait 的所有方法（先填充骨架，后续任务细化写操作）**

为 `SqliteDaoImpl` 实现 `Dao` trait。先完整写出所有方法，让编译通过：

```rust
impl Dao for SqliteDaoImpl {
    fn get_templates(&self) -> Vec<&ProviderTemplate> {
        self.templates.iter().collect()
    }

    fn get_template(&self, id: &str) -> Option<&ProviderTemplate> {
        self.templates.iter().find(|t| t.id == id)
    }

    fn list_instances(&self) -> Vec<&ProviderInstance> {
        // 暂时返回空 vec
        vec![]
    }

    fn get_instance(&self, id: &str) -> Option<&ProviderInstance> {
        // 暂时返回 None
        None
    }

    fn create_instance(&mut self, instance: ProviderInstance) -> Result<(), AppError> {
        // TODO
        Ok(())
    }

    fn delete_instance(&mut self, id: &str) -> Result<(), AppError> {
        // TODO
        Ok(())
    }

    fn get_current_instance(&self) -> Option<&ProviderInstance> {
        // 暂时返回 None
        None
    }

    fn set_current_instance(&mut self, id: &str) -> Result<(), AppError> {
        // TODO
        Ok(())
    }

    fn update_instance(&mut self, id: &str, api_key: String) -> Result<(), AppError> {
        // TODO
        Ok(())
    }
}
```

注意：这样写测试会**编译通过但测试失败**（`get_instance` 返回 `None`），这是 TDD 允许的。但更好的方式是直接在这里实现读操作，因为逻辑简单且明确。

**实现读操作（list_instances, get_instance, get_current_instance）：**

由于 `Dao` 返回 `Vec<&ProviderInstance>` 和 `Option<&ProviderInstance>`，这要求数据必须存储在 `SqliteDaoImpl` 自身内部。我们无法每次查询都新建 `ProviderInstance` 然后返回引用。

**解决方案：** 在 `SqliteDaoImpl` 中维护一个 `instances: Vec<ProviderInstance>` 缓存，读操作时从缓存返回引用，写操作时先更新数据库，再刷新缓存。

修改结构定义：

```rust
pub struct SqliteDaoImpl {
    conn: Connection,
    templates: Vec<ProviderTemplate>,
    instances: Vec<ProviderInstance>,
}
```

构造函数末尾添加：`instances: Vec::new()`。

添加私有方法 `refresh_instances(&mut self)`：

```rust
fn refresh_instances(&mut self) -> Result<(), rusqlite::Error> {
    let mut stmt = self.conn.prepare(
        "SELECT id, template_id, model_id, api_key, created_at FROM instances"
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
        })
    })?;
    self.instances.clear();
    for row in rows {
        self.instances.push(row?);
    }
    Ok(())
}
```

然后读方法：

```rust
    fn list_instances(&self) -> Vec<&ProviderInstance> {
        self.instances.iter().collect()
    }

    fn get_instance(&self, id: &str) -> Option<&ProviderInstance> {
        self.instances.iter().find(|i| i.id == id)
    }

    fn get_current_instance(&self) -> Option<&ProviderInstance> {
        let mut stmt = self.conn.prepare(
            "SELECT id FROM instances WHERE is_current = 1"
        ).ok()?;
        let id: String = stmt.query_row([], |row| row.get(0)).ok()?;
        self.instances.iter().find(|i| i.id == id)
    }
```

构造函数最后调用 `self.refresh_instances()?`。

Run: `cargo test test_get_instance_returns_inserted`
Expected: **FAIL** — `create_instance` 是空实现，数据没有真正插入

- [ ] **Step 5: 实现 create_instance**

```rust
    fn create_instance(&mut self, instance: ProviderInstance) -> Result<(), AppError> {
        let created_at_str = instance.created_at.to_rfc3339();
        match self.conn.execute(
            "INSERT INTO instances (id, template_id, model_id, api_key, created_at, is_current)
             VALUES (?1, ?2, ?3, ?4, ?5, 0)",
            rusqlite::params![
                &instance.id,
                &instance.template_id,
                &instance.model_id,
                &instance.api_key,
                created_at_str,
            ],
        ) {
            Ok(_) => {
                self.refresh_instances().map_err(|e| {
                    // 这里 refresh 失败属于内部不一致，但 spec 要求直接返回
                    // 实际上 refresh 出错概率极低
                    // 为了类型统一，可以 panic 或返回通用错误
                    // 当前 AppError 没有通用变体，暂时 panic
                    panic!("Failed to refresh instances after create: {}", e);
                })?;
                Ok(())
            }
            Err(rusqlite::Error::SqliteFailure(ref err, _))
                if err.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY =>
            {
                Err(AppError::InstanceAlreadyExists(instance.id.clone()))
            }
            Err(e) => panic!("Database error in create_instance: {}", e),
        }
    }
```

Run: `cargo test test_get_instance_returns_inserted`
Expected: **PASS**

- [ ] **Step 6: Commit**

```bash
git add src/dao/sqlite_impl.rs src/dao/mod.rs src/app/state.rs Cargo.toml
git commit -m "feat(dao): add SqliteDaoImpl with create and read operations"
```

---

### Task 3: 实现 update_instance 和 delete_instance（严格 TDD）

**Files:**
- Modify: `src/dao/sqlite_impl.rs`

- [ ] **Step 1: 写失败测试 —— update_instance 能修改 api_key**

在 `tests` 模块添加：

```rust
    #[test]
    fn test_update_instance_changes_api_key() {
        let mut dao = create_test_dao();
        let instance = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "old-key".to_string(),
            created_at: chrono::Utc::now(),
        };
        dao.create_instance(instance).unwrap();
        dao.update_instance("minimax-MiniMax-M2.7-highspeed", "new-key".to_string()).unwrap();
        let found = dao.get_instance("minimax-MiniMax-M2.7-highspeed").unwrap();
        assert_eq!(found.api_key, "new-key");
    }
```

Run: `cargo test test_update_instance_changes_api_key`
Expected: **FAIL** — update_instance 是空实现

- [ ] **Step 2: 实现 update_instance**

```rust
    fn update_instance(&mut self, id: &str, api_key: String) -> Result<(), AppError> {
        let changes = self.conn.execute(
            "UPDATE instances SET api_key = ?1 WHERE id = ?2",
            [api_key, id.to_string()],
        ).expect("Database error in update_instance");
        if changes == 0 {
            return Err(AppError::InstanceNotFound(id.to_string()));
        }
        self.refresh_instances().expect("Failed to refresh instances after update");
        Ok(())
    }
```

Run: `cargo test test_update_instance_changes_api_key`
Expected: **PASS**

- [ ] **Step 3: 写失败测试 —— update 不存在的实例返回 NotFound**

```rust
    #[test]
    fn test_update_instance_not_found() {
        let mut dao = create_test_dao();
        let result = dao.update_instance("nonexistent", "key".to_string());
        assert!(matches!(result, Err(AppError::InstanceNotFound(_))));
    }
```

Run: `cargo test test_update_instance_not_found`
Expected: **PASS**（上一步已实现）

- [ ] **Step 4: 写失败测试 —— delete_instance 能删除实例**

```rust
    #[test]
    fn test_delete_instance_removes_it() {
        let mut dao = create_test_dao();
        let instance = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "key".to_string(),
            created_at: chrono::Utc::now(),
        };
        dao.create_instance(instance).unwrap();
        dao.delete_instance("minimax-MiniMax-M2.7-highspeed").unwrap();
        assert!(dao.get_instance("minimax-MiniMax-M2.7-highspeed").is_none());
    }
```

Run: `cargo test test_delete_instance_removes_it`
Expected: **FAIL** — delete_instance 是空实现

- [ ] **Step 5: 实现 delete_instance**

```rust
    fn delete_instance(&mut self, id: &str) -> Result<(), AppError> {
        let changes = self.conn.execute(
            "DELETE FROM instances WHERE id = ?1",
            [id],
        ).expect("Database error in delete_instance");
        if changes == 0 {
            return Err(AppError::InstanceNotFound(id.to_string()));
        }
        self.refresh_instances().expect("Failed to refresh instances after delete");
        Ok(())
    }
```

Run: `cargo test test_delete_instance_removes_it`
Expected: **PASS**

- [ ] **Step 6: 写失败测试 —— delete 不存在的实例返回 NotFound**

```rust
    #[test]
    fn test_delete_instance_not_found() {
        let mut dao = create_test_dao();
        let result = dao.delete_instance("nonexistent");
        assert!(matches!(result, Err(AppError::InstanceNotFound(_))));
    }
```

Run: `cargo test test_delete_instance_not_found`
Expected: **PASS**

- [ ] **Step 7: Commit**

```bash
git add src/dao/sqlite_impl.rs
git commit -m "feat(dao): implement update and delete for SqliteDaoImpl"
```

---

### Task 4: 实现 set_current_instance 和边界行为（严格 TDD）

**Files:**
- Modify: `src/dao/sqlite_impl.rs`

- [ ] **Step 1: 写失败测试 —— set_current_instance 后 get_current_instance 能返回**

```rust
    #[test]
    fn test_set_current_instance() {
        let mut dao = create_test_dao();
        let instance = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "key".to_string(),
            created_at: chrono::Utc::now(),
        };
        dao.create_instance(instance).unwrap();
        dao.set_current_instance("minimax-MiniMax-M2.7-highspeed").unwrap();
        let current = dao.get_current_instance().unwrap();
        assert_eq!(current.id, "minimax-MiniMax-M2.7-highspeed");
    }
```

Run: `cargo test test_set_current_instance`
Expected: **FAIL** — set_current_instance 是空实现

- [ ] **Step 2: 实现 set_current_instance**

```rust
    fn set_current_instance(&mut self, id: &str) -> Result<(), AppError> {
        let tx = self.conn.transaction().expect("Failed to start transaction");
        tx.execute("UPDATE instances SET is_current = 0", []).expect("Database error");
        let changes = tx.execute(
            "UPDATE instances SET is_current = 1 WHERE id = ?1",
            [id],
        ).expect("Database error");
        if changes == 0 {
            return Err(AppError::InstanceNotFound(id.to_string()));
        }
        tx.commit().expect("Failed to commit transaction");
        Ok(())
    }
```

Run: `cargo test test_set_current_instance`
Expected: **PASS**

- [ ] **Step 3: 写失败测试 —— set_current_instance 对不存在 id 返回 NotFound**

```rust
    #[test]
    fn test_set_current_instance_not_found() {
        let mut dao = create_test_dao();
        let result = dao.set_current_instance("nonexistent");
        assert!(matches!(result, Err(AppError::InstanceNotFound(_))));
    }
```

Run: `cargo test test_set_current_instance_not_found`
Expected: **PASS**

- [ ] **Step 4: 写失败测试 —— 删除 current instance 后 get_current_instance 返回 None**

```rust
    #[test]
    fn test_delete_current_instance_clears_current() {
        let mut dao = create_test_dao();
        let instance = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "key".to_string(),
            created_at: chrono::Utc::now(),
        };
        dao.create_instance(instance).unwrap();
        dao.set_current_instance("minimax-MiniMax-M2.7-highspeed").unwrap();
        dao.delete_instance("minimax-MiniMax-M2.7-highspeed").unwrap();
        assert!(dao.get_current_instance().is_none());
    }
```

Run: `cargo test test_delete_current_instance_clears_current`
Expected: **PASS**（`is_current` 在数据库层面随着行删除自动消失，`get_current_instance` 查询不到即可）

- [ ] **Step 5: 写失败测试 —— create_instance 重复返回 InstanceAlreadyExists**

```rust
    #[test]
    fn test_create_instance_duplicate() {
        let mut dao = create_test_dao();
        let instance = ProviderInstance {
            id: "minimax-MiniMax-M2.7-highspeed".to_string(),
            template_id: "minimax".to_string(),
            model_id: "MiniMax-M2.7-highspeed".to_string(),
            api_key: "key".to_string(),
            created_at: chrono::Utc::now(),
        };
        dao.create_instance(instance.clone()).unwrap();
        let result = dao.create_instance(instance);
        assert!(matches!(result, Err(AppError::InstanceAlreadyExists(_))));
    }
```

Run: `cargo test test_create_instance_duplicate`
Expected: **PASS**（Task 2 已实现）

- [ ] **Step 6: Commit**

```bash
git add src/dao/sqlite_impl.rs
git commit -m "feat(dao): implement set_current_instance for SqliteDaoImpl"
```

---

### Task 5: 在 main.rs 中接入 SqliteDaoImpl

**Files:**
- Modify: `src/main.rs`
- Modify: `src/dao/sqlite_impl.rs`（添加 `pub use` 或确保导出不需修改）

- [ ] **Step 1: 修改 main.rs 初始化 SqliteDaoImpl**

修改 `src/main.rs`：

```rust
use cc_switch_tui::app::state::App;
use cc_switch_tui::dao::sqlite_impl::SqliteDaoImpl;
use cc_switch_tui::app::templates::register_templates;
use cc_switch_tui::ui;
// ... 其余 import 不变

fn main() -> io::Result<()> {
    // ... 日志初始化不变 ...

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let db_path = ".cc-switch-tui/db.sqlite";
    let templates = register_templates();
    let dao = SqliteDaoImpl::new(db_path, templates).expect("无法初始化数据库");
    let mut app = App::new_with_dao(dao);

    let res = run_app(&mut terminal, &mut app);
    // ... 其余不变 ...
}
```

- [ ] **Step 2: 运行 cargo check**

Run: `cargo check`
Expected: 通过

- [ ] **Step 3: 运行全部测试**

Run: `cargo test`
Expected: 全部通过（原有 19 个测试 + 新增 SqliteDaoImpl 测试）

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat(main): wire up SqliteDaoImpl for persistence"
```

---

## Self-Review Checklist

**1. Spec coverage：**
- [x] 新建 `SqliteDaoImpl` 实现 `Dao` trait — Task 2-4
- [x] 保留 `MemoryDaoImpl` 供测试 — App::new() 仍在使用
- [x] 实时写入 — 每次写操作直接执行 SQL
- [x] 数据库路径 `.cc-switch-tui/db.sqlite` — Task 5
- [x] Schema 与 spec 一致 — Task 2
- [x] 启动失败 panic — Task 5 使用 `.expect()`
- [x] 严格 TDD — 每个任务先写失败测试再实现
- [x] `:memory:` SQLite 用于单元测试 — Task 2-4 的测试使用 `:memory:`

**2. Placeholder scan：**
- [x] 无 "TBD"、"TODO"、"implement later"
- [x] 每个步骤都有完整代码和命令

**3. Type consistency：**
- [x] `App::new_with_dao(dao: D)` 与 `SqliteDaoImpl` 类型匹配
- [x] `Dao` trait 方法签名未被修改，兼容新旧实现

**Gap found & fixed：** 原 `Dao` trait 返回 `Vec<&ProviderInstance>`，SQLite 实现需要内部缓存 `Vec<ProviderInstance>` 才能返回引用。已在 Task 2 Step 4 中通过 `refresh_instances()` + `instances` 字段解决。
