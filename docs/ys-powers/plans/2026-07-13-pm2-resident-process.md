# Implementation Plan: 将 cc-switch-tui 封装为 PM2 常驻进程

> 上游 spec：[docs/ys-powers/specs/2026-07-13-pm2-resident-process-design.md](../specs/2026-07-13-pm2-resident-process-design.md)
> 上游 intent：[docs/ys-powers/intent/pm2-resident-process.md](../intent/pm2-resident-process.md)
> 工作分支：`feat/pm2-resident-process`（已创建）
> 涉及文件：~5 个文件

## Overview

本次改动目标很小：让 `cc-switch-tui` 能在 PM2 下常驻运行。核心只需要解决三个问题：

1. 启动时自动打开浏览器的行为需要可关闭。
2. 日志文件不应再依赖当前工作目录。
3. 提供 PM2 配置文件与使用文档。

**关键约束**：
- 不引入新依赖（不引入 `clap` 等命令行解析库）。
- 不修改端口 7480、数据目录 `~/.cc-switch-tui/`、前端构建流程。
- 构建与运行分离：PM2 不自动触发 `cargo build --release`。

## Architecture Decisions

### 关键决策 1：环境变量开关优先于命令行参数

`CC_SWITCH_NO_OPEN=1` 比 `--no-open` 更适合 PM2：
- `ecosystem.config.js` 原生支持 `env` 字段。
- 当前二进制无命令行参数解析，引入 `clap` 是过度设计。
- 与现有 `CC_SWITCH_QUIET`、`CC_SWITCH_PROXY_URL` 命名风格一致。

### 关键决策 2：保持 `cwd = 项目根目录`，通过代码改动把日志收敛到 `~/.cc-switch-tui/`

- 若 `cwd = ~/.cc-switch-tui`，`script` 必须写绝对路径，换电脑/换用户名时配置失效。
- 保持 `cwd = 项目根目录` 可使用相对路径 `./target/release/cc-switch-tui`，PM2 配置可随仓库迁移。
- 日志路径改动只影响 `src/main.rs` 的初始化逻辑，范围最小。

### 关键决策 3：单实例 fork 模式

后端固定监听 7480，不能 cluster / 多实例。`ecosystem.config.js` 使用 `exec_mode: 'fork'` + `instances: 1`。

---

## Dependency Graph

```
                    (无依赖起点)
                         │
         ┌───────────────┴───────────────┐
         ▼                               ▼
   Task 1: main.rs              Task 2: main.rs
   CC_SWITCH_NO_OPEN 支持        日志路径收敛到 ~/.cc-switch-tui/
         │                               │
         └───────────────┬───────────────┘
                         ▼
              Checkpoint A: cargo test / cargo build --release
                         │
                         ▼
              Task 3: ecosystem.config.js
              Task 4: README.md 更新
              Task 5: ARCHITECTURE.md 更新
                         │
                         ▼
              Checkpoint B: 配置与文档 review
                         │
                         ▼
              Task 6: PM2 手动 smoke test
                         │
                         ▼
              Checkpoint C: 全部验收标准通过
```

---

## Task List

### Phase 1: Rust 入口改动

#### Task 1: 新增 `CC_SWITCH_NO_OPEN` 环境变量支持

**Description**：在 `src/main.rs` 中新增私有 helper `is_no_open()`，并在自动打开浏览器逻辑前使用它。当环境变量值为 `1` 或 `true`（大小写不敏感）时，跳过 `webbrowser::open`。

**Acceptance criteria**:
- [ ] `src/main.rs` 中新增 `is_no_open()` helper，语义只认 `1` / `true`（大小写不敏感）。
- [ ] 自动开浏览器逻辑改为 `s.auto_open_browser && !is_no_open()`。
- [ ] 新增 `#[cfg(test)]` 单元测试覆盖 `1` / `true` / `True` / `0` / `false` / 未设置 等场景。
- [ ] 测试在每次用例前后清理环境变量，避免串扰。

**Verification**:
- [ ] `cargo test --lib` 通过（含新增 helper 测试）。
- [ ] `cargo clippy --all-targets -- -D warnings` 无新增 warning。

**Dependencies**: None

**Files**:
- `src/main.rs`

**Estimated scope**: S（1 个文件，新增 helper + 测试）

---

#### Task 2: 将 `app.log` 收敛到 `~/.cc-switch-tui/`

**Description**：把 `src/main.rs` 中日志初始化从 `"app.log"` 改为 `default_cc_dir().join("app.log")`，并在打开日志文件前确保 `~/.cc-switch-tui` 目录存在。后续 `ensure_data_migrated` 和 DAO 初始化复用同一个 `cc_dir`。

