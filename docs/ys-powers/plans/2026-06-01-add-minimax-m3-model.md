# Plan: 添加 MiniMax-M3 模型支持

## 依赖关系分析

```
┌─────────────────────────────┐
│  src/app/templates.rs       │  ← 核心改动（唯一源）
│  minimax_template()         │
└──────────────┬──────────────┘
               │
    ┌──────────┼──────────┐
    ▼          ▼          ▼
┌────────┐ ┌────────┐ ┌──────────────┐
│ tests/ │ │ src/   │ │ src/dao/     │
│template│ │shell.rs│ │sqlite_impl.rs│
│_test.rs│ │(tests) │ │(tests)       │
└────────┘ └────────┘ └──────────────┘
```

**关键发现：**
- `src/app/state.rs` 和 `tests/bug_repro_get_sorted.rs` 使用独立的 mock 模板（`m1` / `kimi-for-coding`），与 `register_templates()` 返回的真实模板无关，**无需修改**。
- `src/opencode_config.rs` 通过 `template.models` 和 `instance.opencode_model_id` 动态生成配置，**无需修改**（逻辑已正确）。

---

## 垂直切片（Vertical Slices）

### Slice 1: 核心模板修改

**目标：** 让 `minimax_template()` 返回包含 M3 和 M2.7-highspeed 的模板，M3 作为默认。

**文件：** `src/app/templates.rs`

**具体改动：**
1. `default_env` 中三个 `ANTHROPIC_DEFAULT_*_MODEL` 从 `MiniMax-M2.7-highspeed` → `MiniMax-M3`
2. `models` 列表改为包含两个 `ModelTemplate`：
   - `[0]` M3: `id="MiniMax-M3"`, `env_overrides={ANTHROPIC_MODEL: MiniMax-M3}`
   - `[1]` M2.7 highspeed: `id="MiniMax-M2.7-highspeed"`, `env_overrides={ANTHROPIC_MODEL: MiniMax-M2.7-highspeed}`

**验收标准：**
- `cargo build` 编译通过
- `register_templates()` 返回的 minimax provider 包含 2 个模型
- `minimax.models[0].id == "MiniMax-M3"`
- `minimax.default_env["ANTHROPIC_DEFAULT_OPUS_MODEL"] == "MiniMax-M3"`

**验证：** `cargo test template_test`（此时会失败，Slice 2 修复）

---

### Slice 2: 测试断言同步

**目标：** 更新所有与 minimax 模型相关的硬编码测试数据。

**文件清单：**

#### 2.1 `tests/template_test.rs`
- `models.len()` 从 `1` → `2`
- `model.id` 断言从 `MiniMax-M2.7-highspeed` → `MiniMax-M3`（第一个模型）
- `env_overrides` 断言同步更新

#### 2.2 `src/shell.rs`（测试代码，约 160-245 行）
- `ModelTemplate.id` 从 `MiniMax-M2.7-highspeed` → `MiniMax-M3`
- `ModelTemplate.name` 从 `"MiniMax M2.7 Highspeed"` → `"MiniMax M3"`
- `ProviderInstance.id` 从 `"minimax-MiniMax-M2.7-highspeed-cl-mini"` → `"minimax-MiniMax-M3-cl-mini"`
- `ProviderInstance.model_id` 同步更新
- `ProviderInstance.opencode_model_id` 同步更新
- **注意：** 两个测试用例（`test_generate_aliases_content` 和 `test_generate_aliases_contains_unset_vars`）包含相同的硬编码数据，需要同步更新。

#### 2.3 `src/dao/sqlite_impl.rs`（测试代码，约 280-455 行）
- 所有测试中的 `instance.id` 从 `"minimax-MiniMax-M2.7-highspeed"` → `"minimax-MiniMax-M3"`
- `instance.model_id` 同步更新
- `set_alias()` / `update_instance()` / `delete_instance()` / `rename_instance()` 调用时的 ID 参数同步

**验收标准：**
- `cargo test` 全部通过
- 无编译警告 (`cargo clippy`)

**验证：** `cargo test && cargo clippy`

---

## 任务分解

### Task 1: 修改核心模板
- **文件：** `src/app/templates.rs`
- **改动点：** `minimax_template()` 函数（约 10-54 行）
- **验收：** `cargo build` 通过，模板包含 2 个模型且 M3 为默认
- **验证：** `cargo build`

### Task 2: 更新 `tests/template_test.rs`
- **文件：** `tests/template_test.rs`
- **改动点：** 第 14、17-20 行的断言
- **验收：** `cargo test --test template_test` 通过
- **验证：** `cargo test --test template_test`

### Task 3: 更新 `src/shell.rs` 测试
- **文件：** `src/shell.rs`（测试模块）
- **改动点：** 两个测试用例中的硬编码 `ProviderTemplate` 和 `ProviderInstance`
- **验收：** `cargo test shell` 通过
- **验证：** `cargo test shell`

### Task 4: 更新 `src/dao/sqlite_impl.rs` 测试
- **文件：** `src/dao/sqlite_impl.rs`（测试模块）
- **改动点：** 所有使用 `"minimax-MiniMax-M2.7-highspeed"` 的测试数据
- **验收：** `cargo test dao` 通过
- **验证：** `cargo test dao`

### Task 5: 全量验证
- **验收：** `cargo test` 全部通过，`cargo clippy` 无警告
- **验证：** `cargo test && cargo clippy`

---

## 检查点（Checkpoints）

```
[CP1] Task 1 完成后
    → 确认编译通过
    → 确认模板结构正确（2 个模型，M3 默认）

[CP2] Task 2-4 完成后
    → 运行 `cargo test`，确认全部通过
    → 如有失败，定位是哪个测试文件的问题

[CP3] Task 5 完成后
    → 运行 `cargo clippy`，确认无警告
    → 计划完成，可进入实现阶段
```

---

## 风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 测试数据遗漏 | 测试失败 | 全局搜索 `"MiniMax-M2.7-highspeed"`，确保无残留 |
| opencode 配置未同步 | opencode alias 仍指向旧模型 | 验证 `opencode_config.rs` 逻辑，确认自动跟随 `instance.opencode_model_id` |
| 运行时模型 ID 不匹配 | API 返回 400 | 已通过 `curl` 验证 `MiniMax-M3` 可用，ID 大小写严格一致 |

---

## 无需改动的文件（确认）

以下文件使用独立 mock 数据，与 `register_templates()` 返回的真实模板无关：

- `src/app/state.rs` — `test_templates()` 使用 `id: "m1"` 作为 minimax 模型
- `tests/bug_repro_get_sorted.rs` — `test_templates()` 使用 `id: "m1"` 作为 minimax 模型
- `src/opencode_config.rs` — 动态读取 `template.models` 和 `instance.opencode_model_id`，无需硬编码修改
