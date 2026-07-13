# Code Review: PM2 常驻进程支持

**Reviewed commit:** `feat/pm2-resident-process` @ `2a96b72`
**Review scope:** `src/main.rs`, `ecosystem.config.js`, `README.md`, `docs/codebase/ARCHITECTURE.md`（含关联的 intent/spec/plan 文档）
**Reviewer:** 当前 agent（自审 / 质量门）

---

## 1. 正确性 (Correctness)

### 结论
代码行为与 spec 一致，核心开关逻辑覆盖完整。

### 细节

- ✅ `src/main.rs:11-21`：`is_no_open_enabled` 严格只认 `1` / `true`（大小写不敏感），符合 spec 约定。
- ✅ `src/main.rs:116-119`：自动开浏览器逻辑正确叠加 `settings.auto_open_browser && !is_no_open()`，env var 优先生效。
- ✅ `src/main.rs:26-36`：日志目录在日志初始化前创建，路径收敛到 `default_cc_dir()/app.log`。
- ✅ `ecosystem.config.js`：单实例 fork 模式、`CC_SWITCH_NO_OPEN=1`、`autorestart: true` 均正确。

### 发现

**Important — 验证覆盖缺口**

`cargo test --lib` 不会运行 `src/main.rs` 中的 `#[cfg(test)]` 模块（因为 `main.rs` 属于 binary target）。

- 现象：build 阶段只跑了 `cargo test --lib`，新增的 `test_is_no_open_enabled` 实际未被执行。
- 建议：后续验证应使用 `cargo test --bin cc-switch-tui` 或 `cargo test`。
- 已补测：`cargo test --bin cc-switch-tui` 通过（1 passed）。

**Important — 既有集成测试失败（非本次引入）**

`cargo test --test opencode_test` 在 `main` 分支和本分支均失败：

```
test_get_opencode_config_returns_json
expected 200, got 500
{"error":{"code":"INTERNAL_ERROR","message":"内部错误: instance has no opencode config (missing fields)"}}
```

- 根因：测试用例仍使用旧 model id `"MiniMax-M3"`，而模板已改为 `"MiniMax-M3[1m]"`（前期 feature 遗留）。
- 与本次改动无关，但意味着当前 `cargo test` 全量无法通过。建议单独提一个修复测试用例的 commit。

---

## 2. 可读性 (Readability)

### 结论
命名清晰，逻辑直接，注释与项目风格一致。

### 细节

- ✅ `is_no_open_enabled` / `is_no_open` 命名直观，与 env var 名对应。
- ✅ `src/main.rs:25-26` 注释说明了 `cc_dir` 的提前创建原因。
- ✅ `src/main.rs:104-105` 注释明确 env var 可覆盖 settings。
- ✅ 测试用例分组（启用 / 未启用 / 其他字符串）清楚。

### 建议

**Suggestion — `is_no_open_enabled` 可进一步内联**

当前 `is_no_open` 是对 `is_no_open_enabled` 的薄包装。若未来只有一个调用点，可直接内联；目前保留可提高单测可测试性，无需改动。

---

## 3. 架构 (Architecture)

### 结论
改动最小化，未引入新依赖或新抽象，符合项目模式。

### 细节

- ✅ 复用 `src/data_migration.rs::default_cc_dir()`，不新增目录解析逻辑。
- ✅ 不引入 `clap` 等命令行解析库，符合 spec 决策。
- ✅ `ecosystem.config.js` 使用 CommonJS，与无 `package.json` 的项目根目录兼容。
- ✅ 日志路径改动与数据目录收敛，方向正确。

### 建议

**Suggestion — 未来若 env var 增多，考虑集中管理**

目前 `CC_SWITCH_NO_OPEN`、`CC_SWITCH_QUIET`、`CC_SWITCH_PROXY_URL` 分散在各自消费处读取。若后续继续增加运行时开关，可抽一个 `src/config.rs` 小模块统一读取与文档化。本次无需改动。

---

## 4. 安全 (Security)

### 结论
无新增安全风险。

### 细节

- ✅ `CC_SWITCH_NO_OPEN` 仅影响浏览器是否打开，无权限、注入或数据泄露风险。
- ✅ `ecosystem.config.js` 不包含 secrets、tokens 或硬编码凭证。
- ✅ 日志路径仍限定在 `~/.cc-switch-tui/`（或 fallback 到当前目录 `.cc-switch-tui/`），未引入任意目录写入。
- ✅ 未处理不可信用户输入。

---

## 5. 性能 (Performance)

### 结论
无性能回归。

### 细节

- ✅ `std::env::var` 只在启动时调用一次。
- ✅ `create_dir_all` 只在启动时调用一次。
- ✅ 日志写入方式与之前一致，仅路径改变。
- ✅ 无循环、无 N+1、无大对象分配。

---

## 6. 边界与测试 (Boundaries & Verification)

### 已验证

- `cargo test --lib`：92 passed（1 ignored）
- `cargo test --bin cc-switch-tui`：1 passed（新增测试）
- `cargo build --release`：成功
- 手动 release 二进制测试：
  - `CC_SWITCH_NO_OPEN=1 ./target/release/cc-switch-tui` 正常启动
  - `curl http://127.0.0.1:7480/api/health` 返回 200
  - 日志写入 `~/.cc-switch-tui/app.log`
  - 项目根目录无 `app.log`
- PM2 smoke test：
  - `pm2 start ecosystem.config.js` online
  - `pkill` 后 PM2 自动重启恢复 online
  - `pm2 stop/delete` 正常

### 未通过 / 需关注

- `cargo test --test opencode_test`：2 failed（既有问题，main 分支同样失败）
- `cargo clippy --all-targets -- -D warnings`：因项目既有 warning 失败，本次改动未在 `src/main.rs` 引入新 warning（已单独确认）。

---

## 7. 其他建议

**Suggestion — `.gitignore` 中 `/app.log` 已成为死条目**

项目根目录不再生成 `app.log`，`/app.log` 在 `.gitignore` 中已无实际作用。可在后续 cleanup 中移除，但当前 harmless。

**Suggestion — 考虑 `CC_SWITCH_NO_OPEN` 的空白字符容忍**

当前实现要求精确匹配 `1` / `true`。若用户从 shell 传 `"1 "` 或 `"true\n"`，会被视为未启用。这符合“严格语义”的设计，但如果希望更宽容，可在读取 env var 后 `trim()`。本次无需改动。

---

## 8. 审查结论

| 维度 | 评级 | 说明 |
|---|---|---|
| 正确性 | ✅ 通过 | 行为符合 spec；发现验证覆盖缺口，已补测 |
| 可读性 | ✅ 通过 | 命名、注释、结构均清晰 |
| 架构 | ✅ 通过 | 最小改动，复用现有抽象 |
| 安全 | ✅ 通过 | 无新增风险 |
| 性能 | ✅ 通过 | 无回归 |

**总体结论：Approve with notes。**

本次改动满足 intent/spec 要求，可继续进入 `/ship` 阶段。建议 ship 前单独处理 `opencode_test` 的既有失败（非本次阻塞，但会影响全量测试信号）。
