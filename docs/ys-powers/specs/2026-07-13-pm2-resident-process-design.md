# Spec: 将 cc-switch-tui 封装为 PM2 常驻进程

> 上游 intent：[docs/ys-powers/intent/pm2-resident-process.md](../intent/pm2-resident-process.md)
> 工作分支：`feat/pm2-resident-process`

## Objective

**做什么**：让 `cc-switch-tui` 可以通过 PM2 在后台长期运行。为支持后台常驻，需要在 Rust 入口里新增一个环境变量开关来禁用自动打开浏览器，并把日志输出位置从项目工作目录收敛到 `~/.cc-switch-tui/`；同时新增 PM2 配置文件与相关文档。

**为什么**：当前 `cc-switch-tui` 启动时会自动打开浏览器，且把 `app.log` 写到当前工作目录，这两个行为都不适合作为无头常驻服务运行。用户已经在本地用 PM2 管理 `cc-view`、`subconverter-clash` 等进程，希望把 `cc-switch-tui` 也纳入同一套管理。

**目标用户**：在本地 Mac/Linux 机器上用 PM2 管理常驻服务的开发者/用户。

**成功的样子**：

| 验收项 | 期望 |
|---|---|
| `cargo build --release` 后 `pm2 start ecosystem.config.js` 能拉起进程 | `pm2 status` 显示 `cc-switch-tui` online |
| 启动时不弹浏览器 | 在 headless / 无 GUI 环境也正常启动，日志无 `webbrowser` 报错 |
| 崩溃后自动重启 | 杀进程后 PM2 自动重新拉起 |
| 日志落在 `~/.cc-switch-tui/app.log` | 项目根目录不再生成 `app.log` |
| 开机自启可用 | `pm2 save` + `pm2 startup` 后重启机器能自动恢复 |
| 文档一致 | `README.md` 与 `ARCHITECTURE.md` 描述与实际行为一致 |

---

## Tech Stack

- Rust 2024 edition（项目既定）
- `std::env::var_os` / `std::env::var` 读取环境变量
- `std::fs::create_dir_all` 确保日志目录存在
- PM2 5.x（用户本地已安装）
- `ecosystem.config.js` 使用 CommonJS 格式
- 不引入新 Rust crate / npm 依赖

---

## Commands

```bash
# 构建 release 二进制
cargo build --release
# 或全量构建（含前端）
make build

# 代码质量门
make lint              # cargo clippy -D warnings + eslint
cargo test             # 全部 Rust 测试
make test              # cargo test + npm test

# PM2 操作（构建完成后）
pm2 start ecosystem.config.js
pm2 status cc-switch-tui
pm2 logs cc-switch-tui
pm2 restart cc-switch-tui
pm2 stop cc-switch-tui
pm2 delete cc-switch-tui

# 开机自启
pm2 save
pm2 startup

# 手动验证健康检查
curl http://127.0.0.1:7480/api/health
```

---

## Project Structure

本次改动触及的文件：

```
src/main.rs                                 # 新增 CC_SWITCH_NO_OPEN 判断 + 改日志路径
  ├─ 读取 CC_SWITCH_NO_OPEN 环境变量        # 1 / true（大小写不敏感）时跳过 webbrowser::open
  └─ 日志文件改为 default_cc_dir()/app.log  # 创建目录后打开

ecosystem.config.js                         # 新增 PM2 配置文件（项目根目录）

README.md                                   # 增加 PM2 部署说明

docs/codebase/ARCHITECTURE.md               # 更新 Logging 段落中 app.log 位置描述

docs/ys-powers/specs/2026-07-13-pm2-resident-process-design.md   # 本文件
```

不动：
- `src/data_migration.rs`（仅复用 `default_cc_dir()`，不修改逻辑）
- `src/port.rs`（端口绑定逻辑不变）
- `src/api/settings.rs`（`auto_open_browser` 内存设置行为不变）
- `web/`、`web-dist/`、前端构建流程
- `Makefile`（不新增 target，但 README 会说明 `make build` 是前置步骤）

---

## Code Style

### 1. 环境变量读取 helper

在 `src/main.rs` 中新增一个私有 helper：

