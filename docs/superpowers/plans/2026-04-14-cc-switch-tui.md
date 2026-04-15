# cc-switch-tui 实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 实现 cc-switch-tui 的核心数据层：domain 模型、Dao 接口、MemoryDaoImpl、内置 minimax 模板，以及 apply_config 空函数。

**Architecture:** 采用经典分层架构，domain 层定义纯数据结构和错误类型，dao 层提供存储抽象和内存实现，app 层管理内置模板注册表。后续 TUI 层将依赖这些基础设施。

**Tech Stack:** Rust 2024 edition, chrono, thiserror

> **注意：** 请严格遵守 `.claude/rules/code.md` 中的代码规范：最小化代码、不引入未请求的功能、只修改必要文件、所有结构体必须带中文注释。

---

## 文件结构

```
src/
├── main.rs              # 入口
├── domain/
│   ├── mod.rs           # domain 模块导出
│   ├── template.rs      # ProviderTemplate, ModelTemplate
│   ├── instance.rs      # ProviderInstance
│   └── error.rs         # AppError
├── dao/
│   ├── mod.rs           # Dao trait
│   └── memory_impl.rs   # MemoryDaoImpl
├── app/
│   ├── mod.rs           # app 模块导出
│   └── templates.rs     # 内置模板注册表
└── shell.rs             # apply_config() 空实现
```

---

### Task 1: 配置 Cargo.toml 依赖

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: 添加 chrono 和 thiserror 依赖**

  修改 `Cargo.toml`：

  ```toml
  [dependencies]
  chrono = { version = "0.4", features = ["serde"] }
  thiserror = "1"
  ```

- [ ] **Step 2: 运行 cargo check 确认依赖下载和编译正常**

  运行：`cargo check`
  预期：成功完成，无错误

- [ ] **Step 3: Commit**

  ```bash
  git add Cargo.toml
  git commit -m "deps: add chrono and thiserror"
  ```

---

### Task 2: 实现 domain/error.rs 错误类型

**Files:**
- Create: `src/domain/error.rs`
- Create: `src/domain/mod.rs`

- [ ] **Step 1: 创建 domain/error.rs**

  ```rust
  use thiserror::Error;

  /// 应用统一的错误类型
  #[derive(Error, Debug, PartialEq)]
  pub enum AppError {
      #[error("实例已存在: {0}")]
      InstanceAlreadyExists(String),

      #[error("实例不存在: {0}")]
      InstanceNotFound(String),

      #[error("模板不存在: {0}")]
      TemplateNotFound(String),

      #[error("模型不存在: {0}")]
      ModelNotFound(String),
  }
  ```

- [ ] **Step 2: 创建 domain/mod.rs 并导出 error**

  ```rust
  pub mod error;
  pub use error::AppError;
  ```

- [ ] **Step 3: 运行 cargo check 确认编译正常**

  运行：`cargo check`
  预期：成功完成

- [ ] **Step 4: Commit**

  ```bash
  git add src/domain/
  git commit -m "feat(domain): add AppError with thiserror"
  ```

---

### Task 3: 实现 domain/template.rs 和 domain/instance.rs

**Files:**
- Create: `src/domain/template.rs`
- Create: `src/domain/instance.rs`
- Modify: `src/domain/mod.rs`

- [ ] **Step 1: 创建 domain/template.rs**

  ```rust
  use std::collections::HashMap;

  /// Provider 模板，内置在程序中，包含默认环境变量和可选模型列表
  #[derive(Debug, Clone, PartialEq)]
  pub struct ProviderTemplate {
      /// 模板唯一标识，如 "minimax"
      pub id: String,
      /// 模板显示名称，如 "MiniMax"
      pub name: String,
      /// 默认环境变量键值对
      pub default_env: HashMap<String, String>,
      /// 该 Provider 下支持的模型列表
      pub models: Vec<ModelTemplate>,
  }

  /// 模型模板，定义特定模型对默认环境变量的覆盖项
  #[derive(Debug, Clone, PartialEq)]
  pub struct ModelTemplate {
      /// 模型唯一标识，如 "MiniMax-M2.7-highspeed"
      pub id: String,
      /// 模型显示名称
      pub name: String,
      /// 需要覆盖或追加的环境变量键值对
      pub env_overrides: HashMap<String, String>,
  }
  ```

