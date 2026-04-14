# cc-switch-tui TUI 层实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 基于 ratatui + crossterm 实现 cc-switch-tui 的终端交互界面，支持实例列表展示、新建向导、编辑 api-key、删除确认。

**Architecture:** 采用事件驱动架构：主循环读取 crossterm 键盘事件，更新 App 状态机，再由 ratatui 渲染对应页面。数据层复用已有的 Dao trait 和 MemoryDaoImpl。

**Tech Stack:** Rust 2024 edition, ratatui 0.26, crossterm 0.27, chrono, thiserror

> **注意：** 请严格遵守 `.claude/rules/code.md` 中的代码规范：最小化代码、不引入未请求的功能、只修改必要文件、所有结构体必须带中文注释。

---

## 文件结构

```
src/
├── main.rs              # 入口：初始化终端并启动 TUI 主循环
├── lib.rs               # 导出所有模块
├── app.rs               # App 结构体、状态机、事件处理、页面路由
├── event.rs             # 键盘事件解析和转换
├── ui/
│   ├── mod.rs           # UI 渲染入口（根据 AppState 分发）
│   ├── list.rs          # 主界面：左侧列表 + 右侧信息面板 + 底部帮助栏
│   ├── create.rs        # 新建向导：Provider 选择、Model 选择、API Key 输入
│   ├── edit.rs          # 编辑页面：修改 API Key
│   └── popup.rs         # 删除确认对话框
├── dao/                 # 已有模块
├── domain/              # 已有模块
├── app/                 # 已有模块
└── shell.rs             # 已有模块
```

---

### Task 1: 添加 TUI 依赖

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: 添加 ratatui 和 crossterm 依赖**

  修改 `Cargo.toml` 的 `[dependencies]` 段：

  ```toml
  [dependencies]
  chrono = { version = "0.4", features = ["serde"] }
  crossterm = "0.27"
  ratatui = "0.26"
  thiserror = "1"
  ```

- [ ] **Step 2: 运行 cargo check 确认依赖下载和编译正常**

  运行：`cargo check`
  预期：成功完成，无错误

- [ ] **Step 3: Commit**

  ```bash
  git add Cargo.toml
  git commit -m "deps: add ratatui and crossterm"
  ```

---

### Task 2: 为 Dao trait 和 MemoryDaoImpl 添加 update_instance 方法

**Files:**
- Modify: `src/dao/mod.rs`
- Modify: `src/dao/memory_impl.rs`

- [ ] **Step 1: 修改 Dao trait，增加 update_instance 方法**

  在 `src/dao/mod.rs` 的 `Dao` trait 中新增方法：

  ```rust
  /// 更新实例的 API Key，如果实例不存在则返回错误
  fn update_instance(&mut self, id: &str, api_key: String) -> Result<(), AppError>;
  ```

  完整 trait 应变为：

  ```rust
  pub mod memory_impl;

  use crate::domain::{AppError, ProviderInstance, ProviderTemplate};

  /// 数据访问对象接口，抽象 provider 配置和实例的存储
  pub trait Dao {
      /// 获取所有内置 Provider 模板
      fn get_templates(&self) -> Vec<&ProviderTemplate>;

      /// 根据 ID 获取 Provider 模板
      fn get_template(&self, id: &str) -> Option<&ProviderTemplate>;

      /// 获取所有用户创建的实例
      fn list_instances(&self) -> Vec<&ProviderInstance>;

      /// 根据 ID 获取实例
      fn get_instance(&self, id: &str) -> Option<&ProviderInstance>;

      /// 创建实例，如果实例已存在则返回错误
      fn create_instance(&mut self, instance: ProviderInstance) -> Result<(), AppError>;

      /// 删除实例，如果实例不存在则返回错误
      fn delete_instance(&mut self, id: &str) -> Result<(), AppError>;

      /// 更新实例的 API Key，如果实例不存在则返回错误
      fn update_instance(&mut self, id: &str, api_key: String) -> Result<(), AppError>;

      /// 获取当前选中的实例
      fn get_current_instance(&self) -> Option<&ProviderInstance>;

      /// 设置当前选中的实例
      fn set_current_instance(&mut self, id: &str) -> Result<(), AppError>;
  }
  ```

- [ ] **Step 2: 在 MemoryDaoImpl 中实现 update_instance**

  在 `src/dao/memory_impl.rs` 的 `impl Dao for MemoryDaoImpl` 块中新增：

  ```rust
  fn update_instance(&mut self, id: &str, api_key: String) -> Result<(), AppError> {
      let instance = self.instances.get_mut(id)
          .ok_or_else(|| AppError::InstanceNotFound(id.to_string()))?;
      instance.api_key = api_key;
      Ok(())
  }
  ```