**Acceptance criteria**:
- [ ] 日志文件路径改为 `default_cc_dir().join("app.log")`。
- [ ] 在 `OpenOptions::open` 之前调用 `std::fs::create_dir_all(&cc_dir)`。
- [ ] 原 `"app.log"` 字符串不再出现在 `src/main.rs` 中。
- [ ] 项目根目录在后续运行中不再生成新的 `app.log`。

**Verification**:
- [ ] `cargo build --release` 成功。
- [ ] 手动跑一次 `./target/release/cc-switch-tui`（设置 `CC_SWITCH_NO_OPEN=1`），确认 `~/.cc-switch-tui/app.log` 生成且项目根目录无 `app.log`。

**Dependencies**: Task 1（建议先合入 env var 支持，避免 Task 2 单独跑 main 时弹浏览器；功能上无强依赖）

**Files**:
- `src/main.rs`

**Estimated scope**: S（1 个文件，改动初始化顺序）

---

### Checkpoint A: 代码改动验证

- [ ] `cargo test --lib` 全部通过
- [ ] `cargo clippy --all-targets -- -D warnings` 无新增 warning
- [ ] `cargo build --release` 成功
- [ ] 手动运行 release 二进制：`CC_SWITCH_NO_OPEN=1 ./target/release/cc-switch-tui`，确认不弹浏览器、日志写到 `~/.cc-switch-tui/app.log`

---

### Phase 2: 配置与文档

#### Task 3: 新增 `ecosystem.config.js`

**Description**：在项目根目录创建 PM2 配置文件，使用 CommonJS 格式，配置单实例 fork 模式，默认传入 `CC_SWITCH_NO_OPEN=1` 与 `RUST_LOG=INFO`。

**Acceptance criteria**:
- [ ] 文件位于项目根目录 `ecosystem.config.js`。
- [ ] `name: 'cc-switch-tui'`。
- [ ] `script: './target/release/cc-switch-tui'`。
- [ ] `cwd: '.'`。
- [ ] `exec_mode: 'fork'`，`instances: 1`。
- [ ] `autorestart: true`。
- [ ] `env.CC_SWITCH_NO_OPEN = '1'`。
- [ ] 可选项：`max_restarts: 10`，`min_uptime: '5s'`。

**Verification**:
- [ ] `pm2 start ecosystem.config.js` 成功启动进程。
- [ ] `pm2 status cc-switch-tui` 显示 online。
- [ ] `curl http://127.0.0.1:7480/api/health` 返回 200。

**Dependencies**: Task 1, Task 2

**Files**:
- `ecosystem.config.js`

**Estimated scope**: XS（1 个新文件）

---

#### Task 4: 更新 `README.md`

**Description**：在 `README.md` 中新增「PM2 常驻运行」小节，说明构建前置、启动、停止、重启、查看日志、设置开机自启等命令。

**Acceptance criteria**:
- [ ] `README.md` 中新增 PM2 常驻运行说明（建议在「快速开始」之后或「相关文档」之前）。
- [ ] 包含 `cargo build --release` 前置步骤。
- [ ] 包含 `pm2 start ecosystem.config.js`。
- [ ] 包含 `pm2 stop ecosystem.config.js` / `pm2 restart ecosystem.config.js`。
- [ ] 包含 `pm2 logs cc-switch-tui`。
- [ ] 包含 `pm2 save` + `pm2 startup` 开机自启说明。
- [ ] 说明启动时不会自动打开浏览器（由 `CC_SWITCH_NO_OPEN` 控制）。

**Verification**:
- [ ] 在浏览器 / markdown 渲染中查看新增段落格式正确。
- [ ] 命令可复制执行。

**Dependencies**: Task 3

**Files**:
- `README.md`

**Estimated scope**: XS（1 个文件，文档段落）

---

#### Task 5: 更新 `docs/codebase/ARCHITECTURE.md`

**Description**：同步更新 `ARCHITECTURE.md` 中 Main Binary 与 Logging 段落关于 `app.log` 位置的描述，从「当前工作目录」改为「`~/.cc-switch-tui/app.log`」。

**Acceptance criteria**:
- [ ] Main Binary responsibilities 第 1 条描述更新为 `~/.cc-switch-tui/app.log`。
- [ ] Logging 跨章节段落中「Log destination is `app.log` in the working directory」更新为 `~/.cc-switch-tui/app.log`。
- [ ] Cross-Cutting Concerns / Logging 的 `Output` 字段同步更新。

**Verification**:
- [ ] `grep -n "working directory" docs/codebase/ARCHITECTURE.md` 不再命中与 `app.log` 相关的旧描述。
- [ ] 全文对 `app.log` 位置的描述一致。