- [ ] **Step 2: 创建 domain/instance.rs**

  ```rust
  use chrono::{DateTime, Utc};

  /// 用户创建的 Provider 实例，对应一个具体的模板和模型配置
  #[derive(Debug, Clone, PartialEq)]
  pub struct ProviderInstance {
      /// 实例唯一标识，格式为 "template_id-model_id"
      pub id: String,
      /// 关联的 Provider 模板 ID
      pub template_id: String,
      /// 关联的 Model 模板 ID
      pub model_id: String,
      /// 用户输入的 API Key
      pub api_key: String,
      /// 实例创建时间
      pub created_at: DateTime<Utc>,
  }
  ```

- [ ] **Step 3: 修改 domain/mod.rs 导出 template 和 instance**

  ```rust
  pub mod error;
  pub mod instance;
  pub mod template;

  pub use error::AppError;
  pub use instance::ProviderInstance;
  pub use template::{ModelTemplate, ProviderTemplate};
  ```

- [ ] **Step 4: 运行 cargo check 确认编译正常**

  运行：`cargo check`
  预期：成功完成

- [ ] **Step 5: Commit**

  ```bash
  git add src/domain/
  git commit -m "feat(domain): add ProviderTemplate, ModelTemplate and ProviderInstance"
  ```

---

### Task 4: 实现 Dao trait

**Files:**
- Create: `src/dao/mod.rs`

- [ ] **Step 1: 创建 dao/mod.rs 定义 Dao trait**

  ```rust
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

      /// 获取当前选中的实例
      fn get_current_instance(&self) -> Option<&ProviderInstance>;

      /// 设置当前选中的实例
      fn set_current_instance(&mut self, id: &str) -> Result<(), AppError>;
  }
  ```

- [ ] **Step 2: 运行 cargo check 确认编译正常**

  运行：`cargo check`
  预期：成功完成

- [ ] **Step 3: Commit**

  ```bash
  git add src/dao/mod.rs
  git commit -m "feat(dao): define Dao trait"
  ```

---

### Task 5: TDD 实现 MemoryDaoImpl

**Files:**
- Create: `src/dao/memory_impl.rs`
- Modify: `src/dao/mod.rs`