- [ ] **Step 3: 编写测试验证 update_instance**

  在 `tests/dao_test.rs` 末尾添加：

  ```rust
  #[test]
  fn test_update_instance() {
      let template = create_test_template();
      let mut dao = MemoryDaoImpl::new(vec![template]);
      let instance = create_test_instance();

      dao.create_instance(instance).unwrap();
      dao.update_instance("minimax-m1", "new-key".to_string()).unwrap();
      let updated = dao.get_instance("minimax-m1").unwrap();
      assert_eq!(updated.api_key, "new-key");
  }

  #[test]
  fn test_update_nonexistent_instance_fails() {
      let template = create_test_template();
      let mut dao = MemoryDaoImpl::new(vec![template]);

      let result = dao.update_instance("not-exist", "key".to_string());
      assert_eq!(result, Err(AppError::InstanceNotFound("not-exist".to_string())));
  }
  ```

- [ ] **Step 4: 运行 cargo test --test dao_test 确认通过**

  运行：`cargo test --test dao_test`
  预期：9 个测试全部 PASS

- [ ] **Step 5: Commit**

  ```bash
  git add src/dao/ tests/dao_test.rs
  git commit -m "feat(dao): add update_instance for api-key editing"
  ```

---

### Task 3: 实现 event.rs 键盘事件解析

