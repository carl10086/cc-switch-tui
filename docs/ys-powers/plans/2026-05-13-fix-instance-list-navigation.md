# Plan: 修复实例列表导航在多 alias 场景下失效

## 依赖图

```
[src/domain/instance.rs] 注释更新 ──→ 独立，无外部依赖
                                   
[src/app/state.rs] get_sorted_instances ──→ [src/ui/list.rs] draw_instance_list
         │                                          │
         │（提供有序列表）                            │（消费有序列表）
         └────────────────────────────────────────────┘
                    flat_index ≡ list_index
```

**关键约束**：`state.rs` 改动必须在 `list.rs` 之前，因为 UI 渲染要消费新排序结果。

---

## 垂直切片（按完整路径拆分，不按层）

### 任务 1：核心排序修复 + 测试（`state.rs`）

**修改内容**：
- `src/app/state.rs:143-155`：`get_sorted_instances` 从 `"{}-{}"` 查单条改为 filter + sort_by_key(created_at)
- 新增 `#[cfg(test)] mod tests`（当前 `state.rs` 无测试模块），放 3 个测试

**验证步骤**：
```bash
cargo test --lib          # 旧测试不挂
cargo test --test bug_repro_get_sorted  # 复现测试从红变绿
```

**验收标准**：
- `app.get_sorted_instances().len() == app.dao.list_instances().len()`
- `sorted[0].template_id == "minimax"`（template 顺序优先）
- 同 (template, model) 内按 `created_at` 升序

---

### 任务 2：UI 渲染对齐（`list.rs`）

**修改内容**：
- `src/ui/list.rs:47-95`：`draw_instance_list` 数据源从 `list_instances()` 改为 `get_sorted_instances()`
- 用 `last_template_id: Option<String>` 检测 template 切换，插入分组标题
- 保留 model name 查找和选中样式逻辑不变

**验证步骤**：
```bash
cargo test                # 全绿
cargo run                 # 手动验证：kimi 下 2 个实例可上下切换
```

**验收标准**：
- `flat_index` 与 `app.list_index` 一一对应
- 同一 template 下多个 instance 的 group 内顺序与 `get_sorted_instances` 一致
- 跨 template 移动正常

---

### 任务 3：文档清理（`domain/instance.rs`）

**修改内容**：
- `src/domain/instance.rs:7`：doc comment `"template_id-model_id"` → `"template_id-model_id-alias"`

**验证步骤**：
```bash
cargo doc --no-deps       # 生成文档，确认注释正确
```

**验收标准**：注释与 `submit_create` 中 `format!("{}-{}-{}", template_id, model_id, alias)` 一致。

---

## 检查点

| 检查点 | 触发条件 | 验证命令 | 通过标准 |
|---|---|---|---|
| CP-1 | 任务 1 完成后 | `cargo test --lib && cargo test --test bug_repro_get_sorted` | 全绿，复现测试通过 |
| CP-2 | 任务 2 完成后 | `cargo test && cargo run` | 全绿；手动验证上下切换正常 |
| CP-3 | 全部完成后 | `cargo build --release` | 编译通过；准备提交 |

---

## 风险与回退

| 风险 | 影响 | 缓解 |
|---|---|---|
| `get_sorted_instances` 改完后 UI 仍用旧数据源 | 显示与导航仍不一致 | 任务 2 紧跟任务 1，CP-2 强制要求手动验证 |
| `list_instances()` 在其他地方被依赖 | 改 UI 渲染后其他地方行为变化 | 全局搜索 `list_instances` 调用，确认只有 UI 渲染和 shell::generate_aliases 用它，后者不改 |
| 旧 id 格式 `"template_id-model_id"` 无 alias | 排序时 alias 为空，created_at 决定顺序 | 兼容，空 alias 也是合法 instance |

---

## 引用

- Spec: [`../specs/2026-05-13-fix-instance-list-navigation-design.md`](../specs/2026-05-13-fix-instance-list-navigation-design.md)
- 上游 Spec: [`../specs/2026-05-13-multi-alias-per-model-design.md`](../specs/2026-05-13-multi-alias-per-model-design.md)