- [ ] **Step 1: 创建测试文件 tests/dao_test.rs（先写失败测试）**

  ```rust
  use cc_switch_tui::dao::memory_impl::MemoryDaoImpl;
  use cc_switch_tui::dao::Dao;
  use cc_switch_tui::domain::{AppError, ProviderInstance, ProviderTemplate, ModelTemplate};
  use chrono::Utc;
  use std::collections::HashMap;

  fn create_test_template() -> ProviderTemplate {
      ProviderTemplate {
          id: "minimax".to_string(),
          name: "MiniMax".to_string(),
          default_env: HashMap::new(),
          models: vec![ModelTemplate {
              id: "m1".to_string(),
              name: "Model 1".to_string(),
              env_overrides: HashMap::new(),
          }],
      }
  }

  fn create_test_instance() -> ProviderInstance {
      ProviderInstance {
          id: "minimax-m1".to_string(),
          template_id: "minimax".to_string(),
          model_id: "m1".to_string(),
          api_key: "test-key".to_string(),
          created_at: Utc::now(),
      }
  }

  #[test]
  fn test_create_and_get_instance() {
      let template = create_test_template();
      let mut dao = MemoryDaoImpl::new(vec![template]);
      let instance = create_test_instance();

      dao.create_instance(instance.clone()).unwrap();
      let retrieved = dao.get_instance("minimax-m1").unwrap();
      assert_eq!(retrieved.id, "minimax-m1");
      assert_eq!(retrieved.api_key, "test-key");
  }

  #[test]
  fn test_create_duplicate_instance_fails() {
      let template = create_test_template();
      let mut dao = MemoryDaoImpl::new(vec![template]);
      let instance = create_test_instance();

      dao.create_instance(instance.clone()).unwrap();
      let result = dao.create_instance(instance);
      assert_eq!(result, Err(AppError::InstanceAlreadyExists("minimax-m1".to_string())));
  }

  #[test]
  fn test_delete_instance() {
      let template = create_test_template();
      let mut dao = MemoryDaoImpl::new(vec![template]);
      let instance = create_test_instance();

      dao.create_instance(instance).unwrap();
      dao.delete_instance("minimax-m1").unwrap();
      assert!(dao.get_instance("minimax-m1").is_none());
  }

  #[test]
  fn test_delete_nonexistent_instance_fails() {
      let template = create_test_template();
      let mut dao = MemoryDaoImpl::new(vec![template]);

      let result = dao.delete_instance("not-exist");
      assert_eq!(result, Err(AppError::InstanceNotFound("not-exist".to_string())));
  }

  #[test]
  fn test_set_and_get_current_instance() {
      let template = create_test_template();
      let mut dao = MemoryDaoImpl::new(vec![template]);
      let instance = create_test_instance();

      dao.create_instance(instance).unwrap();
      dao.set_current_instance("minimax-m1").unwrap();
      let current = dao.get_current_instance().unwrap();
      assert_eq!(current.id, "minimax-m1");
  }

  #[test]
  fn test_set_current_nonexistent_instance_fails() {
      let template = create_test_template();
      let mut dao = MemoryDaoImpl::new(vec![template]);

      let result = dao.set_current_instance("not-exist");
      assert_eq!(result, Err(AppError::InstanceNotFound("not-exist".to_string())));
  }

  #[test]
  fn test_get_template() {
      let template = create_test_template();
      let dao = MemoryDaoImpl::new(vec![template]);

      let t = dao.get_template("minimax").unwrap();
      assert_eq!(t.id, "minimax");
      assert!(dao.get_template("not-exist").is_none());
  }
  ```

- [ ] **Step 2: 运行 cargo test 确认测试失败**

  运行：`cargo test --test dao_test`
  预期：大量编译错误，因为 `MemoryDaoImpl` 和 `memory_impl` 模块还不存在

- [ ] **Step 3: 创建 dao/memory_impl.rs 实现 MemoryDaoImpl**

  ```rust
  use crate::dao::Dao;
  use crate::domain::{AppError, ProviderInstance, ProviderTemplate};
  use std::collections::HashMap;

  /// 基于内存的 Dao 实现，数据仅在进程生命周期内有效
  pub struct MemoryDaoImpl {
      /// 内置 Provider 模板列表
      templates: Vec<ProviderTemplate>,
      /// 用户创建的实例映射，key 为 instance.id
      instances: HashMap<String, ProviderInstance>,
      /// 当前选中的实例 ID
      current_instance_id: Option<String>,
  }

  impl MemoryDaoImpl {
      /// 使用给定的模板列表创建新的 MemoryDaoImpl
      pub fn new(templates: Vec<ProviderTemplate>) -> Self {
          Self {
              templates,
              instances: HashMap::new(),
              current_instance_id: None,
          }
      }
  }

  impl Dao for MemoryDaoImpl {
      fn get_templates(&self) -> Vec<&ProviderTemplate> {
          self.templates.iter().collect()
      }

      fn get_template(&self, id: &str) -> Option<&ProviderTemplate> {
          self.templates.iter().find(|t| t.id == id)
      }

      fn list_instances(&self) -> Vec<&ProviderInstance> {
          self.instances.values().collect()
      }

      fn get_instance(&self, id: &str) -> Option<&ProviderInstance> {
          self.instances.get(id)
      }

      fn create_instance(&mut self, instance: ProviderInstance) -> Result<(), AppError> {
          if self.instances.contains_key(&instance.id) {
              return Err(AppError::InstanceAlreadyExists(instance.id.clone()));
          }
          self.instances.insert(instance.id.clone(), instance);
          Ok(())
      }

      fn delete_instance(&mut self, id: &str) -> Result<(), AppError> {
          if self.instances.remove(id).is_none() {
              return Err(AppError::InstanceNotFound(id.to_string()));
          }
          if self.current_instance_id.as_deref() == Some(id) {
              self.current_instance_id = None;
          }
          Ok(())
      }

      fn get_current_instance(&self) -> Option<&ProviderInstance> {
          self.current_instance_id
              .as_ref()
              .and_then(|id| self.instances.get(id))
      }

      fn set_current_instance(&mut self, id: &str) -> Result<(), AppError> {
          if !self.instances.contains_key(id) {
              return Err(AppError::InstanceNotFound(id.to_string()));
          }
          self.current_instance_id = Some(id.to_string());
          Ok(())
      }
  }
  ```