**Files:**
- Create: `src/event.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: 创建 event.rs**

  ```rust
  use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};

  /// 应用级别的键盘事件，屏蔽掉重复的 KeyRelease 事件
  #[derive(Debug, Clone, PartialEq)]
  pub enum AppEvent {
      /// 键盘按下事件
      Key(KeyEvent),
      /// 终端大小改变事件
      Resize(u16, u16),
      /// 无事件（轮询超时）
      Tick,
  }

  /// 从 crossterm 的 Event 转换为 AppEvent
  pub fn parse_event(event: Event) -> Option<AppEvent> {
      match event {
          Event::Key(key) if key.kind == KeyEventKind::Press => Some(AppEvent::Key(key)),
          Event::Resize(w, h) => Some(AppEvent::Resize(w, h)),
          _ => None,
      }
  }
  ```

- [ ] **Step 2: 修改 lib.rs 导出 event 模块**

  在 `src/lib.rs` 中添加：

  ```rust
  pub mod app;
  pub mod dao;
  pub mod domain;
  pub mod event;
  pub mod shell;
  ```

- [ ] **Step 3: 运行 cargo check 确认编译正常**

  运行：`cargo check`
  预期：成功完成

- [ ] **Step 4: Commit**

  ```bash
  git add src/event.rs src/lib.rs
  git commit -m "feat(event): add keyboard event parsing"
  ```

---

### Task 4: 实现 app.rs App 状态机和事件循环

**Files:**
- Create: `src/app.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: 创建 app.rs 的 AppState 枚举和 App 结构体**

  ```rust
  use crate::app::templates::register_templates;
  use crate::dao::memory_impl::MemoryDaoImpl;
  use crate::dao::Dao;
  use crate::domain::{AppError, ProviderInstance, ProviderTemplate};
  use std::io;

  /// 应用当前所处的页面状态
  #[derive(Debug, Clone, PartialEq)]
  pub enum AppState {
      /// 主界面：实例列表 + 信息面板
      List,
      /// 新建向导：选择 Provider
      CreateProvider,
      /// 新建向导：选择 Model
      CreateModel { template_id: String },
      /// 新建向导：输入 API Key
      CreateApiKey { template_id: String, model_id: String },
      /// 编辑页面
      Edit { instance_id: String },
      /// 删除确认对话框
      DeleteConfirm { instance_id: String },
  }

  /// 输入框状态，用于 API Key 输入和编辑
  #[derive(Debug, Clone, PartialEq)]
  pub struct InputState {
      /// 当前输入的字符串
      pub value: String,
      /// 光标在字符串中的位置（字符索引）
      pub cursor: usize,
  }

  impl InputState {
      /// 使用默认值创建输入状态
      pub fn new(value: String) -> Self {
          let cursor = value.chars().count();
          Self { value, cursor }
      }

      /// 在光标位置插入一个字符
      pub fn insert_char(&mut self, c: char) {
          let byte_pos = self.value.char_indices()
              .nth(self.cursor)
              .map(|(i, _)| i)
              .unwrap_or(self.value.len());
          self.value.insert(byte_pos, c);
          self.cursor += 1;
      }

      /// 删除光标前一个字符
      pub fn backspace(&mut self) {
          if self.cursor == 0 {
              return;
          }
          let byte_pos = self.value.char_indices()
              .nth(self.cursor - 1)
              .map(|(i, _)| i)
              .unwrap_or(0);
          let next_byte_pos = self.value.char_indices()
              .nth(self.cursor)
              .map(|(i, _)| i)
              .unwrap_or(self.value.len());
          self.value.drain(byte_pos..next_byte_pos);
          self.cursor -= 1;
      }

      /// 光标左移
      pub fn move_left(&mut self) {
          if self.cursor > 0 {
              self.cursor -= 1;
          }
      }

      /// 光标右移
      pub fn move_right(&mut self) {
          if self.cursor < self.value.chars().count() {
              self.cursor += 1;
          }
      }
  }

  /// 应用主结构体，包含 Dao、状态、输入、列表索引等
  pub struct App {
      /// 数据访问对象
      pub dao: MemoryDaoImpl,
      /// 当前页面状态
      pub state: AppState,
      /// 实例列表中当前高亮的索引
      pub list_index: usize,
      /// 创建向导中 Provider 列表的索引
      pub provider_index: usize,
      /// 创建向导中 Model 列表的索引
      pub model_index: usize,
      /// API Key 输入状态
      pub api_key_input: InputState,
      /// 编辑时的输入状态
      pub edit_input: InputState,
      /// 错误消息（显示在主界面底部）
      pub error_message: Option<String>,
      /// 是否退出应用
      pub should_quit: bool,
  }

  impl App {
      /// 创建新的 App 实例
      pub fn new() -> Self {
          let templates = register_templates();
          Self {
              dao: MemoryDaoImpl::new(templates),
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

      /// 获取所有已创建的实例，按模板顺序分组排列
      pub fn get_sorted_instances(&self) -> Vec<&ProviderInstance> {
          let templates = self.dao.get_templates();
          let mut result = Vec::new();
          for template in templates {
              for model in &template.models {
                  let id = format!("{}-{}", template.id, model.id);
                  if let Some(instance) = self.dao.get_instance(&id) {
                      result.push(instance);
                  }
              }
          }
          result
      }

      /// 获取当前高亮的实例
      pub fn current_instance(&self) -> Option<&ProviderInstance> {
          let instances = self.get_sorted_instances();
          instances.get(self.list_index).copied()
      }

      /// 获取当前选中的 Provider 模板（用于新建向导）
      pub fn current_provider(&self) -> Option<&ProviderTemplate> {
          let templates = self.dao.get_templates();
          templates.get(self.provider_index).copied()
      }

      /// 获取当前选中的 Model（用于新建向导）
      pub fn current_model(&self) -> Option<&crate::domain::ModelTemplate> {
          let provider = self.current_provider()?;
          provider.models.get(self.model_index)
      }
  }
  ```

