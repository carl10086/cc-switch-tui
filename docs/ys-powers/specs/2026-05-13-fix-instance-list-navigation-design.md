# Spec: 修复实例列表导航在多 alias 场景下失效

## Objective

**问题**：`cargo run` 后进入主界面，同一个 model（如 kimi/kimi-for-coding）下有 2 个 alias 不同的实例时，UI 显示这 2 个实例但 ↑/↓ 键无法在它们之间切换；跨不同 (template, model) 的实例之间可以切换。

**根因**：`App::get_sorted_instances`（`src/app/state.rs:143-155`）的查找逻辑与 multi-alias 数据模型不兼容：

```rust
// 当前错误实现
let id = format!("{}-{}", template.id, model.id);
if let Some(instance) = self.dao.get_instance(&id) {
    result.push(instance);
}
```

但真实 `instance.id = "{template_id}-{model_id}-{alias}"`（由 `submit_create` 拼装，参见 `state.rs:370`），所以每个 (template, model) 最多匹配 0 或 1 条记录。`handle_list` 的 ↑/↓ 上下界依赖 `get_sorted_instances().len()`，于是同 model 的多个 alias 实例之间被卡死。

**上游 spec 关系**：本 fix 是 [`2026-05-13-multi-alias-per-model-design.md`](2026-05-13-multi-alias-per-model-design.md) 落地遗漏的补丁。该 spec 引入了 `{template}-{model}-{alias}` 新 id 格式并修了 `submit_create`、`draw_instance_list`、`rename_instance`，但漏了 `get_sorted_instances`。

**验收标准**：
- [ ] `get_sorted_instances` 返回所有 instance（不再丢失同 model 多 alias 的实例）
- [ ] UI 渲染与导航游标基于同一个有序列表（flat_index ≡ list_index）
- [ ] 同 (template, model) 内按 `created_at` 升序排序
- [ ] cargo test 全绿
- [ ] cargo run 后可在 kimi 下 2 个实例之间上下切换

---

## 修改点

### 改动 1：`src/app/state.rs::get_sorted_instances`

```rust
pub fn get_sorted_instances(&self) -> Vec<&ProviderInstance> {
    let templates = self.dao.get_templates();
    let instances = self.dao.list_instances();
    let mut result = Vec::new();
    for template in templates {
        for model in &template.models {
            let mut group: Vec<&ProviderInstance> = instances.iter()
                .filter(|i| i.template_id == template.id && i.model_id == model.id)
                .copied()
                .collect();
            group.sort_by_key(|i| i.created_at);
            result.extend(group);
        }
    }
    result
}
```

排序契约：**按 `templates` 顺序 → 按 `template.models` 顺序 → 同 (template, model) 内按 `created_at` 升序**。

### 改动 2：`src/ui/list.rs::draw_instance_list`

改用 `app.get_sorted_instances()` 作为唯一数据源，遍历时用 `last_template_id` 状态变量在 template 切换处插入分组标题。flat_index 与 `app.list_index` 一一对应，永不漂移。

伪代码：

```rust
let sorted = app.get_sorted_instances();
let templates = app.dao.get_templates();
let mut last_template_id: Option<String> = None;

for (flat_index, instance) in sorted.iter().enumerate() {
    if last_template_id.as_deref() != Some(instance.template_id.as_str()) {
        // 找 template 渲染分组标题
        last_template_id = Some(instance.template_id.clone());
    }
    let is_selected = flat_index == app.list_index;
    // 渲染 instance 行
}
```

### 改动 3：`src/domain/instance.rs` 注释更新

```rust
// 修改前（已过时）
/// 实例唯一标识，格式为 "template_id-model_id"

// 修改后
/// 实例唯一标识，格式为 "template_id-model_id-alias"
```

### 不动的部分

- `Dao` trait 及 `memory_impl` / `sqlite_impl` 实现（契约不变）
- `handle_list`、`current_instance`、`handle_delete_confirm` 中 `list_index` 越界回退（本就走 `get_sorted_instances`，自动正确）
- `submit_create` 中 `self.list_index = self.get_sorted_instances().len().saturating_sub(1)`（自动正确）
- 迁移脚本 `tools/migrate_instances_id.rs`（数据迁移与本 fix 无关）

---

## Project Structure

```
cc-switch-tui/
├── src/
│   ├── app/state.rs    ← 改动 1
│   └── ui/list.rs      ← 改动 2
└── docs/ys-powers/specs/
    └── 2026-05-13-fix-instance-list-navigation-design.md  ← 本文件
```

---

## Testing Strategy

### 单元测试（新增到 `src/app/state.rs` 的 `#[cfg(test)] mod tests`）

```
test_get_sorted_instances_groups_by_template_then_created_at
  setup: 创建 2 个 kimi 实例 + 1 个 minimax 实例（created_at 错乱）
  assert: result.len() == 3
  assert: result[0].template_id == "minimax"（template 顺序优先）
  assert: result[1..3] 都是 kimi 且按 created_at 升序

test_get_sorted_instances_empty_when_no_instances
  assert: 空 dao → 返回 []

test_get_sorted_instances_handles_multiple_aliases_same_model
  setup: 同一 (kimi, kimi-for-coding) 下 2 个不同 alias 实例
  assert: 两个都在返回结果中
  目的: 回归保护，直接复现用户报告的 bug
```

### 手动验证

```
1. cargo test                              → 全绿
2. cargo run                               → 进入 TUI
3. 进入主界面，确认 kimi 下 2 个实例都显示
4. 按 ↓ → 高亮在 2 个 kimi 实例之间移动 ✓
5. 按 ↑ → 反向移动 ✓
6. minimax ↔ kimi 跨组移动 → 仍然正常 ✓
```

---

## Code Style

- 遵循现有 Rust 风格（rustfmt 默认配置）
- `get_sorted_instances` 保留 `pub` 可见性（外部依赖未变）
- 不引入新依赖
- 测试用 `MemoryDaoImpl`，与现有测试一致

---

## Boundaries

**Always do**：
- 保留 multi-alias 数据模型的所有现有不变量
- 保持 `Dao` trait 契约不变
- 通过 `cargo test` 与手动验证双保险

**Ask first**：
- 任何对 `Dao` trait 或迁移逻辑的改动
- UI 分组渲染规则的进一步调整（如增删 template 标题、改色）

**Never do**：
- 不删除旧 id 格式的历史记录（`kimi-kimi-for-coding` 这类旧记录依然由 `list_instances()` 返回，本 fix 后也会被排序展示）
- 不在本次 fix 内重构 UI 主题/帮助栏/事件分发
- 不强制要求 alias 非空（保持 `multi-alias-per-model` spec 的兼容策略）

---

## Open Questions

无。设计已闭环：方案 A 已由用户确认；修改点、测试、边界全部明确。