- [ ] **Step 4: 修改 dao/mod.rs 导出 memory_impl**

  ```rust
  pub mod memory_impl;
  pub mod error;
  pub mod instance;
  pub mod template;

  pub use error::AppError;
  pub use instance::ProviderInstance;
  pub use template::{ModelTemplate, ProviderTemplate};

  pub use crate::dao::Dao;
  ```

  修改为：

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

      /// 获取当前选中的实例
      fn get_current_instance(&self) -> Option<&ProviderInstance>;

      /// 设置当前选中的实例
      fn set_current_instance(&mut self, id: &str) -> Result<(), AppError>;
  }
  ```

  然后修改 `src/lib.rs`（如果不存在则创建），导出 dao 和 domain：

  ```rust
  pub mod dao;
  pub mod domain;
  ```

- [ ] **Step 5: 运行 cargo test 确认测试全部通过**

  运行：`cargo test --test dao_test`
  预期：7 个测试全部 PASS

- [ ] **Step 6: Commit**

  ```bash
  git add src/dao/ src/domain/ src/lib.rs tests/
  git commit -m "feat(dao): implement MemoryDaoImpl with TDD"
  ```

---

### Task 6: 实现内置 minimax 模板注册

**Files:**
- Create: `src/app/mod.rs`
- Create: `src/app/templates.rs`

- [ ] **Step 1: 创建 app/templates.rs**

  ```rust
  use crate::domain::{ModelTemplate, ProviderTemplate};
  use std::collections::HashMap;

  /// 注册并返回所有内置的 Provider 模板
  pub fn register_templates() -> Vec<ProviderTemplate> {
      vec![minimax_template()]
  }

  /// 构建 minimax Provider 模板
  fn minimax_template() -> ProviderTemplate {
      let mut default_env = HashMap::new();
      default_env.insert(
          "ANTHROPIC_BASE_URL".to_string(),
          "https://api.minimaxi.com/anthropic".to_string(),
      );
      default_env.insert(
          "ANTHROPIC_DEFAULT_HAIKU_MODEL".to_string(),
          "MiniMax-M2.7-highspeed".to_string(),
      );
      default_env.insert(
          "ANTHROPIC_DEFAULT_OPUS_MODEL".to_string(),
          "MiniMax-M2.7-highspeed".to_string(),
      );
      default_env.insert(
          "ANTHROPIC_DEFAULT_SONNET_MODEL".to_string(),
          "MiniMax-M2.7-highspeed".to_string(),
      );
      default_env.insert("API_TIMEOUT_MS".to_string(), "3000000".to_string());
      default_env.insert(
          "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC".to_string(),
          "1".to_string(),
      );

      let mut env_overrides = HashMap::new();
      env_overrides.insert(
          "ANTHROPIC_MODEL".to_string(),
          "MiniMax-M2.7-highspeed".to_string(),
      );

      ProviderTemplate {
          id: "minimax".to_string(),
          name: "MiniMax".to_string(),
          default_env,
          models: vec![ModelTemplate {
              id: "MiniMax-M2.7-highspeed".to_string(),
              name: "MiniMax M2.7 Highspeed".to_string(),
              env_overrides,
          }],
      }
  }
  ```

- [ ] **Step 2: 创建 app/mod.rs**

  ```rust
  pub mod templates;
  ```

- [ ] **Step 3: 修改 src/lib.rs 导出 app**

  ```rust
  pub mod app;
  pub mod dao;
  pub mod domain;
  ```

- [ ] **Step 4: 编写集成测试验证 minimax 模板**

  创建 `tests/template_test.rs`：

  ```rust
  use cc_switch_tui::app::templates::register_templates;

  #[test]
  fn test_minimax_template_registered() {
      let templates = register_templates();
      assert_eq!(templates.len(), 1);

      let minimax = templates.iter().find(|t| t.id == "minimax").unwrap();
      assert_eq!(minimax.name, "MiniMax");
      assert_eq!(
          minimax.default_env.get("ANTHROPIC_BASE_URL").unwrap(),
          "https://api.minimaxi.com/anthropic"
      );
      assert_eq!(minimax.models.len(), 1);

      let model = &minimax.models[0];
      assert_eq!(model.id, "MiniMax-M2.7-highspeed");
      assert_eq!(
          model.env_overrides.get("ANTHROPIC_MODEL").unwrap(),
          "MiniMax-M2.7-highspeed"
      );
  }
  ```

- [ ] **Step 5: 运行 cargo test 确认通过**

  运行：`cargo test --test template_test`
  预期：1 个测试 PASS

- [ ] **Step 6: Commit**

  ```bash
  git add src/app/ src/lib.rs tests/template_test.rs
  git commit -m "feat(app): add built-in minimax template registry"
  ```

---

### Task 7: 实现 apply_config() 空函数

**Files:**
- Create: `src/shell.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: 创建 shell.rs**

  ```rust
  use crate::domain::{ProviderInstance, ProviderTemplate};

  /// 将选中的实例配置应用到当前环境
  /// TODO: 当前为空实现，后续将生成 source 脚本或设置环境变量
  pub fn apply_config(instance: &ProviderInstance, template: &ProviderTemplate) {
      let _ = instance;
      let _ = template;
      // TODO: 生成 ~/.cc-switch-tui/env.sh
  }
  ```