- [ ] **Step 2: 在 app.rs 中继续添加事件处理逻辑**

  在 `src/app.rs` 的 `App` impl 块中继续添加：

  ```rust
  use crossterm::event::{KeyCode, KeyEvent};

  impl App {
      /// 处理键盘事件
      pub fn on_key(&mut self, key: KeyEvent) {
          self.error_message = None;
          match &self.state.clone() {
              AppState::List => self.handle_list(key),
              AppState::CreateProvider => self.handle_create_provider(key),
              AppState::CreateModel { .. } => self.handle_create_model(key),
              AppState::CreateApiKey { .. } => self.handle_create_api_key(key),
              AppState::Edit { .. } => self.handle_edit(key),
              AppState::DeleteConfirm { .. } => self.handle_delete_confirm(key),
          }
      }

      fn handle_list(&mut self, key: KeyEvent) {
          let instances = self.get_sorted_instances();
          match key.code {
              KeyCode::Char('q') => self.should_quit = true,
              KeyCode::Char('n') => {
                  self.state = AppState::CreateProvider;
                  self.provider_index = 0;
              }
              KeyCode::Char('e') => {
                  if let Some(instance) = self.current_instance() {
                      self.edit_input = InputState::new(instance.api_key.clone());
                      self.state = AppState::Edit { instance_id: instance.id.clone() };
                  }
              }
              KeyCode::Char('d') => {
                  if let Some(instance) = self.current_instance() {
                      self.state = AppState::DeleteConfirm { instance_id: instance.id.clone() };
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

      fn handle_create_provider(&mut self, key: KeyEvent) {
          let templates = self.dao.get_templates();
          match key.code {
              KeyCode::Esc => self.state = AppState::List,
              KeyCode::Enter => {
                  if let Some(template) = templates.get(self.provider_index) {
                      self.state = AppState::CreateModel {
                          template_id: template.id.clone(),
                      };
                      self.model_index = 0;
                  }
              }
              KeyCode::Up => {
                  if self.provider_index > 0 {
                      self.provider_index -= 1;
                  }
              }
              KeyCode::Down => {
                  if self.provider_index + 1 < templates.len() {
                      self.provider_index += 1;
                  }
              }
              _ => {}
          }
      }

      fn handle_create_model(&mut self, key: KeyEvent) {
          match key.code {
              KeyCode::Esc => self.state = AppState::CreateProvider,
              KeyCode::Enter => {
                  if let Some(model) = self.current_model() {
                      if let Some(template) = self.current_provider() {
                          self.state = AppState::CreateApiKey {
                              template_id: template.id.clone(),
                              model_id: model.id.clone(),
                          };
                          self.api_key_input = InputState::new(String::new());
                      }
                  }
              }
              KeyCode::Up => {
                  if let Some(template) = self.current_provider() {
                      if self.model_index > 0 {
                          self.model_index -= 1;
                      }
                  }
              }
              KeyCode::Down => {
                  if let Some(template) = self.current_provider() {
                      if self.model_index + 1 < template.models.len() {
                          self.model_index += 1;
                      }
                  }
              }
              _ => {}
          }
      }

      fn handle_create_api_key(&mut self, key: KeyEvent) {
          match key.code {
              KeyCode::Esc => {
                  if let AppState::CreateApiKey { template_id, .. } = &self.state {
                      let tid = template_id.clone();
                      self.state = AppState::CreateModel { template_id: tid };
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

      fn submit_create(&mut self) {
          if let AppState::CreateApiKey { template_id, model_id } = self.state.clone() {
              let id = format!("{}-{}", template_id, model_id);
              let instance = ProviderInstance {
                  id,
                  template_id,
                  model_id,
                  api_key: self.api_key_input.value.clone(),
                  created_at: chrono::Utc::now(),
              };
              match self.dao.create_instance(instance) {
                  Ok(()) => {
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

      fn handle_edit(&mut self, key: KeyEvent) {
          match key.code {
              KeyCode::Esc => self.state = AppState::List,
              KeyCode::Enter => {
                  if let AppState::Edit { instance_id } = self.state.clone() {
                      if let Err(e) = self.dao.update_instance(&instance_id, self.edit_input.value.clone()) {
                          self.error_message = Some(e.to_string());
                      }
                      self.state = AppState::List;
                  }
              }
              KeyCode::Backspace => self.edit_input.backspace(),
              KeyCode::Left => self.edit_input.move_left(),
              KeyCode::Right => self.edit_input.move_right(),
              KeyCode::Char(c) => self.edit_input.insert_char(c),
              _ => {}
          }
      }

      fn handle_delete_confirm(&mut self, key: KeyEvent) {
          match key.code {
              KeyCode::Char('y') | KeyCode::Char('Y') => {
                  if let AppState::DeleteConfirm { instance_id } = self.state.clone() {
                      if let Err(e) = self.dao.delete_instance(&instance_id) {
                          self.error_message = Some(e.to_string());
                      } else {
                          let instances = self.get_sorted_instances();
                          if self.list_index >= instances.len() && self.list_index > 0 {
                              self.list_index -= 1;
                          }
                      }
                      self.state = AppState::List;
                  }
              }
              _ => self.state = AppState::List,
          }
      }
  }
  ```

- [ ] **Step 3: 修改 lib.rs 导出 app 模块（注意已有 app/ 目录，这里是指 src/app.rs）**

  等等，`src/app/` 目录已经存在。设计 spec 中的 `app.rs` 建议放在 `src/app.rs`，但已有 `src/app/mod.rs`。为避免冲突，我们把 App 状态机和事件循环放在 `src/app/state.rs`。

  修正计划：创建 `src/app/state.rs` 而不是 `src/app.rs`。

  创建 `src/app/state.rs`，把上面的代码完整写进去。

  然后修改 `src/app/mod.rs`：

  ```rust
  pub mod state;
  pub mod templates;
  ```

  修改 `src/lib.rs` 不变（已经 `pub mod app;`）。

