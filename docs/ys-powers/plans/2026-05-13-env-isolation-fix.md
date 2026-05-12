# Implementation Plan: 修复环境变量隔离问题

## Overview

将 `aliases.zsh` 从 alias 格式改为函数格式，解决环境变量残留导致的 bug。

## 架构决策

| 决策 | 理由 |
|------|------|
| 使用 shell 函数代替 alias | 可以在函数内 unset 环境变量后再 export |
| 使用 `command claude` | 避免函数名与 claude binary 冲突导致递归调用 |
| 收集所有模板变量到 unset 列表 | 确保任何模板切换时都能清除所有可能的环境变量 |

---

## Task List

### Phase 1: 核心实现

#### Task 1: 新增 `get_all_env_vars()` 辅助函数

**Description:** 收集所有模板的 `default_env` 和 `env_overrides` 的 key，生成完整的 unset 变量列表。

**Acceptance criteria:**
- [ ] 返回包含所有环境变量 key 的 `Vec<String>`
- [ ] 包含 `ANTHROPIC_AUTH_TOKEN` 和 `CMUX_PRESERVE_CLAUDE_AUTH_SELECTION_ENV`

**Dependencies:** None

**Files likely touched:**
- `src/shell.rs`

**Estimated scope:** XS

---

#### Task 2: 修改 `format_alias_cmd()` 为 `format_function()`

**Description:** 将输出格式从 alias 改为函数。

**Acceptance criteria:**
- [ ] 生成函数格式：`name() { unset ...; export ...; command claude }`
- [ ] 函数体内先 unset 所有变量，再 export 新值
- [ ] 使用 `command claude` 调用 binary

**Dependencies:** Task 1

**Files likely touched:**
- `src/shell.rs`

**Estimated scope:** S

---

#### Task 3: 修改 `generate_aliases()` 调用新格式

**Description:** 更新 `generate_aliases()` 调用新的函数格式化逻辑。

**Acceptance criteria:**
- [ ] 生成的文件使用函数格式
- [ ] 同时生成 `claude` 函数（当前选中的实例）

**Dependencies:** Task 2

**Files likely touched:**
- `src/shell.rs`

**Estimated scope:** XS

---

### Phase 2: 测试与验证

#### Task 4: 更新测试用例

**Description:** 更新 `shell.rs` 中的测试用例，验证新格式输出。

**Acceptance criteria:**
- [ ] 测试验证生成的函数包含 `unset` 和 `export`
- [ ] 测试验证使用 `command claude`

**Dependencies:** Task 3

**Files likely touched:**
- `src/shell.rs`

**Estimated scope:** XS

---

### Checkpoint: 核心功能完成

- [ ] `cargo test` 全部通过
- [ ] cargo build --release 编译成功
- [ ] 生成的 aliases.zsh 使用函数格式

---

### Phase 3: 手动验证

#### Task 5: 手动集成验证

**Description:** 在真实环境中测试 alias 切换是否正确隔离环境变量。

**Acceptance criteria:**
- [ ] 启动 TUI，生成新的 aliases.zsh
- [ ] `source ~/.zshrc` 加载新配置
- [ ] 测试 `cl-mini` 后 `cl-kimi` 切换，验证环境变量正确清除

**Dependencies:** Task 4

**Verification:**
- 使用 `env | grep ANTHROPIC` 检查残留变量

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| 用户未重新 source aliases.zsh | 低 | 旧配置仍可使用，只是不会自动清除变量 |
| 函数名与系统命令冲突 | 低 | cl- 前缀减少冲突概率 |

---

## 执行顺序

```
Task 1 → Task 2 → Task 3 → Task 4 → Checkpoint → Task 5
```

---

## Open Questions

无
