# cc-switch-tui Kimi Provider 设计文档

## 1. 目标

为 cc-switch-tui 添加 Kimi provider 支持，用户可在 TUI 中创建 Kimi 实例并生成对应的 shell 别名。

## 2. 现状

现有 `src/app/templates.rs` 只注册了 MiniMax 一个 provider，通过 `register_templates()` 返回。所有 provider 模板硬编码在程序中。

## 3. 设计

### 3.1 新增 Kimi Provider 模板

在 `templates.rs` 中新增 `kimi_template()` 函数：

```rust
fn kimi_template() -> ProviderTemplate {
    let mut default_env = HashMap::new();
    default_env.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        "https://api.kimi.com/coding/".to_string(),
    );

    ProviderTemplate {
        id: "kimi".to_string(),
        name: "Kimi".to_string(),
        default_env,
        models: vec![ModelTemplate {
            id: "kimi-for-coding".to_string(),
            name: "Kimi for Coding".to_string(),
            env_overrides: HashMap::new(),
        }],
    }
}
```

`register_templates()` 改为注册两个 provider：

```rust
pub fn register_templates() -> Vec<ProviderTemplate> {
    vec![minimax_template(), kimi_template()]
}
```

### 3.2 向导流程不变

用户选择 Provider 后进入模型选择页（Kimi 只有 1 个模型，但仍需显示选中确认），然后是 API Key 输入，最后是 Alias 输入。

### 3.3 别名生成

Kimi 实例生成的 alias 示例：

```zsh
alias cl-kimi='ANTHROPIC_BASE_URL=https://api.kimi.com/coding/ ANTHROPIC_AUTH_TOKEN=sk-xxx ANTHROPIC_MODEL=kimi-for-coding claude'
```

环境变量合并顺序不变：`template.default_env` → `model.env_overrides` → `ANTHROPIC_AUTH_TOKEN`（由调用方注入）。

### 3.4 无需改动的地方

- `shell.rs` — 已处理任意数量的 provider
- `domain/model.rs` — `ProviderTemplate` 和 `ModelTemplate` 已支持任意数量字段
- UI 代码 — 列表渲染、帮助栏、创建向导均已泛化处理 provider 数量
- `sqlite_impl.rs` — schema 无需变更

## 4. 测试

- 手动运行 TUI，验证列表中能看到 Kimi 选项
- 验证能成功创建 Kimi 实例并激活
- 验证 `~/.cc-switch-tui/aliases.zsh` 中生成正确的 alias 行