- [ ] **Step 4: 运行 cargo check 确认编译正常**

  运行：`cargo check`
  预期：成功完成

- [ ] **Step 5: Commit**

  ```bash
  git add src/app/state.rs src/app/mod.rs
  git commit -m "feat(app): add App state machine and event handlers"
  ```

---

### Task 5: TDD 实现 InputState

**Files:**
- Create: `tests/input_test.rs`

- [ ] **Step 1: 创建测试**

  ```rust
  use cc_switch_tui::app::state::InputState;

  #[test]
  fn test_input_new() {
      let input = InputState::new("hello".to_string());
      assert_eq!(input.value, "hello");
      assert_eq!(input.cursor, 5);
  }

  #[test]
  fn test_insert_char() {
      let mut input = InputState::new("helo".to_string());
      input.move_left();
      input.insert_char('l');
      assert_eq!(input.value, "hello");
      assert_eq!(input.cursor, 4);
  }

  #[test]
  fn test_backspace() {
      let mut input = InputState::new("hello".to_string());
      input.backspace();
      assert_eq!(input.value, "hell");
      assert_eq!(input.cursor, 4);
  }

  #[test]
  fn test_move_left_right() {
      let mut input = InputState::new("hi".to_string());
      input.move_left();
      assert_eq!(input.cursor, 1);
      input.move_left();
      assert_eq!(input.cursor, 0);
      input.move_right();
      assert_eq!(input.cursor, 1);
  }

  #[test]
  fn test_unicode_insert_and_backspace() {
      let mut input = InputState::new("中".to_string());
      input.insert_char('文');
      assert_eq!(input.value, "中文");
      assert_eq!(input.cursor, 2);
      input.backspace();
      assert_eq!(input.value, "中");
      assert_eq!(input.cursor, 1);
  }
  ```

- [ ] **Step 2: 运行 cargo test --test input_test**

  运行：`cargo test --test input_test`
  预期：5 个测试全部 PASS

- [ ] **Step 3: Commit**

  ```bash
  git add tests/input_test.rs
  git commit -m "test(input): add InputState tests"
  ```

---

### Task 6: 实现 ui/list.rs 主界面渲染

