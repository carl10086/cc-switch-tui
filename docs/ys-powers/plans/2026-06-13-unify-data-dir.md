# 统一数据目录到 ~/.cc-switch-tui/ — 实施计划

## Spec 来源

- `docs/ys-powers/specs/2026-06-13-unify-data-dir-design.md`

## 当前状态

- `src/main.rs` 使用相对路径 `.cc-switch-tui/db.sqlite` / `.cc-switch-tui/traces.sqlite`
- 实际数据落在项目目录 `/Users/yusizhen/soft/projects/cc-switch-tui/.cc-switch-tui/`
- `src/api/health.rs` 硬编码返回 `".cc-switch-tui/db.sqlite"`
- `src/api/diagnostics.rs` 已经在使用 `~/.cc-switch-tui/`，与其他地方不一致
- 项目目录下已有真实数据：3 条 instances、若干 trace sessions
- home 目录下 `db.sqlite` 为空、`traces.sqlite` 为空

## 组件依赖图

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  src/main.rs    │────>│ src/data_migration.rs │────>│ ~/.cc-switch-tui/ │
│ (启动入口)       │     │ (迁移判断+复制)   │     │ (新数据目录)     │
└─────────────────┘     └──────────────────┘     └─────────────────┘
         │                                               ▲
         │                                               │
         │         ┌─────────────────┐                   │
         └────────>│ src/api/health.rs│───────────────────┘
                   │ (返回真实 db_path)│
                   └─────────────────┘
```

依赖关系：
- `main.rs` 依赖 `data_migration.rs`：启动前先执行迁移
- `health.rs` 依赖 `dirs::home_dir()` 和固定路径约定
- `data_migration.rs` 只依赖文件系统，不依赖 DAO / TraceStore
- `diagnostics.rs` 不需要改，但迁移后它返回的路径会与实际一致

## 任务列表

### Task 1: 新增 `src/data_migration.rs` 迁移模块

**目标**：实现启动时自动迁移逻辑，并覆盖核心场景的单测。

**实现内容**：
- 新增 `src/data_migration.rs`
- 暴露 `pub fn ensure_data_migrated(home_cc_dir: &Path, project_dir: &Path) -> Result<(), DataMigrationError>`
- 迁移逻辑：
  1. 计算 `project_data_dir = project_dir.join(".cc-switch-tui")`
  2. 如果 `home_cc_dir.join("db.sqlite")` 存在 → 跳过
  3. 如果 `project_data_dir.join("db.sqlite")` 不存在 → 跳过
  4. 否则创建 `home_cc_dir`，复制以下文件（存在才复制）：
     - `db.sqlite`
     - `traces.sqlite`
     - `traces.sqlite-wal`
     - `traces.sqlite-shm`
  5. 记录 `tracing::info!` 日志
- 迁移失败返回 `DataMigrationError` 并阻止启动

**验收标准**：
- [ ] 模块能正确编译
- [ ] 单测覆盖 5 个场景：
  - home 有 db 时跳过
  - 项目目录无旧数据时跳过
  - 正常复制 db.sqlite + traces.sqlite
  - 复制 WAL/SHM 文件
  - 迁移后不删除源文件

**验证步骤**：
```bash
cargo test data_migration
```

---

### Task 2: 修改 `src/main.rs` 使用 home 目录路径并调用迁移

**目标**：让 server 启动时自动把旧数据复制到 `~/.cc-switch-tui/`，并从此使用该目录。

**实现内容**：
- 复用 `cc_switch_tui_home()` 函数获取 `~/.cc-switch-tui/`
- 启动 `SqliteDaoImpl` 和 `TraceStore` 之前调用 `ensure_data_migrated`
- `db_path` 改为 `cc_dir.join("db.sqlite")`
- `trace_store` 路径改为 `cc_dir.join("traces.sqlite")`
- 若迁移失败，返回 `io::Error` 阻止启动

**验收标准**：
- [ ] `main.rs` 不再使用相对路径 `.cc-switch-tui/...`
- [ ] 启动时优先执行迁移
- [ ] 迁移失败时 server 不启动并打印错误

**验证步骤**：
```bash
cargo build
cargo test
```

**Checkpoint 1**：
- Task 1 + Task 2 完成后，编译通过、单测通过，但**不启动 server**。
- 在此暂停确认：代码结构是否满意，再进入集成验证。

---

### Task 3: 修正 `src/api/health.rs` 返回真实路径

**目标**：让 `/api/health` 返回的 `db_path` 与实际使用的路径一致。

**实现内容**：
- 修改 `db_path_default()`：
  - 能获取 home 目录时返回 `~/.cc-switch-tui/db.sqlite` 绝对路径
  - 获取失败时回退 `".cc-switch-tui/db.sqlite"`

**验收标准**：
- [ ] `/api/health` 返回的 `db_path` 是 `~/.cc-switch-tui/db.sqlite`
- [ ] 不破坏现有 health 测试

**验证步骤**：
```bash
cargo test health
cargo test
```

---

### Task 4: 集成验证 — 真实数据迁移

**目标**：在真实环境下验证迁移逻辑，确保项目目录数据被正确复制到 home 目录。

**前置条件**：
- Task 1~3 已完成并通过测试
- 当前 server 正在运行（占用 7480），需要先停止

**步骤**：
1. 停止当前 server
2. 备份并清空 `~/.cc-switch-tui/db.sqlite` 和 `~/.cc-switch-tui/traces.sqlite`（当前是空文件，可直接删除或重命名）
3. 确认项目目录 `/Users/yusizhen/soft/projects/cc-switch-tui/.cc-switch-tui/` 下仍有旧数据
4. 从项目目录启动 server：`cargo run` 或运行构建后的二进制
5. 观察日志，确认出现迁移成功日志
6. 查询 home 目录数据库：
   ```bash
   sqlite3 ~/.cc-switch-tui/db.sqlite "SELECT COUNT(*) FROM instances;"
   sqlite3 ~/.cc-switch-tui/traces.sqlite "SELECT COUNT(*) FROM sessions;"
   ```
7. 确认项目目录旧文件仍然保留
8. 调用 `/api/health` 确认 `dbPath` 为 `~/.cc-switch-tui/db.sqlite`

**验收标准**：
- [ ] home 目录 `db.sqlite` 包含项目目录原有的 3 条 instances
- [ ] home 目录 `traces.sqlite` 包含原有 trace 数据
- [ ] 项目目录旧 `db.sqlite` / `traces.sqlite` 仍然存在
- [ ] `/api/health` 返回路径为 `~/.cc-switch-tui/db.sqlite`
- [ ] server 正常监听 7480

**验证步骤**：
```bash
# 停止 server 后执行
rm -f ~/.cc-switch-tui/db.sqlite ~/.cc-switch-tui/traces.sqlite ~/.cc-switch-tui/traces.sqlite-*
cd /Users/yusizhen/soft/projects/cc-switch-tui
cargo run

