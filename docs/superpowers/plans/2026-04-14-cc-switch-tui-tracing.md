# cc-switch-tui tracing 日志实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 cc-switch-tui 引入 tracing 日志系统，默认输出到 `app.log`，通过 `RUST_LOG` 控制级别，覆盖 main.rs、dao、app state 和 ui 渲染的关键埋点。

**Architecture:** 使用 `tracing` + `tracing-subscriber` 的 fmt layer，配置 `EnvFilter` 支持环境变量覆写。日志写入当前目录 `app.log`（append 模式），无 ANSI 颜色。在业务逻辑层和 TUI 层的关键函数中插入 `info!`/`debug!`/`trace!` 埋点。

**Tech Stack:** Rust, tracing 0.1, tracing-subscriber 0.3

> **注意：** 请严格遵守 `.claude/rules/code.md`：最小化代码、不引入未请求的功能、只修改必要文件。

---

## 文件结构

- `Cargo.toml` — 新增 `tracing` 和 `tracing-subscriber` 依赖
- `src/main.rs` — 在 `main()` 开头初始化 tracing subscriber
- `src/dao/memory_impl.rs` — 在 create/update/delete 埋点
- `src/app/state.rs` — 在 on_key、submit_create、handle_edit、handle_delete_confirm、handle_list 埋点
- `src/ui/mod.rs` — 在 `draw()` 中埋点

---

### Task 1: 添加 tracing 依赖

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: 添加 tracing 和 tracing-subscriber 依赖**

  在 `Cargo.toml` 的 `[dependencies]` 段追加：

  ```toml
  tracing = "0.1"
  tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }
  ```

- [ ] **Step 2: 运行 cargo check 确认依赖下载和编译正常**

  运行：`cargo check`
  预期：成功完成，无错误

- [ ] **Step 3: Commit**

  ```bash
  git add Cargo.toml
  git commit -m "deps: add tracing and tracing-subscriber"
  ```

---

### Task 2: 在 main.rs 初始化 tracing subscriber

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: 在 main() 开头插入 subscriber 初始化代码**

  把 `src/main.rs` 的 `main()` 函数修改为：

  ```rust
  fn main() -> io::Result<()> {
      let log_file = std::fs::OpenOptions::new()
          .create(true)
          .append(true)
          .open("app.log")
          .expect("无法创建日志文件");

      tracing_subscriber::fmt()
          .with_writer(move || log_file.try_clone().unwrap())
          .with_env_filter(
              tracing_subscriber::EnvFilter::try_from_default_env()
                  .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("INFO"))
          )
          .with_ansi(false)
          .with_target(true)
          .init();

      tracing::info!("cc-switch-tui starting");

      enable_raw_mode()?;
      let mut stdout = io::stdout();
      crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
      let backend = CrosstermBackend::new(stdout);
      let mut terminal = Terminal::new(backend)?;

      let mut app = App::new();
      let res = run_app(&mut terminal, &mut app);

      disable_raw_mode()?;
      crossterm::execute!(
          terminal.backend_mut(),
          LeaveAlternateScreen,
          DisableMouseCapture
      )?;
      terminal.show_cursor()?;

      if let Err(e) = res {
          eprintln!("Error: {}", e);
      }

      tracing::info!("cc-switch-tui exiting");
      Ok(())
  }
  ```

  同时确认文件顶部已有这些 import（如果没有请添加）：
  ```rust
  use cc_switch_tui::app::state::App;
  use cc_switch_tui::ui;
  use crossterm::{
      event::{self, DisableMouseCapture, EnableMouseCapture, Event},
      terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
  };
  use ratatui::backend::CrosstermBackend;
  use ratatui::Terminal;
  use std::io;
  use std::time::Duration;
  ```

- [ ] **Step 2: 运行 cargo check 确认编译正常**

  运行：`cargo check`
  预期：成功完成，无警告

- [ ] **Step 3: Commit**

  ```bash
  git add src/main.rs
  git commit -m "feat(tracing): initialize subscriber in main"
  ```

---

### Task 3: 在 Dao 层埋点

**Files:**
- Modify: `src/dao/memory_impl.rs`