**Files:**
- Create: `src/ui/list.rs`
- Create: `src/ui/mod.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: 创建 ui/mod.rs**

  ```rust
  pub mod list;
  pub mod create;
  pub mod edit;
  pub mod popup;
  ```

  修改 `src/lib.rs` 添加 `pub mod ui;`。

- [ ] **Step 2: 创建 ui/list.rs**

  ```rust
  use crate::app::state::App;
  use ratatui::{
      layout::{Constraint, Direction, Layout, Margin},
      style::{Color, Style},
      text::{Line, Span},
      widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
      Frame,
  };

  /// 渲染主界面：左侧实例列表 + 右侧信息面板 + 底部帮助栏
  pub fn draw_list(frame: &mut Frame, app: &App) {
      let main_layout = Layout::default()
          .direction(Direction::Vertical)
          .constraints([Constraint::Min(0), Constraint::Length(1)])
          .split(frame.area());

      let content_layout = Layout::default()
          .direction(Direction::Horizontal)
          .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
          .split(main_layout[0]);

      draw_instance_list(frame, content_layout[0], app);
      draw_info_panel(frame, content_layout[1], app);
      draw_help_bar(frame, main_layout[1]);

      if let Some(ref msg) = app.error_message {
          draw_error_popup(frame, msg);
      }
  }

  fn draw_instance_list(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
      let templates = app.dao.get_templates();
      let mut items: Vec<ListItem> = Vec::new();
      let mut flat_index = 0;

      for template in templates {
          let group_instances: Vec<_> = template.models.iter()
              .filter_map(|m| {
                  let id = format!("{}-{}", template.id, m.id);
                  app.dao.get_instance(&id)
              })
              .collect();

          if group_instances.is_empty() {
              continue;
          }

          items.push(ListItem::new(Line::from(vec![
              Span::styled(
                  format!("[{}]", template.name),
                  Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD),
              ),
          ])));

          for instance in group_instances {
              let model = template.models.iter()
                  .find(|m| m.id == instance.model_id)
                  .map(|m| m.name.as_str())
                  .unwrap_or("Unknown");

              let is_selected = flat_index == app.list_index;
              let style = if is_selected {
                  Style::default().bg(Color::Blue).fg(Color::White)
              } else {
                  Style::default()
              };

              items.push(ListItem::new(Line::from(vec![
                  Span::raw("  "),
                  Span::raw(model),
              ])).style(style));
              flat_index += 1;
          }
      }

      let list = List::new(items)
          .block(Block::default().title("实例列表").borders(Borders::ALL));
      frame.render_widget(list, area);
  }

  fn draw_info_panel(frame: &mut Frame, area: ratatui::layout::Rect, app: &App) {
      let mut text = vec![];

      if let Some(instance) = app.current_instance() {
          if let Some(template) = app.dao.get_template(&instance.template_id) {
              let model = template.models.iter()
                  .find(|m| m.id == instance.model_id)
                  .map(|m| m.name.as_str())
                  .unwrap_or("Unknown");

              text.push(Line::from(vec![Span::styled(
                  "实例详情",
                  Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD),
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
                  Style::default().fg(Color::Cyan).add_modifier(ratatui::style::Modifier::BOLD),
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
                  text.push(Line::from(format!("{}={}", key, env.get(key).unwrap())));
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

  fn draw_help_bar(frame: &mut Frame, area: ratatui::layout::Rect) {
      let help = "↑↓:移动  n:新建  e:编辑  d:删除  q:退出";
      let paragraph = Paragraph::new(help)
          .style(Style::default().bg(Color::DarkGray).fg(Color::White));
      frame.render_widget(paragraph, area);
  }

  fn draw_error_popup(frame: &mut Frame, msg: &str) {
      let area = frame.area();
      let popup_area = ratatui::layout::Rect {
          x: area.width / 4,
          y: area.height / 2 - 2,
          width: area.width / 2,
          height: 5,
      };
      frame.render_widget(Clear, popup_area);
      let paragraph = Paragraph::new(msg)
          .block(Block::default().title("错误").borders(Borders::ALL))
          .style(Style::default().fg(Color::Red));
      frame.render_widget(paragraph, popup_area);
  }
  ```

- [ ] **Step 3: 运行 cargo check 确认编译正常**

  运行：`cargo check`
  预期：成功完成

- [ ] **Step 4: Commit**

  ```bash
  git add src/ui/ src/lib.rs
  git commit -m "feat(ui): add main list and info panel rendering"
  ```

---

### Task 7: 实现 ui/create.rs 新建向导渲染

**Files:**
- Create: `src/ui/create.rs`

- [ ] **Step 1: 创建 ui/create.rs**

  ```rust
  use crate::app::state::{App, AppState};
  use ratatui::{
      layout::{Alignment, Constraint, Direction, Layout, Rect},
      style::{Color, Style},
      text::{Line, Span},
      widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
      Frame,
  };

  /// 根据当前 AppState 渲染新建向导的对应页面
  pub fn draw_create(frame: &mut Frame, app: &App) {
      match &app.state {
          AppState::CreateProvider => draw_provider_select(frame, app),
          AppState::CreateModel { .. } => draw_model_select(frame, app),
          AppState::CreateApiKey { .. } => draw_api_key_input(frame, app),
          _ => {}
      }
  }

  fn centered_rect(frame: &Frame, width: u16, height: u16) -> Rect {
      let area = frame.area();
      Rect {
          x: area.width.saturating_sub(width) / 2,
          y: area.height.saturating_sub(height) / 2,
          width: width.min(area.width),
          height: height.min(area.height),
      }
  }

  fn draw_provider_select(frame: &mut Frame, app: &App) {
      let area = centered_rect(frame, 40, 12);
      frame.render_widget(Clear, area);

      let templates = app.dao.get_templates();
      let items: Vec<ListItem> = templates.iter().enumerate().map(|(i, t)| {
          let style = if i == app.provider_index {
              Style::default().bg(Color::Blue).fg(Color::White)
          } else {
              Style::default()
          };
          ListItem::new(t.name.clone()).style(style)
      }).collect();

      let list = List::new(items)
          .block(Block::default().title("选择 Provider").borders(Borders::ALL));
      frame.render_widget(list, area);
  }

  fn draw_model_select(frame: &mut Frame, app: &App) {
      let area = centered_rect(frame, 40, 12);
      frame.render_widget(Clear, area);

      if let Some(template) = app.current_provider() {
          let items: Vec<ListItem> = template.models.iter().enumerate().map(|(i, m)| {
              let style = if i == app.model_index {
                  Style::default().bg(Color::Blue).fg(Color::White)
              } else {
                  Style::default()
              };
              ListItem::new(m.name.clone()).style(style)
          }).collect();

          let list = List::new(items)
              .block(Block::default().title(format!("选择 Model - {}", template.name)).borders(Borders::ALL));
          frame.render_widget(list, area);
      }
  }

  fn draw_api_key_input(frame: &mut Frame, app: &App) {
      let area = centered_rect(frame, 50, 7);
      frame.render_widget(Clear, area);

      let text = vec![
          Line::from("请输入 API Key:"),
          Line::from(""),
          Line::from(vec![
              Span::raw("> "),
              Span::raw(app.api_key_input.value.clone()),
              Span::styled("_", Style::default().fg(Color::Yellow)),
          ]),
      ];

      let paragraph = Paragraph::new(text)
          .block(Block::default().title("输入 API Key").borders(Borders::ALL));
      frame.render_widget(paragraph, area);
  }
  ```

- [ ] **Step 2: 运行 cargo check 确认编译正常**

  运行：`cargo check`
  预期：成功完成

- [ ] **Step 3: Commit**

  ```bash
  git add src/ui/create.rs
  git commit -m "feat(ui): add create wizard rendering"
  ```

---

### Task 8: 实现 ui/edit.rs 和 ui/popup.rs

**Files:**
- Create: `src/ui/edit.rs`
- Create: `src/ui/popup.rs`

- [ ] **Step 1: 创建 ui/edit.rs**

  ```rust
  use crate::app::state::App;
  use ratatui::{
      layout::Rect,
      style::{Color, Style},
      text::Line,
      widgets::{Block, Borders, Clear, Paragraph},
      Frame,
  };

  /// 渲染编辑 API Key 的弹窗
  pub fn draw_edit(frame: &mut Frame, app: &App) {
      let area = frame.area();
      let popup_area = Rect {
          x: area.width / 4,
          y: area.height / 2 - 3,
          width: area.width / 2,
          height: 7,
      };
      frame.render_widget(Clear, popup_area);

      let text = vec![
          Line::from("修改 API Key:"),
          Line::from(""),
          Line::from(format!("> {}{}",
              app.edit_input.value,
              "_"
          )),
      ];

      let paragraph = Paragraph::new(text)
          .block(Block::default().title("编辑").borders(Borders::ALL));
      frame.render_widget(paragraph, popup_area);
  }
  ```

- [ ] **Step 2: 创建 ui/popup.rs**

  ```rust
  use crate::app::state::App;
  use ratatui::{
      layout::Rect,
      style::{Color, Style},
      text::Line,
      widgets::{Block, Borders, Clear, Paragraph},
      Frame,
  };

  /// 渲染删除确认对话框
  pub fn draw_delete_confirm(frame: &mut Frame, app: &App) {
      let area = frame.area();
      let popup_area = Rect {
          x: area.width / 4,
          y: area.height / 2 - 3,
          width: area.width / 2,
          height: 7,
      };
      frame.render_widget(Clear, popup_area);

      let instance_id = match &app.state {
          crate::app::state::AppState::DeleteConfirm { instance_id } => instance_id.clone(),
          _ => return,
      };

      let text = vec![
          Line::from(format!("确定删除 {} 的实例吗？", instance_id)),
          Line::from(""),
          Line::from("[Y] 确认    [N] 取消"),
      ];

      let paragraph = Paragraph::new(text)
          .block(Block::default().title("确认删除").borders(Borders::ALL))
          .style(Style::default().fg(Color::Red));
      frame.render_widget(paragraph, popup_area);
  }
  ```

- [ ] **Step 3: 运行 cargo check 确认编译正常**

  运行：`cargo check`
  预期：成功完成

- [ ] **Step 4: Commit**

  ```bash
  git add src/ui/edit.rs src/ui/popup.rs
  git commit -m "feat(ui): add edit and delete confirm popups"
  ```

---

### Task 9: 实现 ui/mod.rs 渲染分发和主循环

**Files:**
- Modify: `src/ui/mod.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: 修改 ui/mod.rs 添加渲染分发函数**

  ```rust
  pub mod create;
  pub mod edit;
  pub mod list;
  pub mod popup;

  use crate::app::state::{App, AppState};
  use ratatui::Frame;

  /// 根据 App 状态分发渲染逻辑
  pub fn draw(frame: &mut Frame, app: &App) {
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

- [ ] **Step 2: 重写 main.rs 启动 TUI 主循环**

  ```rust
  use cc_switch_tui::app::state::App;
  use cc_switch_tui::event::{parse_event, AppEvent};
  use cc_switch_tui::ui;
  use crossterm::{
      event::{self, DisableMouseCapture, EnableMouseCapture, Event as CEvent},
      terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
  };
  use ratatui::backend::CrosstermBackend;
  use ratatui::Terminal;
  use std::io;
  use std::time::Duration;

  fn main() -> io::Result<()> {
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

      Ok(())
  }

  fn run_app<B: ratatui::backend::Backend>(
      terminal: &mut Terminal<B>,
      app: &mut App,
  ) -> io::Result<()> {
      let tick_rate = Duration::from_millis(100);

      loop {
          terminal.draw(|f| ui::draw(f, app))?;

          if event::poll(tick_rate)? {
              if let CEvent::Key(key) = event::read()? {
                  if let Some(app_event) = parse_event(CEvent::Key(key)) {
                      if let AppEvent::Key(key_event) = app_event {
                          app.on_key(key_event);
                      }
                  }
              }
          }

          if app.should_quit {
              return Ok(());
          }
      }
  }
  ```

  等等，上面的 `parse_event` 传入的是 `CEvent::Key(key)` 但 `parse_event` 接收的是 `Event`，需要修正。实际上可以直接简化 main.rs：

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

  fn main() -> io::Result<()> {
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

      Ok(())
  }

  fn run_app<B: ratatui::backend::Backend>(
      terminal: &mut Terminal<B>,
      app: &mut App,
  ) -> io::Result<()> {
      let tick_rate = Duration::from_millis(100);

      loop {
          terminal.draw(|f| ui::draw(f, app))?;

          if event::poll(tick_rate)? {
              if let Event::Key(key) = event::read()? {
                  if key.kind == event::KeyEventKind::Press {
                      app.on_key(key);
                  }
              }
          }

          if app.should_quit {
              return Ok(());
          }
      }
  }
  ```

- [ ] **Step 3: 运行 cargo build 确认编译通过**

  运行：`cargo build`
  预期：成功完成

- [ ] **Step 4: Commit**

  ```bash
  git add src/ui/mod.rs src/main.rs
  git commit -m "feat(tui): wire up main loop and rendering dispatcher"
  ```

---

### Task 10: 运行完整测试并手动验证 TUI

**Files:**
- 无新增文件

- [ ] **Step 1: 运行 cargo test 确认全部测试通过**

  运行：`cargo test`
  预期：11 个测试全部 PASS（dao_test 9 个 + input_test 5 个 + template_test 1 个）

- [ ] **Step 2: 运行 cargo run 确认 TUI 能正常启动**

  运行：`cargo run`
  预期：进入 TUI 界面，左侧显示空列表，右侧显示"暂无实例，按 n 新建"，底部有按键提示
  按 `q` 正常退出

- [ ] **Step 3: 手动测试基本流程**

  运行：`cargo run`
  按 `n` -> 选择 minimax -> Enter -> 选择模型 -> Enter -> 输入 api-key -> Enter
  预期：返回主界面，左侧出现新实例，右侧显示详情

  选中实例 -> 按 `e` -> 修改 api-key -> Enter
  预期：返回主界面，右侧 api-key 已更新

  选中实例 -> 按 `d` -> 按 `Y`
  预期：实例被删除，列表为空

  按 `q` 退出

- [ ] **Step 4: Commit**

  ```bash
  git commit --allow-empty -m "test: manual TUI smoke test passed"
  ```

---

## Self-Review

**1. Spec coverage:**
- ✅ 主界面布局：左侧列表 + 右侧信息面板 + 底部帮助栏
- ✅ 实例按 provider 分组展示
- ✅ 新建向导：Provider 选择 -> Model 选择 -> API Key 输入
- ✅ 编辑页面：修改 API Key
- ✅ 删除确认对话框：Y/N
- ✅ 按键操作：↑↓ n e d q Esc
- ✅ 无"当前激活"状态
- ✅ 错误提示

**2. Placeholder scan:**
- 无 TBD、TODO 等占位符（除了已有 shell.rs 中的 apply_config，不参与本次实现）

**3. Type一致性：**
- `AppState` 变体与所有使用处一致
- `InputState` 方法在所有调用处一致
- `Dao::update_instance` 签名在 trait 和 impl 中一致

---

## 执行方式

Plan complete and saved to `docs/superpowers/plans/2026-04-14-cc-switch-tui-tui.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints for review

Which approach?