```rust
fn is_no_open() -> bool {
    std::env::var("CC_SWITCH_NO_OPEN")
        .map(|v| v.eq_ignore_ascii_case("1") || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}
```

- 只认 `1` 或 `true`（大小写不敏感），其他值（如 `0`、`false`、空字符串）视为未启用。
- 没有该环境变量时保持原行为：仍然尝试自动打开浏览器。

### 2. 日志路径初始化

在 `src/main.rs` 中，把：

```rust
let log_file = std::fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open("app.log")
    .expect("无法创建日志文件");
```

改为：

```rust
let cc_dir = default_cc_dir();
std::fs::create_dir_all(&cc_dir).expect("无法创建数据目录");
let log_path = cc_dir.join("app.log");
let log_file = std::fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open(&log_path)
    .expect("无法创建日志文件");
```

注意点：
- `cc_dir` 的创建要提前到日志初始化之前。
- 后续 `ensure_data_migrated` 和 DAO 初始化继续复用同一个 `cc_dir`。
- 日志路径变更后，原项目根目录的 `app.log` 不再生成；已有的根目录 `app.log` 不会被删除或迁移。

### 3. 自动开浏览器逻辑

把原来的：

```rust
let auto_open = {
    let s = state.settings.read().await;
    s.auto_open_browser
};
```

改为环境变量优先：

```rust
let auto_open = {
    let s = state.settings.read().await;
    s.auto_open_browser && !is_no_open()
};
```

- 保留 `settings.auto_open_browser` 的内存默认值 `true`。
- 当 `CC_SWITCH_NO_OPEN=1` 时，无论 settings 如何都不打开浏览器。

### 4. `ecosystem.config.js`

```javascript
module.exports = {
  apps: [
    {
      name: 'cc-switch-tui',
      script: './target/release/cc-switch-tui',
      cwd: '.',
      exec_mode: 'fork',
      instances: 1,
      autorestart: true,
      env: {
        CC_SWITCH_NO_OPEN: '1',
        RUST_LOG: 'INFO',
      },
      max_restarts: 10,
      min_uptime: '5s',
    },
  ],
};
```

说明：
- `cwd: '.'` 表示 PM2 启动时的工作目录为项目根目录（即 `ecosystem.config.js` 所在目录）。
- `script` 使用相对路径 `./target/release/cc-switch-tui`，因此只要重新执行 `cargo build --release`，PM2 重启后就会加载新二进制。
- `instances: 1` + `exec_mode: 'fork'`：因为后端固定监听 7480，不能多实例。
- `max_restarts` / `min_uptime` 避免启动失败时无限快速重启。

---

## Testing Strategy

### 自动测试

1. **环境变量判断测试**：
   - 新增一个 `#[cfg(test)]` 单元测试覆盖 `is_no_open()`：
     - `CC_SWITCH_NO_OPEN=1` → `true`
     - `CC_SWITCH_NO_OPEN=true` → `true`
     - `CC_SWITCH_NO_OPEN=True` → `true`
     - `CC_SWITCH_NO_OPEN=0` → `false`
     - `CC_SWITCH_NO_OPEN=false` → `false`
     - 未设置 → `false`
   - 测试需要在每个用例前后清理 `std::env::remove_var` / `set_var`，避免串扰。

2. **日志路径测试**：
   - 由于 `main()` 副作用较多，不适合直接单测。改为通过 `cargo build --release` 后手动验证 `~/.cc-switch-tui/app.log` 是否生成且项目根目录无 `app.log`。

3. **回归测试**：
   - `cargo test` 全部通过。
   - `cargo clippy --all-targets -- -D warnings` 无新增 warning。

### 手动测试

```bash
# 1. 构建
cargo build --release

# 2. 清理旧日志，便于观察
rm -f app.log ~/.cc-switch-tui/app.log

# 3. 用 PM2 启动
pm2 start ecosystem.config.js

# 4. 验证进程在线
pm2 status cc-switch-tui

# 5. 验证健康检查
curl http://127.0.0.1:7480/api/health

# 6. 验证日志位置
ls app.log || echo "项目根目录没有 app.log ✅"
tail ~/.cc-switch-tui/app.log

# 7. 验证自动重启
pkill -f "target/release/cc-switch-tui"
sleep 5
pm2 status cc-switch-tui   # 应仍显示 online

# 8. 停止并清理
pm2 stop ecosystem.config.js
pm2 delete ecosystem.config.js
```