- [ ] **Step 1: 在 create_instance 中添加 info 日志**

  在 `src/dao/memory_impl.rs` 的 `create_instance` 方法中，在 `Ok(())` 返回前添加：

  ```rust
  tracing::info!("dao create_instance: id={}", instance.id);
  ```

  完整函数段如下（仅展示需修改的部分）：

  ```rust
  fn create_instance(&mut self, instance: ProviderInstance) -> Result<(), AppError> {
      if self.instances.contains_key(&instance.id) {
          return Err(AppError::InstanceAlreadyExists(instance.id.clone()));
      }
      self.instances.insert(instance.id.clone(), instance);
      tracing::info!("dao create_instance: id={}", instance.id);
      Ok(())
  }
  ```

  注意：由于 `instance` 已被 move 进 `insert`，你需要在 `insert` 之前记录 id，或者调整顺序为：

  ```rust
  fn create_instance(&mut self, instance: ProviderInstance) -> Result<(), AppError> {
      if self.instances.contains_key(&instance.id) {
          return Err(AppError::InstanceAlreadyExists(instance.id.clone()));
      }
      let id = instance.id.clone();
      self.instances.insert(id.clone(), instance);
      tracing::info!("dao create_instance: id={}", id);
      Ok(())
  }
  ```

- [ ] **Step 2: 在 update_instance 中添加 info 日志**

  修改为：

  ```rust
  fn update_instance(&mut self, id: &str, api_key: String) -> Result<(), AppError> {
      let instance = self.instances.get_mut(id)
          .ok_or_else(|| AppError::InstanceNotFound(id.to_string()))?;
      instance.api_key = api_key;
      tracing::info!("dao update_instance: id={}", id);
      Ok(())
  }
  ```

- [ ] **Step 3: 在 delete_instance 中添加 info 日志**

  修改为：

  ```rust
  fn delete_instance(&mut self, id: &str) -> Result<(), AppError> {
      if self.instances.remove(id).is_none() {
          return Err(AppError::InstanceNotFound(id.to_string()));
      }
      tracing::info!("dao delete_instance: id={}", id);
      if self.current_instance_id.as_deref() == Some(id) {
          self.current_instance_id = None;
      }
      Ok(())
  }
  ```

- [ ] **Step 4: 运行 cargo check 确认编译正常**

  运行：`cargo check`
  预期：成功完成，无警告

- [ ] **Step 5: Commit**

  ```bash
  git add src/dao/memory_impl.rs
  git commit -m "feat(tracing): add logs to MemoryDaoImpl"
  ```

---

### Task 4: 在 App 状态机埋点

**Files:**
- Modify: `src/app/state.rs`

- [ ] **Step 1: 在 on_key 中添加 debug 日志**

  把 `on_key` 方法修改为：

  ```rust
  pub fn on_key(&mut self, key: KeyEvent) {
      self.error_message = None;
      tracing::debug!("key_event = {:?}", key);
      match &self.state.clone() {
          AppState::List => self.handle_list(key),
          AppState::CreateProvider => self.handle_create_provider(key),
          AppState::CreateModel { .. } => self.handle_create_model(key),
          AppState::CreateApiKey { .. } => self.handle_create_api_key(key),
          AppState::Edit { .. } => self.handle_edit(key),
          AppState::DeleteConfirm { .. } => self.handle_delete_confirm(key),
      }
  }
  ```

- [ ] **Step 2: 在 handle_list 的 n/e/d/q 分支中添加 debug 日志**

  把 `handle_list` 方法中的对应分支修改为：

  ```rust
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
          tracing::debug!("state transition: List -> Edit({})", instance.id);
          let api_key = instance.api_key.clone();
          let instance_id = instance.id.clone();
          self.edit_input = InputState::new(api_key);
          self.state = AppState::Edit { instance_id };
      }
  }
  KeyCode::Char('d') => {
      if let Some(instance) = self.current_instance() {
          tracing::debug!("state transition: List -> DeleteConfirm({})", instance.id);
          self.state = AppState::DeleteConfirm { instance_id: instance.id.clone() };
      }
  }
  ```

- [ ] **Step 3: 在 submit_create 中添加 info 日志**

  把 `submit_create` 中的 `Ok(())` 分支修改为：

  ```rust
  Ok(()) => {
      tracing::info!("create instance success: id={}", id);
      self.state = AppState::List;
      self.list_index = self.get_sorted_instances().len().saturating_sub(1);
  }
  ```