# 另一个终端
curl -s http://localhost:7480/api/health | jq '.dbPath'
sqlite3 ~/.cc-switch-tui/db.sqlite "SELECT COUNT(*) FROM instances;"
sqlite3 ~/.cc-switch-tui/traces.sqlite "SELECT COUNT(*) FROM sessions;"
ls -lh /Users/yusizhen/soft/projects/cc-switch-tui/.cc-switch-tui/
```

---

## 风险管理

| 风险 | 缓解措施 |
|------|----------|
| 迁移覆盖 home 已有数据 | 只要 `~/.cc-switch-tui/db.sqlite` 存在就跳过迁移 |
| 迁移过程中文件被占用 | server 启动前 DB 尚未打开，文件未被占用 |
| 迁移后旧 WAL 文件与目标不匹配 | 一起复制 WAL/SHM，复制完成后再打开数据库 |
| 用户误删旧数据 | 不自动删除任何旧文件，保留给用户检查 |

## 回滚方案

若迁移后发现问题：
1. 停止 server
2. 删除 `~/.cc-switch-tui/db.sqlite` 和 `~/.cc-switch-tui/traces.sqlite*`
3. 回滚代码到修改前版本
4. 重新启动 server，继续使用项目目录下的旧数据

## 完成后的清理建议（由用户手动执行）

验证无误后，用户可手动删除项目目录下的旧数据：

```bash
rm -f /Users/yusizhen/soft/projects/cc-switch-tui/.cc-switch-tui/db.sqlite
rm -f /Users/yusizhen/soft/projects/cc-switch-tui/.cc-switch-tui/traces.sqlite*
```
