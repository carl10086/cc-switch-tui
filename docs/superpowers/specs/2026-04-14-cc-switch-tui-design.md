# cc-switch-tui 设计文档

## 1. 项目目标

构建一个 Rust TUI 工具，用于在终端中管理并切换不同的 Claude Code Anthropic API 接入点（provider）。用户通过 TUI 选择 provider 和 model，生成对应的环境变量配置，并通过 `source` 模式应用到当前 shell。

## 2. 核心原则

- **不修改 Claude Code 的 settings.json**，仅通过环境变量影响当前 shell 会话
- **内置模板驱动**：provider 和 model 均为下拉框选择，不允许用户手动输入
- **实例 name 不允许自定义**：由 `template_id-model_id` 组合生成
- **轻量、可扩展**：存储层抽象为 Dao trait，当前用内存实现（MemoryDaoImpl），后续可扩展为文件/数据库存储

## 3. 模块架构

```
src/
├── main.rs              # 入口：初始化 Dao、启动 TUI
├── domain/
│   ├── mod.rs
│   ├── template.rs      # ProviderTemplate, ModelTemplate（内置）
│   ├── instance.rs      # ProviderInstance（用户创建的实例）
│   └── error.rs         # AppError 错误类型
├── dao/
│   ├── mod.rs           # Dao trait
│   └── memory_impl.rs   # MemoryDaoImpl
├── app/
│   ├── mod.rs           # App 状态和业务逻辑编排
│   └── templates.rs     # 内置模板注册表
├── tui/
│   ├── mod.rs           # TUI 入口
│   └── ...              # TUI 页面和组件（本次设计不做详细展开）
└── shell.rs             # apply_config() 空实现
```

## 4. 数据模型

### 4.1 ProviderTemplate

```rust
/// Provider 模板，内置在程序中，包含默认环境变量和可选模型列表
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
```

### 4.2 ModelTemplate

```rust
/// 模型模板，定义特定模型对默认环境变量的覆盖项
pub struct ModelTemplate {
    /// 模型唯一标识，如 "MiniMax-M2.7-highspeed"
    pub id: String,
    /// 模型显示名称
    pub name: String,
    /// 需要覆盖或追加的环境变量键值对
    pub env_overrides: HashMap<String, String>,
}
```

### 4.3 ProviderInstance

```rust
/// 用户创建的 Provider 实例，对应一个具体的模板和模型配置
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

## 5. Dao 接口

```rust
pub trait Dao {
    // 模板（只读，内置）
    fn get_templates(&self) -> Vec<&ProviderTemplate>;
    fn get_template(&self, id: &str) -> Option<&ProviderTemplate>;

    // 实例（CRUD + 切换）
    fn list_instances(&self) -> Vec<&ProviderInstance>;
    fn get_instance(&self, id: &str) -> Option<&ProviderInstance>;
    fn create_instance(&mut self, instance: ProviderInstance) -> Result<(), AppError>;
    fn delete_instance(&mut self, id: &str) -> Result<(), AppError>;

    // 当前选中的实例
    fn get_current_instance(&self) -> Option<&ProviderInstance>;
    fn set_current_instance(&mut self, id: &str) -> Result<(), AppError>;
}
```

## 6. 错误处理

使用 `thiserror` 定义统一的 `AppError`：

```rust
pub enum AppError {
    #[error("Instance already exists: {0}")]
    InstanceAlreadyExists(String),

    #[error("Instance not found: {0}")]
    InstanceNotFound(String),

    #[error("Template not found: {0}")]
    TemplateNotFound(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),
}
```

## 7. apply_config 占位

切换实例时调用 `apply_config()`，当前为空实现，后续填充生成 `~/.cc-switch-tui/env.sh` 的逻辑：

```rust
pub fn apply_config(instance: &ProviderInstance, template: &ProviderTemplate) {
    // TODO: 生成 source 脚本或设置环境变量
    let _ = (instance, template);
}
```

## 8. 内置模板

当前仅内置 **minimax** provider：

```json
{
  "id": "minimax",
  "name": "MiniMax",
  "default_env": {
    "ANTHROPIC_BASE_URL": "https://api.minimaxi.com/anthropic",
    "ANTHROPIC_DEFAULT_HAIKU_MODEL": "MiniMax-M2.7-highspeed",
    "ANTHROPIC_DEFAULT_OPUS_MODEL": "MiniMax-M2.7-highspeed",
    "ANTHROPIC_DEFAULT_SONNET_MODEL": "MiniMax-M2.7-highspeed",
    "API_TIMEOUT_MS": "3000000",
    "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1"
  },
  "models": [
    {
      "id": "MiniMax-M2.7-highspeed",
      "name": "MiniMax M2.7 Highspeed",
      "env_overrides": {
        "ANTHROPIC_MODEL": "MiniMax-M2.7-highspeed"
      }
    }
  ]
}
```

## 9. 依赖项（Cargo.toml）

```toml
[dependencies]
chrono = { version = "0.4", features = ["serde"] }
thiserror = "1"
```

后续 TUI 实现时再加入 `ratatui`、`crossterm` 等依赖。

## 10. 后续工作

1. 实现 domain 层数据结构和错误类型
2. 实现 Dao trait 和 MemoryDaoImpl
3. 注册内置 minimax 模板
4. 搭建 TUI 骨架（ratatui + crossterm）
5. 填充 `apply_config()` 的 source 脚本生成逻辑
