# Spec: 修复环境变量隔离问题

## Objective

**问题**: 当前 alias 切换时，旧环境变量不会被清除，导致错误配置影响后续使用。

**场景**:
1. 用户先激活 `cl-mini`，设置了 `ANTHROPIC_MODEL=MiniMax-M2.7-highspeed` 等多个变量
2. 用户切换到 `cl-kimi`，只设置了 `ANTHROPIC_BASE_URL` 等少数变量
3. `ANTHROPIC_MODEL` 等旧变量仍然残留，导致 kimi 使用了错误配置

**目标**: 每次切换时，先 unset 所有相关环境变量，再设置新值，确保环境完全隔离。

---

## 技术方案

### 方案：将 alias 改为 shell 函数

**当前格式 (alias)**:
```zsh
alias cl-mini='ANTHROPIC_AUTH_TOKEN=xxx ANTHROPIC_BASE_URL=xxx ... claude'
```

**新格式 (函数)**:
```zsh
cl-mini() {
  unset ANTHROPIC_AUTH_TOKEN ANTHROPIC_BASE_URL ANTHROPIC_MODEL ...
  export ANTHROPIC_AUTH_TOKEN=xxx ANTHROPIC_BASE_URL=xxx ...
  command claude
}
```

**关键设计点**:

1. **unset 变量列表**: 收集所有模板的 `default_env` 和 `env_overrides` 的 key，加上 `ANTHROPIC_AUTH_TOKEN` 和 `CMUX_PRESERVE_CLAUDE_AUTH_SELECTION_ENV`

2. **使用 `command claude`**: 确保调用的是 `claude` binary，避免函数自身递归调用

3. **函数调用方式不变**: 用户仍然使用 `cl-mini`，无需改变使用习惯

---

## 涉及变量清单

需要 unset 的完整变量列表（按来源）:

| 来源 | 变量 |
|------|------|
| minimax default_env | `ANTHROPIC_BASE_URL`, `ANTHROPIC_DEFAULT_HAIKU_MODEL`, `ANTHROPIC_DEFAULT_OPUS_MODEL`, `ANTHROPIC_DEFAULT_SONNET_MODEL`, `API_TIMEOUT_MS`, `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` |
| minimax env_overrides | `ANTHROPIC_MODEL` |
| kimi default_env | `ANTHROPIC_BASE_URL` |
| 全局固定 | `ANTHROPIC_AUTH_TOKEN`, `CMUX_PRESERVE_CLAUDE_AUTH_SELECTION_ENV` |

**完整 unset 列表**:
```
ANTHROPIC_AUTH_TOKEN ANTHROPIC_BASE_URL ANTHROPIC_DEFAULT_HAIKU_MODEL ANTHROPIC_DEFAULT_OPUS_MODEL ANTHROPIC_DEFAULT_SONNET_MODEL ANTHROPIC_MODEL API_TIMEOUT_MS CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC CMUX_PRESERVE_CLAUDE_AUTH_SELECTION_ENV
```

---

## 涉及文件变更

### `src/shell.rs`

**修改内容**:

1. 新增 `get_all_env_vars()` 函数 — 收集所有需要 unset 的变量
2. 修改 `format_function()` — 生成函数格式而非 alias 格式
3. 修改 `generate_aliases()` — 调用函数生成逻辑

**输出格式对比**:

**旧 (alias)**:
```zsh
alias cl-mini='ANTHROPIC_AUTH_TOKEN=xxx ANTHROPIC_BASE_URL=xxx claude'
```

**新 (function)**:
```zsh
cl-mini() {
  unset ANTHROPIC_AUTH_TOKEN ANTHROPIC_BASE_URL ...
  export ANTHROPIC_AUTH_TOKEN=xxx ANTHROPIC_BASE_URL=xxx ...
  command claude
}
```

---

## 兼容性考虑

1. **旧版 aliases.zsh**: 用户需要 `source ~/.zshrc` 或重新生成 alias 文件
2. **函数命名**: 确保不会与现有命令冲突
3. **向后兼容**: `claude` 命令的 `command claude` 调用确保调用 binary

---

## 验收标准

- [ ] 生成的 aliases.zsh 包含函数格式（而非 alias）
- [ ] 每个函数都先 unset 所有相关变量，再 export 新值
- [ ] 使用 `command claude` 调用 claude binary
- [ ] 切换 alias 后环境变量完全隔离，无残留
- [ ] 测试用例通过

---

## Open Questions

无

---

## Tech Stack

- Rust (无新依赖)
- Shell (zsh)

## Commands

```bash
# 构建
cargo build --release

# 测试
cargo test

# 本地测试 aliases.zsh 生成
cargo run --release
```
