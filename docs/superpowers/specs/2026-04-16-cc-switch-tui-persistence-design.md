# cc-switch-tui 数据持久化设计文档

## 1. 目标

将实例数据从内存存储迁移到 SQLite 持久化存储，使 TUI 重启后用户创建的 provider 实例不丢失。

## 2. 设计原则

- **保留 Dao trait**：不破坏现有接口契约，`App` 和 UI 层无需改动
- **实时写入**：每次创建/编辑/删除实例后立即落盘，意外退出不丢数据
- **测试友好**：保留 `MemoryDaoImpl` 用于单元测试，新建 `SqliteDaoImpl` 用于运行期
- **最小依赖**：使用同步 `rusqlite`，不引入 async runtime

## 3. 文件结构变更

```
src/dao/
├── mod.rs           # 导出 memory_impl 和 sqlite_impl
├── memory_impl.rs   # 保留，供测试使用
└── sqlite_impl.rs   # 新增，Dao 的 SQLite 实现
```

其他修改：
- `Cargo.toml` — 添加 `rusqlite = { version = "0.32", features = ["bundled"] }`
- `src/app/state.rs` — 将 `App.dao` 从 `MemoryDaoImpl` 改为 `Box<dyn Dao>`，并调整 `new()` / 新增 `new_with_dao()`
- `src/main.rs` — 初始化 `SqliteDaoImpl` 替代 `MemoryDaoImpl`

## 4. 数据库存储位置

- **路径**：当前工作目录下的 `.cc-switch-tui/db.sqlite`
- **初始化**：`SqliteDaoImpl::new(path)` 自动 `create_dir_all` 目录并建表

## 5. 数据库 Schema

```sql
CREATE TABLE IF NOT EXISTS instances (
    id TEXT PRIMARY KEY,
    template_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    api_key TEXT NOT NULL,
    created_at TEXT NOT NULL,
    is_current INTEGER NOT NULL DEFAULT 0
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_current ON instances(is_current) WHERE is_current = 1;
```

**字段说明：**
- `id`：`template_id-model_id` 组合主键
- `created_at`：ISO 8601 / RFC 3339 字符串（如 `2026-04-16T10:00:00Z`）
- `is_current`：`0` 或 `1`，标记当前选中的实例

**API Key**：明文存储（本地单用户场景，与当前威胁模型一致）

## 6. SqliteDaoImpl 实现策略

### 6.1 结构定义

```rust
pub struct SqliteDaoImpl {
    conn: rusqlite::Connection,
    templates: Vec<ProviderTemplate>,
}
```

### 6.2 方法映射

| Dao 方法 | 数据库操作 |
|----------|-----------|
| `get_templates` / `get_template` | 走内存 `templates`，和之前一致 |
| `list_instances` | `SELECT * FROM instances` |
| `get_instance(id)` | `SELECT * FROM instances WHERE id = ?` |
| `get_current_instance` | `SELECT * FROM instances WHERE is_current = 1` |
| `create_instance` | `INSERT INTO instances (...)`，主键冲突返回 `InstanceAlreadyExists` |
| `update_instance` | `UPDATE instances SET api_key = ? WHERE id = ?`，`changes() == 0` 返回 `InstanceNotFound` |
| `delete_instance` | `DELETE FROM instances WHERE id = ?`，`changes() == 0` 返回 `InstanceNotFound`，同时若该实例为 current 则自动清空 |
| `set_current_instance` | 事务内：先 `UPDATE ... SET is_current = 0`，再 `UPDATE ... SET is_current = 1 WHERE id = ?`，第二步无变更则回滚并返回 `InstanceNotFound` |

### 6.3 错误处理

- `rusqlite::Error` 在 `sqlite_impl.rs` 内部通过 `map_err` 转换为 `AppError`
- 启动时数据库打开失败（目录不可写、磁盘满等）直接 `panic` 并打印错误信息

## 7. 测试策略

在 `sqlite_impl.rs` 的 `#[cfg(test)]` 模块中：

- 使用 `tempfile::NamedTempFile` 创建临时数据库路径
- 覆盖以下场景：
  1. 正常创建实例
  2. 重复创建返回 `InstanceAlreadyExists`
  3. 查询实例
  4. 更新 API Key
  5. 删除实例
  6. 设置当前实例
  7. 删除当前实例后 `get_current_instance` 返回 `None`

`tempfile` 添加到 `dev-dependencies`。

## 8. 启动流程变更

`main.rs` 中：

```rust
let db_path = ".cc-switch-tui/db.sqlite";
let templates = register_templates();
let dao = SqliteDaoImpl::new(db_path, templates).expect("无法初始化数据库");
let mut app = App::new_with_dao(dao);
```

（若 `App::new()` 当前写死 `MemoryDaoImpl`，则新增 `App::new_with_dao(dao: impl Dao)` 或调整为泛型/Box<dyn Dao>）