---

## Boundaries

### 必须做
- `CC_SWITCH_NO_OPEN` 环境变量判断要精确：只认 `1` / `true`（大小写不敏感）。
- 日志目录创建必须在打开日志文件之前。
- `ecosystem.config.js` 必须纳入 git。
- `README.md` 必须包含 PM2 启动 / 停止 / 重启 / 开机自启命令。
- `ARCHITECTURE.md` 必须同步更新日志路径描述。

### 必须先问再做
- 如果要引入 `clap` 等命令行参数解析库 → 必须先回到 intent/spec 阶段重新确认。
- 如果要修改端口 7480 或支持可配置端口 → 必须先确认，因为 `ys-proxy` 硬编码依赖该端口。
- 如果要将 release 二进制复制到 `~/.cc-switch-tui/` 并由 PM2 从那里启动 → 需要重新确认部署流程。

### 绝不做
- 不改动 `cargo run` 开发模式的行为。
- 不在 PM2 启动 / 重启时自动执行 `cargo build --release`。
- 不引入 Node.js 包装脚本或 shell 包装脚本。
- 不修改端口 7480 的绑定逻辑。
- 不修改 `~/.cc-switch-tui/` 数据目录的解析逻辑。
- 不删除或迁移项目根目录已有的 `app.log`。

---

## Acceptance Criteria

1. `cargo test` 全部通过。
2. `cargo build --release` 成功，且 `target/release/cc-switch-tui` 存在。
3. `cargo clippy --all-targets -- -D warnings` 无新增 warning。
4. `pm2 start ecosystem.config.js` 成功启动进程，`pm2 status` 显示 `cc-switch-tui` online。
5. 启动时未尝试打开浏览器（在 headless 环境也能正常启动，日志无 `webbrowser` 错误）。
6. 进程崩溃后 PM2 能自动重启（可通过 `pkill` 模拟验证）。
7. `~/.cc-switch-tui/app.log` 存在且包含启动日志；项目根目录不再生成新的 `app.log`。
8. `curl http://127.0.0.1:7480/api/health` 返回 `200`。
9. `README.md` 包含 PM2 部署说明。
10. `docs/codebase/ARCHITECTURE.md` 中 Logging 段落描述 `app.log` 位于 `~/.cc-switch-tui/`。

---

## Risks and Mitigations

| 风险 | 影响 | 缓解 |
|---|---|---|
| 日志目录 `~/.cc-switch-tui` 创建失败导致启动 panic | 高 | 在日志初始化前显式 `create_dir_all`；使用与 SQLite 初始化相同的 `default_cc_dir()` |
| `CC_SWITCH_NO_OPEN` 语义过宽导致用户意外禁用浏览器 | 中 | 只认 `1` / `true`，其他值视为 false，保持原行为 |
| `ecosystem.config.js` 中 `cwd` 理解错误导致 PM2 找不到二进制 | 中 | `cwd: '.'` 表示配置文件所在目录；`script` 用相对路径 `./target/release/cc-switch-tui` |
| 用户换电脑后 home 路径变化，但 PM2 配置使用相对路径不受影响 | 低 | 已避免使用绝对路径；只要重新 clone 并 build 即可 |
| 端口 7480 被占用导致 PM2 启动失败 | 低 | 原逻辑已会报错；PM2 `autorestart` 会重试，但需用户自行解决端口冲突 |
| 旧文档与 `app.log` 新位置不一致 | 低 | 同步更新 `ARCHITECTURE.md` 与 `README.md` |

---

## References

- 上游 intent：`docs/ys-powers/intent/pm2-resident-process.md`
- 入口文件：`src/main.rs`
- 数据目录解析：`src/data_migration.rs::default_cc_dir()`
- 端口逻辑：`src/port.rs`
- 架构文档：`docs/codebase/ARCHITECTURE.md`（Main Binary / Logging 段落）
- PM2 文档：https://pm2.keymetrics.io/docs/usage/application-declaration/
