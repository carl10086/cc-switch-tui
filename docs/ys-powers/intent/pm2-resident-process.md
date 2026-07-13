# Intent: 将 cc-switch-tui 封装为 PM2 常驻进程

## TL;DR

让 `cc-switch-tui` 可以通过 PM2 在后台长期运行，并解决两个后台运行与原交互设计冲突的问题：

1. 新增环境变量 `CC_SWITCH_NO_OPEN`：当值为 `1` 或 `true` 时，启动过程**不自动打开浏览器**。
2. 将 `tracing` 日志输出从当前工作目录的 `app.log` 收敛到 `~/.cc-switch-tui/app.log`，使运行态产物与项目 checkout 路径解耦。
3. 新增 `ecosystem.config.js`（项目根目录，纳入 git），配置 PM2 以 `target/release/cc-switch-tui` 为常驻进程运行。
4. 更新 `README.md` 与 `docs/codebase/ARCHITECTURE.md` 中关于启动方式、日志位置的描述。

---

## Outcome

- 执行 `pm2 start ecosystem.config.js` 后，`cc-switch-tui` 以守护进程方式在后台运行。
- 进程崩溃时 PM2 自动重启；配置开机自启（`pm2 save` + `pm2 startup`）后可随系统启动。
- 启动时**不会**尝试弹浏览器窗口。
- SQLite 数据、trace 数据、port 文件、aliases.zsh、app.log 全部位于 `~/.cc-switch-tui/`，不受项目目录位置影响。
- 换目录或换电脑重新 clone 项目后，只要重新 `cargo build --release`，原 PM2 配置无需修改即可继续工作。

---

## 范围内 (In Scope)

### 代码改动

1. **`src/main.rs`**：
   - 在自动打开浏览器逻辑前读取环境变量 `CC_SWITCH_NO_OPEN`；值为 `1` / `true` 时跳过 `webbrowser::open`。
   - 将日志文件路径从 `"app.log"` 改为 `default_cc_dir().join("app.log")`，并在初始化前确保 `~/.cc-switch-tui` 目录存在。

2. **新增 `ecosystem.config.js`**（项目根目录）：
   - `name: 'cc-switch-tui'`
   - `script: './target/release/cc-switch-tui'`
   - `cwd: '.'`
   - `env: { CC_SWITCH_NO_OPEN: '1' }`
   - `autorestart: true`
   - `instances: 1`，`exec_mode: 'fork'`（端口 7480 固定，只能单实例）

3. **文档更新**：
   - `README.md`：增加 PM2 启动、停止、重启、查看状态、设置开机自启的示例命令。
   - `docs/codebase/ARCHITECTURE.md`：同步更新 Logging 段落中 `app.log` 位置的描述。

4. **版本控制**：
   - `ecosystem.config.js`、`README.md`、`ARCHITECTURE.md` 的改动提交到 git。

### 构建与部署流程

- 构建仍由用户手动执行 `cargo build --release`。
- PM2 只负责运行产物，不负责自动构建或监听源码变化触发编译。

---

## 范围外 (Out of Scope)

- 不改动 `cargo run` 开发模式的行为。
- 不在 PM2 启动 / 重启时自动执行 `cargo build --release`。
- 不引入 Node.js 包装脚本或 shell 包装脚本。
- 不修改端口 7480 的绑定逻辑，不改为可配置端口。
- 不修改 `~/.cc-switch-tui/` 数据目录的解析逻辑。
- 不做日志轮转、日志清理策略。
- 不支持多实例 / cluster 模式。

---

## 关键设计决策与原因

### 决策 1：用环境变量 `CC_SWITCH_NO_OPEN` 而不是命令行参数 `--no-open`

**原因**：

- PM2 的 `ecosystem.config.js` 原生支持 `env` 字段，配置直观且无需处理 `args` 拼接。
- 当前二进制没有命令行参数解析逻辑，加环境变量比引入 `clap` / `std::env::args` 改动更小。
- 与项目已有的 `CC_SWITCH_QUIET`、`CC_SWITCH_PROXY_URL` 等 `CC_SWITCH_*` 命名风格一致。

### 决策 2：保持 `cwd = 项目根目录`，但把 `app.log` 改写到 `~/.cc-switch-tui/`

**原因**：

- 如果 `cwd = ~/.cc-switch-tui`，`script` 必须写绝对路径，换电脑 / 换用户名时配置会失效。
- 保持 `cwd = 项目根目录` 可以让 `script` 用相对路径 `./target/release/cc-switch-tui`，PM2 配置可随仓库迁移。
- 通过代码改动把日志写到 `~/.cc-switch-tui/`，同样实现了运行态产物收敛，且不牺牲配置可移植性。

### 决策 3：构建与运行分离

**原因**：

- `cargo build --release` 在 Rust 项目上耗时较长，不适合作为 PM2 每次启动的前置步骤。
- 用户明确希望手动控制构建时机（例如前端更新后需要 `make web-build && cargo build --release`）。
- PM2 只负责进程生命周期管理，符合其设计定位。

### 决策 4：`ecosystem.config.js` 使用 CommonJS 格式

**原因**：

- 项目根目录没有 `package.json` 的 `type: "module"`，使用 CommonJS 是最不引入假设的做法。
- PM2 对 `.js` 配置文件兼容 CommonJS 与 ESM，CommonJS 更通用。

---

## 不做 (Non-Goals)

- 不将 release 二进制安装 / 复制到 `~/.cc-switch-tui/`。
- 不修改前端构建流程或 `web-dist/` 的嵌入方式。
- 不新增持久化设置项来关闭浏览器；仅使用一次性环境变量开关。
- 不改动 `~/.zshrc` 的 source 行注入逻辑。
- 不动 `ys-proxy` / `cl-*` aliases / `oc-*` aliases 的生成逻辑。

---

## 验收方法 (Definition of Done)

1. 代码质量门：
   - `cargo test` 全部通过
   - `cargo build --release` 成功
   - `cargo clippy` 无新增 warning

2. 功能验证：
   ```bash
   # 构建
   cargo build --release

   # PM2 启动
   pm2 start ecosystem.config.js

   # 健康检查
   curl http://127.0.0.1:7480/api/health
   # 期望返回 {"status":"ok", ...}

   # 确认没有浏览器弹窗行为（在 headless / 无 GUI 环境也适用）
   # 确认日志写入 ~/.cc-switch-tui/app.log
   tail ~/.cc-switch-tui/app.log

   # 开机自启
   pm2 save
   pm2 startup

   # PM2 状态
   pm2 status cc-switch-tui
   ```

3. 文档一致性：
   - `README.md` 包含 PM2 部署命令。
   - `docs/codebase/ARCHITECTURE.md` 中 Logging 段落描述 `app.log` 位于 `~/.cc-switch-tui/`。

---

## 上下文引用

- 入口文件：`src/main.rs`
- 数据目录解析：`src/data_migration.rs::default_cc_dir()`
- 端口逻辑：`src/port.rs`
- 架构文档：`docs/codebase/ARCHITECTURE.md`（Main Binary / Logging 段落）
- 用户本地 PM2 状态：已运行 `cc-view`、`subconverter-clash` 两个常驻进程
