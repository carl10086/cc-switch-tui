# OpenCode Alias Support 设计文档

## 1. Objective

为 cc-switch-tui 增加 OpenCode 并行支持：用户在 TUI 中配置的每个 provider instance 同时生成 Claude Code alias（`cl-*`）和 OpenCode alias（`oc-*`），通过 `OPENCODE_CONFIG` 环境变量指向独立配置文件实现切换。

核心目标：
- 配置一次，双端生效（claude + opencode）
- TUI 中可选择 OpenCode model
- 后台自动拉取并缓存模型列表
- 不影响现有 claude alias 生成逻辑

## 2. Commands

无需新增 CLI 命令。TUI 内部操作：
- 创建 instance → 新增"选择 OpenCode Model"步骤
- 编辑 instance → 右侧面板新增 OpenCode Model 下拉框 + 刷新按钮
- 生成 aliases → 同时生成 `cl-*` 和 `oc-*`

## 3. Project Structure

### 3.1 文件变动

```
src/
  domain/
    template.rs          # ProviderTemplate/ModelTemplate 新增 opencode 字段
    instance.rs          # ProviderInstance 新增 opencode_model_id
  dao/
    sqlite_impl.rs       # 新增 opencode_models 表操作
    mod.rs               # DAO trait 新增模型缓存方法
  app/
    templates.rs         # minimax/kimi 模板配置 opencode 映射
    state.rs             # 新增后台模型刷新逻辑
  ui/
    create.rs            # 创建流程新增 model 选择步骤
    edit.rs              # 编辑面板新增 OpenCode Model 下拉框
  shell.rs              # 生成逻辑扩展为同时输出 cl 和 oc alias
  opencode_config.rs    # 【新增】生成 opencode.json 配置文件
```

### 3.2 数据模型

```rust
// domain/template.rs
pub struct ProviderTemplate {
    pub id: String,
    pub name: String,
    pub default_env: HashMap<String, String>,
    pub models: Vec<ModelTemplate>,
    // 新增：opencode provider 级映射
    pub opencode_provider_id: String,
    pub opencode_npm: String,
    pub opencode_base_url: String,
    pub opencode_env_var: String,
}

pub struct ModelTemplate {
    pub id: String,
    pub name: String,
    pub env_overrides: HashMap<String, String>,
    // 新增：opencode model ID 映射
    pub opencode_model_id: String,
}

// domain/instance.rs
pub struct ProviderInstance {
    pub id: String,
    pub template_id: String,
    pub model_id: String,
    pub api_key: String,
    pub created_at: DateTime<Utc>,
    pub alias: String,
    // 新增
    pub opencode_model_id: String,
}
```

### 3.3 数据库 Schema

```sql
-- 新增表：缓存 opencode 官方模型列表
CREATE TABLE opencode_models (
    provider_id TEXT NOT NULL,
    model_id TEXT NOT NULL,
    model_name TEXT NOT NULL,
    context_limit INTEGER,
    output_limit INTEGER,
    updated_at TEXT NOT NULL,
    PRIMARY KEY (provider_id, model_id)
);

-- instances 表扩展
ALTER TABLE instances ADD COLUMN opencode_model_id TEXT NOT NULL DEFAULT '';
```

## 4. Code Style

- 遵循现有 Rust 代码风格，不引入新抽象
- TUI 中使用 ratatui 的现有组件模式（List/Popup）
- 错误处理使用 `thiserror`，与现有代码一致
- 字符串常量优先使用 `&'static str`

## 5. Testing Strategy

### 5.1 单元测试
- `opencode_config.rs`: 验证 JSON 输出格式符合 opencode schema
- `shell.rs`: 验证 `oc-*` alias 包含正确的 `OPENCODE_CONFIG` 和 env var

### 5.2 集成测试
- 启动 TUI 后验证 `opencode_models` 表已写入数据
- 创建 instance 后验证 `~/.cc-switch-tui/opencode/*.json` 文件生成正确
- source aliases.zsh 后验证 `oc-*` 函数可正常调用

## 6. Boundaries

### Always
- 所有现有 provider 模板必须同时配置 opencode 映射
- `cl-*` alias 生成逻辑 100% 保持原样
- opencode 配置文件独立存放于 `~/.cc-switch-tui/opencode/`

### Ask First
- 新增第三方 provider 模板时是否需要同时支持 opencode
- 模型列表拉取失败后是否阻断 TUI 启动

### Never
- 不修改 claude alias 的格式或环境变量
- 不在 opencode.json 中硬编码 API key（使用 `{env:VAR}`）
- 不引入除 `OPENCODE_CONFIG` 以外的 opencode 配置方式