**Dependencies**: Task 2

**Files**:
- `docs/codebase/ARCHITECTURE.md`

**Estimated scope**: XS（1 个文件，3 处描述）

---

### Checkpoint B: 配置与文档 review

- [ ] `ecosystem.config.js` 存在且格式正确
- [ ] `README.md` 新增 PM2 说明段落可读、命令正确
- [ ] `ARCHITECTURE.md` 中 `app.log` 位置描述已更新
- [ ] 三个文件均无拼写或格式错误

---

### Phase 3: 集成验证

#### Task 6: PM2 手动 smoke test

**Description**：从干净状态开始，完整走一遍构建 → PM2 启动 → 健康检查 → 日志位置检查 → 自动重启验证 → 停止 的端到端流程。

**Acceptance criteria**:
- [ ] `cargo build --release` 产出干净的 `target/release/cc-switch-tui`。
- [ ] `pm2 start ecosystem.config.js` 拉起进程并显示 online。
- [ ] `curl http://127.0.0.1:7480/api/health` 返回 200。
- [ ] `~/.cc-switch-tui/app.log` 包含启动日志；项目根目录无 `app.log`。
- [ ] 杀掉进程后，PM2 能在数秒内自动重启并恢复 online。
- [ ] `pm2 stop ecosystem.config.js` / `pm2 delete ecosystem.config.js` 能正常停止并移除进程。

**Verification**:
```bash
# 1. 清理
rm -f app.log ~/.cc-switch-tui/app.log
pm2 delete ecosystem.config.js 2>/dev/null || true

# 2. 构建
cargo build --release

# 3. PM2 启动
pm2 start ecosystem.config.js

# 4. 验证在线
pm2 status cc-switch-tui

# 5. 健康检查
curl -s http://127.0.0.1:7480/api/health | grep '"status":"ok"'

# 6. 验证日志位置
ls app.log 2>/dev/null && echo "FAIL: 项目根目录不应有 app.log" || echo "OK: 项目根目录无 app.log"
tail -n 5 ~/.cc-switch-tui/app.log

# 7. 验证自动重启
pkill -f "target/release/cc-switch-tui"
sleep 5
pm2 status cc-switch-tui | grep online

# 8. 停止
pm2 stop ecosystem.config.js
pm2 delete ecosystem.config.js
```

**Dependencies**: Task 3, Task 4, Task 5

**Files**: 无（仅验证）

**Estimated scope**: S（手动端到端验证）

---

### Checkpoint C: 全部验收标准通过

- [ ] `cargo test --lib` 全部通过
- [ ] `cargo clippy --all-targets -- -D warnings` 无新增 warning
- [ ] `cargo build --release` 成功
- [ ] `pm2 start ecosystem.config.js` 成功启动并 online
- [ ] 启动时不弹浏览器
- [ ] 崩溃后 PM2 自动重启成功
- [ ] 日志落在 `~/.cc-switch-tui/app.log`，项目根目录无 `app.log`
- [ ] `curl http://127.0.0.1:7480/api/health` 返回 200
- [ ] `README.md` 包含 PM2 部署说明
- [ ] `ARCHITECTURE.md` 中 Logging 描述已更新
- [ ] `ecosystem.config.js`、`README.md`、`ARCHITECTURE.md` 已纳入 git 工作区

---

## Risks and Mitigations

| 风险 | 影响 | 缓解 |
|---|---|---|
| `CC_SWITCH_NO_OPEN` 语义写宽，导致用户意外禁用浏览器 | 中 | 只认 `1` / `true`，其他值视为 false |
| 日志目录创建失败导致启动 panic | 高 | 在打开日志前显式 `create_dir_all`；复用 `default_cc_dir()` |
| `ecosystem.config.js` 的 `cwd` 理解错误导致找不到二进制 | 中 | `cwd: '.'` 表示配置文件所在目录；`script` 用相对路径 |
| 单元测试串改环境变量影响其他测试 | 低 | 每个用例前后 `remove_var` / `set_var` 清理 |
| 端口 7480 被占用导致 PM2 启动失败 | 低 | 原逻辑已会报错；PM2 会重试，需用户自行解决冲突 |
| 文档中 app.log 位置描述不一致 | 低 | 同步更新 `ARCHITECTURE.md` 与 `README.md` |

---

## Open Questions

- 是否需要将 release 二进制安装 / 复制到 `~/.cc-switch-tui/` 以进一步解耦项目目录？（当前 spec 明确不做）
- 是否需要新增 `make pm2` 之类的 Makefile target 来封装 `cargo build --release && pm2 start ecosystem.config.js`？（当前 spec 不做，README 手动说明即可）
