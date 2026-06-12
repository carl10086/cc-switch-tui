# 统一数据目录到 ~/.cc-switch-tui/

## 1. Objective（目标）

将 `cc-switch-tui` 的运行时数据统一放到 `~/.cc-switch-tui/` 下，解决当前 `main.rs` 使用相对路径导致数据实际落在项目目录 `/Users/yusizhen/soft/projects/cc-switch-tui/.cc-switch-tui/` 的问题。

本次变更需满足：
- `db.sqlite` 和 `traces.sqlite` 默认都在 `~/.cc-switch-tui/`
- 已有数据从项目目录**复制**迁移到 home 目录，**不删除**旧文件
- 迁移过程在 server 启动时自动完成，无需用户手动执行脚本
- 迁移失败时给出明确错误，不静默降级

## 2. Background（背景）

当前代码里数据路径不一致：

| 文件 | 当前路径 | 问题 |
|------|----------|------|
| `src/main.rs` | `.cc-switch-tui/db.sqlite` / `.cc-switch-tui/traces.sqlite` | 相对路径，数据位置取决于启动目录 |
| `src/api/health.rs` | 硬编码 `.cc-switch-tui/db.sqlite` | 返回给前端的路径与实际不一致 |
| `src/api/diagnostics.rs` | `~/.cc-switch-tui/db.sqlite` | 与实际 DB 路径不一致 |
| aliases / opencode / port 文件 | `~/.cc-switch-tui/` | 已经在 home 目录 |

实际运行结果：server 从项目目录启动时，数据写在 `/Users/yusizhen/soft/projects/cc-switch-tui/.cc-switch-tui/`，而用户按直觉查询 `~/.cc-switch-tui/traces.sqlite` 得到空文件。

## 3. Design（设计）

### 3.1 路径统一

所有运行时数据统一以 `~/.cc-switch-tui/` 为根目录：

```rust
let home = dirs::home_dir()?;
let cc_dir = home.join(".cc-switch-tui");
let db_path = cc_dir.join("db.sqlite");
let trace_path = cc_dir.join("traces.sqlite");
```

`src/api/health.rs` 的 `db_path_default()` 同步改为返回 `~/.cc-switch-tui/db.sqlite` 的绝对路径字符串。

### 3.2 启动时自动迁移

新增 `src/data_dir.rs`（或放在 `src/migration.rs`），暴露一个函数：

```rust
pub fn ensure_data_migrated(home_cc_dir: &Path) -> Result<(), DataMigrationError>
```

迁移判断逻辑：

1. 获取项目目录数据路径 `project_dir.join(".cc-switch-tui")`
2. 如果 `home_cc_dir/db.sqlite` **已存在**，跳过迁移（认为 home 目录已是权威数据）
3. 如果 `project_dir/.cc-switch-tui/db.sqlite` **不存在**，跳过迁移（无旧数据）
4. 否则：
   - 确保 `home_cc_dir` 存在
   - 复制 `db.sqlite`、`traces.sqlite` 及其 WAL/SHM 文件（如果存在）到 `home_cc_dir`
   - 复制完成后写一条日志：`"migrated data from {project_dir}/.cc-switch-tui to {home_cc_dir}"`
   - 旧文件保留不动

### 3.3 迁移范围

只迁移 SQLite 数据文件：

- `db.sqlite`
- `traces.sqlite`
- `traces.sqlite-wal`
- `traces.sqlite-shm`

**不迁移**：
- `aliases.zsh`
- `opencode/` 目录
- `port` 文件
- 其他配置文件（这些本来就在 home 目录或会重新生成）

### 3.4 错误处理

迁移失败时：
- 记录 `ERROR` 日志，包含源路径和目标路径
- 返回错误，阻止 server 启动
- 不删除任何旧文件或部分写入的新文件

### 3.5 health.rs 修正

`src/api/health.rs` 当前返回 `".cc-switch-tui/db.sqlite"`，改为返回绝对路径：

