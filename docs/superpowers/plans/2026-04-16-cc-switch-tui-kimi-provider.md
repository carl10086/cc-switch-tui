# Kimi Provider Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在 cc-switch-tui 中添加 Kimi provider，用户可创建 Kimi 实例并生成对应 shell alias。

**Architecture:** 在 `templates.rs` 新增 `kimi_template()` 函数，返回只包含一个模型 `kimi-for-coding` 的 provider 配置。`register_templates()` 改为返回两个 provider 向导列表。

**Tech Stack:** Rust / Ratatui TUI

---

### Task 1: 添加 Kimi provider 模板

**Files:**
- Modify: `src/app/templates.rs`
- Modify: `tests/template_test.rs`

- [ ] **Step 1: 修改测试，验证 Kimi provider 未注册（预期失败）**

修改 `tests/template_test.rs`，在 `test_minimax_template_registered` 下方新增一个测试：

```rust
#[test]
fn test_kimi_template_registered() {
    let templates = register_templates();
    assert_eq!(templates.len(), 2);

    let kimi = templates.iter().find(|t| t.id == "kimi").unwrap();
    assert_eq!(kimi.name, "Kimi");
    assert_eq!(
        kimi.default_env.get("ANTHROPIC_BASE_URL").unwrap(),
        "https://api.kimi.com/coding/"
    );
    assert_eq!(kimi.models.len(), 1);

    let model = &kimi.models[0];
    assert_eq!(model.id, "kimi-for-coding");
    assert_eq!(model.name, "Kimi for Coding");
    assert!(model.env_overrides.is_empty());
}
```

同时修改 `test_minimax_template_registered` 中的 `assert_eq!(templates.len(), 1);` 为 `assert_eq!(templates.len(), 2);`。

Run: `cargo test test_kimi_template_registered`
Expected: FAIL（Kimi 尚未实现）

- [ ] **Step 2: 添加 kimi_template 函数**

修改 `src/app/templates.rs`：

```rust
/// 构建 kimi Provider 模板
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

- [ ] **Step 3: 修改 register_templates 注册两个 provider**

将 `src/app/templates.rs` 的 `register_templates` 改为：

```rust
pub fn register_templates() -> Vec<ProviderTemplate> {
    vec![minimax_template(), kimi_template()]
}
```

- [ ] **Step 4: 运行测试验证**

Run: `cargo test test_kimi_template_registered`
Expected: PASS

Run: `cargo test test_minimax_template_registered`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/app/templates.rs tests/template_test.rs
git commit -m "feat: add Kimi provider support"
```

---

## Self-Review Checklist

**1. Spec coverage：**
- [x] Kimi provider 模板新增 — Task 1
- [x] `kimi-for-coding` 模型注册 — Task 1
- [x] `ANTHROPIC_BASE_URL` 正确设置 — Task 1
- [x] 测试验证 — Task 1

**2. Placeholder scan：**
- [x] 无 "TBD"、"TODO"

**3. Type一致性：**
- [x] `kimi_template()` 返回 `ProviderTemplate`，与 `minimax_template()` 签名一致
