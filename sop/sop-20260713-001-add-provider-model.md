---
title: "SOP: 添加新 provider model（Kimi highspeed 默认 model 集成）"
created: 2026-07-13
tags: [feature, other, provider-template, 2026-07-13]
project: cc-switch-tui
---

## 背景

在 cc-switch-tui 项目中，第三方 Claude Code provider（MiniMax、Kimi 等）通过
`src/templates.rs` 内置的 `ProviderTemplate` 切换。当 provider 后端新增档位
（如 Kimi `kimi-for-coding-highspeed`、MiniMax 未来新档位），需要把新 model 加进
对应模板；如要设为默认 model，则放在 models 列表首位。

本 SOP 基于 2026-07-13 Kimi highspeed 集成（commit `c6ecc9c` → release v0.7.0）的
完整实践。

## 解决方案

### 伪代码步骤

1. 读 `src/templates.rs::{provider}_template()`，确认现有 models 列表结构、
   `env_overrides` 模式、是否需要 `CLAUDE_CODE_AUTO_COMPACT_WINDOW`
2. 把目标档位以官方 stable alias 形式加入 `models` 列表的**首位**（如需设默认）
3. 构造该档位独立的 `env_overrides`（用 `src/templates.rs` 的两个 helper）：
   - `routing_env_vars(model_id)` —— 6 个模型路由槽位（MODEL / HAIKU / OPUS /
     SONNET / FABLE / SUBAGENT）全部指向该档位 model id
   - `context_env_vars(auto_compact_window, max_context_tokens)` —— 两个值语义不同：
     `CLAUDE_CODE_AUTO_COMPACT_WINDOW` 是压缩触发阈值，`CLAUDE_CODE_MAX_CONTEXT_TOKENS`
     是模型真实窗口（走自定义 base URL 时 Claude Code 不会自己认出来，必须显式声明）
   - provider 支持思考档位时再追加 `CLAUDE_CODE_EFFORT_LEVEL`
   - 不与现有 model 共享 HashMap（避免默认切换互相污染）
4. 其余字段（id / name / default_env）**不动**
6. 在 `src/shell.rs::tests` 新增单测，仿 `test_aliases_contain_minimax_m3_1_flash_model_id`：
   - 构造 instance（template_id + model_id 用新档位）
   - 调 `generate_aliases()`
   - 断言 aliases.zsh 含新档位 model id 字符串
7. RED 步：跑该单测，确认 fail（模板未改前 model id 不在输出中）
8. GREEN 步：实现模板改动 → 跑单测 pass
9. 跑 `cargo test --lib` 验证无回归；`cargo build` 验证编译
10. /ys-review 5 维度审查
11. /gc：建 `feature/{keyword}-{MMDD}` 分支 → commit → push → PR → merge
12. /make-release：用户确认 version bump → 前置 7 项 → bump + tag + publish

### 关键信息

- src/templates.rs
  - function kimi_template()
  - function minimax_template()
  - function register_templates()
- src/shell.rs
  - function generate_aliases()
  - tests::test_aliases_contain_kimi_for_coding_highspeed_model_id
  - tests::test_aliases_contain_minimax_m3_1_flash_model_id（参考样板）
- docs/codebase/ARCHITECTURE.md
  - §5 Infrastructure（不写 model 列表，只描述 templates 层职责）

### 关键命令

```bash
# 单测
cargo test --lib shell::tests::test_aliases_contain_{new_model_test_name}
cargo test --lib   # 全套

# /gc
git checkout -b feature/{keyword}-{MMDD}
git add src/templates.rs src/shell.rs
git commit -m "feat({provider}): add {model_id} as default model"
git push -u origin HEAD

# /make-release
make build
make tag       # git tag v{X}
make release   # git push origin v{X}
make publish   # gh release create + upload
gh release view v{X} --json assets,url --jq '.url, (.assets[].name)'  # 二次验证
```

### 关键决策

- **per-model 独立 `env_overrides` HashMap**：不共享；即使两个 model 用同一组
  4 个 env var 也复制一份。理由：未来若档位差异扩大（如加 auto-compact window），
  共享会引入耦合；项目现有 minimax 模式已是如此。
- **默认 model 放 models[0]**：UI 下拉默认选中第一个；这是用户产品决策，
  必须在 /clarify-intent 显式确认。
- **窗口 env 分两个值**：`CLAUDE_CODE_AUTO_COMPACT_WINDOW`（压缩阈值）与
  `CLAUDE_CODE_MAX_CONTEXT_TOKENS`（真实窗口）不可绑成同一个值。例：MiniMax
  M3.1-Flash 真实窗口 1M，但官方推荐在 512K 处压缩 → `context_env_vars(524288, 1048576)`。
- **不更新 ARCHITECTURE.md / CLAUDE.md**：项目惯例——这两个文档只描述抽象字段，
  不写 model 列表。/clarify-intent 时主动确认是否需要更新。
- **不写 intent 文档**：feature commit body + plan 文件
  `docs/ys-powers/plans/{YYYY-MM-DD}-{keyword}.md` 已足够留档。
- **测试只 assert "contains"，不验证出现次数**：仿 M3[1m] 最小形态；测试目的
  是确认 model id 被注入，不是验证导出格式。
- **OpenCode 集成已于 2026-10 删除**：模板不再有 `opencode_*` 字段，新增 model
  不需要也不应该碰任何 opencode 相关代码。