- [ ] **Step 4: 在 handle_edit 中添加 info 日志**

  把 `handle_edit` 中的 `KeyCode::Enter` 分支修改为：

  ```rust
  KeyCode::Enter => {
      if let AppState::Edit { instance_id } = self.state.clone() {
          if let Err(e) = self.dao.update_instance(&instance_id, self.edit_input.value.clone()) {
              self.error_message = Some(e.to_string());
          } else {
              tracing::info!("update api_key: id={}", instance_id);
          }
          self.state = AppState::List;
      }
  }
  ```

- [ ] **Step 5: 在 handle_delete_confirm 中添加 info 日志**

  把 `handle_delete_confirm` 中的确认分支修改为：

  ```rust
  KeyCode::Char('y') | KeyCode::Char('Y') => {
      if let AppState::DeleteConfirm { instance_id } = self.state.clone() {
          if let Err(e) = self.dao.delete_instance(&instance_id) {
              self.error_message = Some(e.to_string());
          } else {
              tracing::info!("delete instance: id={}", instance_id);
              let instances = self.get_sorted_instances();
              if self.list_index >= instances.len() && self.list_index > 0 {
                  self.list_index -= 1;
              }
          }
          self.state = AppState::List;
      }
  }
  ```

- [ ] **Step 6: 运行 cargo check 确认编译正常**

  运行：`cargo check`
  预期：成功完成，无警告

- [ ] **Step 7: Commit**

  ```bash
  git add src/app/state.rs
  git commit -m "feat(tracing): add logs to App state machine"
  ```

---

### Task 5: 在 UI 渲染层埋点

**Files:**
- Modify: `src/ui/mod.rs`

- [ ] **Step 1: 在 draw 函数中添加 trace 日志**

  把 `src/ui/mod.rs` 的 `draw` 函数修改为：

  ```rust
  pub fn draw(frame: &mut Frame, app: &App) {
      tracing::trace!("render frame, state={:?}", app.state);
      match &app.state {
          AppState::List => list::draw_list(frame, app),
          AppState::CreateProvider
          | AppState::CreateModel { .. }
          | AppState::CreateApiKey { .. } => {
              list::draw_list(frame, app);
              create::draw_create(frame, app);
          }
          AppState::Edit { .. } => {
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

- [ ] **Step 2: 运行 cargo check 确认编译正常**

  运行：`cargo check`
  预期：成功完成，无警告

- [ ] **Step 3: Commit**

  ```bash
  git add src/ui/mod.rs
  git commit -m "feat(tracing): add trace log to ui draw"
  ```

---

### Task 6: 运行完整测试并验证日志输出

**Files:**
- 无新增文件

- [ ] **Step 1: 运行 cargo test 确认全部测试通过**

  运行：`cargo test`
  预期：19 个测试全部 PASS

- [ ] **Step 2: 运行 cargo build 确认编译通过**

  运行：`cargo build`
  预期：成功完成

- [ ] **Step 3: 运行 cargo run 并验证 app.log 生成**

  运行：`cargo run`
  在 TUI 出现后按 `q` 退出，然后检查当前目录是否生成了 `app.log`，内容应包含：
  - `cc-switch-tui starting`
  - `cc-switch-tui exiting`

- [ ] **Step 4: 使用 RUST_LOG=debug 验证更多日志**

  运行：`RUST_LOG=debug cargo run`
  按 `n` 进入创建向导后按 `Esc` 返回，再按 `q` 退出。
  检查 `app.log` 应包含 `key_event = ...` 和 `state transition: ...` 等 debug 日志。

- [ ] **Step 5: Commit**

  ```bash
  git commit --allow-empty -m "test: verify tracing logs output correctly"
  ```

---

## Self-Review

**1. Spec coverage:**
- ✅ Cargo.toml 依赖（Task 1）
- ✅ main.rs subscriber 初始化 + 启动/退出日志（Task 2）
- ✅ Dao 层 create/update/delete 埋点（Task 3）
- ✅ App 状态机 on_key / submit_create / handle_edit / handle_delete_confirm / handle_list 埋点（Task 4）
- ✅ UI draw trace 埋点（Task 5）
- ✅ 手动验证 app.log 输出（Task 6）

**2. Placeholder scan:**
- 无 TBD、TODO 等占位符

**3. Type一致性：**
- `tracing::info!` / `debug!` / `trace!` 使用正确
- `EnvFilter` 初始化路径一致

---

## 执行方式

Plan complete and saved to `docs/superpowers/plans/2026-04-14-cc-switch-tui-tracing.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints for review

Which approach?