- [ ] **Step 2: 修改 src/lib.rs 导出 shell**

  ```rust
  pub mod app;
  pub mod dao;
  pub mod domain;
  pub mod shell;
  ```

- [ ] **Step 3: 运行 cargo check 确认编译正常**

  运行：`cargo check`
  预期：成功完成

- [ ] **Step 4: Commit**

  ```bash
  git add src/shell.rs src/lib.rs
  git commit -m "feat(shell): add apply_config placeholder"
  ```

---

### Task 8: 重写 main.rs 作为应用入口

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: 重写 main.rs**

  ```rust
  use cc_switch_tui::app::templates::register_templates;
  use cc_switch_tui::dao::memory_impl::MemoryDaoImpl;

  fn main() {
      let templates = register_templates();
      let _dao = MemoryDaoImpl::new(templates);

      // TODO: 启动 TUI
      println!("cc-switch-tui initialized");
  }
  ```

- [ ] **Step 2: 运行 cargo build 确认项目完整编译**

  运行：`cargo build`
  预期：成功完成，生成可执行文件

- [ ] **Step 3: 运行可执行文件确认输出**

  运行：`cargo run`
  预期：输出 `cc-switch-tui initialized`

- [ ] **Step 4: Commit**

  ```bash
  git add src/main.rs
  git commit -m "feat(main): wire up MemoryDaoImpl and templates"
  ```

---

## Self-Review

**1. Spec coverage:**
- ✅ domain 层数据结构（ProviderTemplate, ModelTemplate, ProviderInstance）
- ✅ AppError 错误类型
- ✅ Dao trait + MemoryDaoImpl
- ✅ 内置 minimax 模板
- ✅ apply_config 空函数
- ✅ 所有结构体带中文注释
- ✅ main.rs 入口

**2. Placeholder scan:**
- 无 "TBD"、"TODO" 等未定义占位符（apply_config 中的 TODO 是设计文档明确要求的）

**3. Type consistency:**
- `ProviderInstance` 字段、`Dao` 方法签名、`MemoryDaoImpl` 实现一致
- `AppError` 变体名称在测试和实现中完全匹配

---

## 执行方式

Plan complete and saved to `docs/superpowers/plans/2026-04-14-cc-switch-tui.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints for review

Which approach?