```rust
fn db_path_default() -> String {
    dirs::home_dir()
        .map(|h| h.join(".cc-switch-tui/db.sqlite").to_string_lossy().into_owned())
        .unwrap_or_else(|| ".cc-switch-tui/db.sqlite".to_string())
}
```

## 4. Project Structure（项目结构变更）

```
src/
├── main.rs              # 改：使用 home 目录路径；启动前调用迁移
├── api/
│   ├── health.rs        # 改：返回 home 目录绝对路径
│   └── diagnostics.rs   # 不变（已在用 home 目录）
└── data_migration.rs    # 新增：迁移逻辑
```

## 5. Code Style（代码风格）

- 使用 `dirs::home_dir()` 获取 home 目录（项目已有 `dirs` 依赖）
- 路径拼接用 `PathBuf::join`，避免字符串拼接
- 迁移逻辑与业务逻辑分离：迁移只做文件复制，不打开数据库
- 日志用 `tracing::info!` / `tracing::error!`
- 错误类型用 `std::io::Error` 或项目内 `AppError`，保持简洁

## 6. Testing Strategy（测试策略）

### 6.1 单元测试

在 `src/data_migration.rs` 增加测试：

- `test_skip_when_home_db_exists`：home 目录已有 `db.sqlite` 时，不覆盖、不迁移
- `test_skip_when_no_project_data`：项目目录无旧数据时，不迁移
- `test_migrate_copies_sqlite_files`：正常复制 `db.sqlite` 和 `traces.sqlite`
- `test_migrate_copies_wal_files`：复制 WAL/SHM 文件
- `test_migrate_does_not_delete_source`：迁移后旧文件仍存在

测试使用 `tempfile::TempDir` 构造假的 home 和 project 目录，避免污染真实文件系统。

### 6.2 集成验证

1. 停止 server
2. 备份并清空 `~/.cc-switch-tui/db.sqlite` 和 `~/.cc-switch-tui/traces.sqlite`
3. 保留项目目录 `/Users/yusizhen/soft/projects/cc-switch-tui/.cc-switch-tui/` 下的旧数据
4. 从项目目录启动 server
5. 验证：
   - `~/.cc-switch-tui/db.sqlite` 存在且有 3 条 instances
   - `~/.cc-switch-tui/traces.sqlite` 存在且有之前 session 数据
   - 项目目录旧文件仍然保留
   - `/api/health` 返回的 `db_path` 是 `~/.cc-switch-tui/db.sqlite`

### 6.3 回归测试

- 运行 `cargo test` 全量测试
- 确认 `src/api/health.rs` 的测试（如有）需要同步更新

## 7. Boundaries（边界与约束）

### 7.1 本次不做

- 不新增 CLI 命令（如 `rm-data`）
- 不自动删除项目目录旧数据
- 不把 `aliases.zsh` / `opencode/` 纳入迁移范围
- 不支持自定义数据目录（如环境变量覆盖）

### 7.2 风险

- 如果 home 目录和项目目录**同时都有** `db.sqlite`，以 home 目录为准，项目目录数据会被忽略。这是预期行为，避免覆盖用户已有数据。
- 迁移只复制文件，不校验 SQLite 内容完整性。若源文件损坏，目标文件也会损坏。

### 7.3 用户后续操作

迁移完成后，用户应手动检查 `~/.cc-switch-tui/` 下数据正确，然后自行删除 `/Users/yusizhen/soft/projects/cc-switch-tui/.cc-switch-tui/` 下的旧 `db.sqlite` 和 `traces.sqlite`。

## 8. Acceptance Criteria（验收标准）

- [ ] `main.rs` 使用 `~/.cc-switch-tui/db.sqlite` 和 `~/.cc-switch-tui/traces.sqlite`
- [ ] 启动时若 home 无数据、项目目录有旧数据，则自动复制到 home
- [ ] 迁移后项目目录旧文件保留
- [ ] `health.rs` 返回 home 目录的绝对路径
- [ ] 所有现有测试通过
- [ ] 新增迁移模块的单元测试覆盖 4 个核心场景
- [ ] 手动验证 server 启动后数据正确出现在 home 目录
