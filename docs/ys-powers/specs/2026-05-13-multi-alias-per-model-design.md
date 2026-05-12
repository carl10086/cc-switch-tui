# Spec: 允许多别名 per Model

## Objective

**问题**: 当前设计一个 model 只能创建一个实例，因为实例 ID 格式为 `{template_id}-{model_id}`， uniqueness 约束导致无法创建多个。

**目标**: 允许同一 model 通过不同 alias 创建多个实例，每个实例有独立的 API Key 和别名。

**用户场景**:
- 用户 A 有两个 kimi 账号，想用 `cl-kimi-pro` 和 `cl-kimi-lite` 分别切换
- 用户 B 想用同一个 kimi model 测试不同配置

**验收标准**:
- [ ] 同一 model 可以创建多个实例，只要 alias 不同
- [ ] 旧数据通过迁移脚本升级到新 ID 格式
- [ ] 迁移脚本支持 dry-run 验证
- [ ] 迁移后数据完整性验证

---

## Instance ID 变更

**旧格式**: `{template_id}-{model_id}`，例如 `kimi-kimi-for-coding`
**新格式**: `{template_id}-{model_id}-{alias}`，例如 `kimi-kimi-for-coding-cl-kimi-pro`

**示例对比**:
| template_id | model_id | alias | 旧 ID | 新 ID |
|-------------|----------|-------|-------|-------|
| kimi | kimi-for-coding | cl-kimi-pro | kimi-kimi-for-coding | kimi-kimi-for-coding-cl-kimi-pro |
| kimi | kimi-for-coding | cl-kimi-lite | (冲突) | kimi-kimi-for-coding-cl-kimi-lite |

---

## 迁移方案

### 迁移脚本: `tools/migrate_instances_id.rs`

**执行时机**: 用户主动运行，非自动

**迁移逻辑**:
1. 备份原数据库 → `~/.cc-switch-tui/db.sqlite.backup.{timestamp}`
2. 扫描所有 alias 非空的记录
3. 计算新 ID = `{template_id}-{model_id}-{alias}`
4. 检测冲突：新 ID 已存在则报错退出
5. 更新记录

**CLI 参数**:
```bash
cargo run --release --bin migrate_instances_id [OPTIONS]
    --dry-run          # 只打印迁移 SQL，不执行
    --backup           # 执行前备份（默认开启）
    --db-path <PATH>   # 指定数据库路径（默认 ~/.cc-switch-tui/db.sqlite）
```

**退出码**:
- 0: 成功或 dry-run 无需迁移
- 1: 迁移失败/冲突

**注意**: alias 为空的旧记录保持不变，等用户编辑后自动更新 ID

---

## 涉及文件变更

### 1. `src/app/state.rs`

**变更位置 1**: `submit_create()` 函数

```rust
// 修改前
let id = format!("{}-{}", template_id, model_id);

// 修改后（alias 在此时已确定）
let id = format!("{}-{}-{}", template_id, model_id, alias);
```

**变更位置 2**: `handle_edit_field()` 中的 alias 变更逻辑

当用户修改 alias 时，需要同步更新 id：

```rust
// 修改前：只更新 alias
EditField::Alias => {
    if let Err(e) = self.validate_alias(&value) {
        Err(e)
    } else {
        self.dao.set_alias(&instance_id, value)
    }
}

// 修改后：alias 变更时同步更新 id
EditField::Alias => {
    if let Err(e) = self.validate_alias(&value) {
        Err(e)
    } else {
        let old_instance = self.dao.get_instance(&instance_id)
            .ok_or(AppError::InstanceNotFound(instance_id.clone()))?;
        let new_id = format!("{}-{}-{}", old_instance.template_id, old_instance.model_id, value);
        self.dao.rename_instance(&instance_id, &new_id, value)
    }
}
```

需要新增 DAO 方法 `rename_instance`（同时更新 id 和 alias）。

### 2. `src/ui/list.rs`

**变更位置**: `draw_instance_list()` 函数

```rust
// 修改前：按固定 id 格式查找
for template in templates {
    for m in &template.models {
        let id = format!("{}-{}", template.id, m.id);
        if let Some(instance) = app.dao.get_instance(&id) {
            // ...
        }
    }
}

// 修改后：直接遍历该 template 下的所有实例
for instance in app.dao.list_instances() {
    if instance.template_id == template.id {
        // ...
    }
}
```

### 3. `src/shell.rs`

**变更位置**: 测试用例中实例 ID 更新为新格式

```rust
// test_generate_aliases_content 中的 instance.id
id: "minimax-MiniMax-M2.7-highspeed-cl-mini".to_string(),  // 新格式
```

### 4. `tools/migrate_instances_id.rs` (新增)

独立 binary，负责数据库迁移

### 5. `src/dao/mod.rs`

新增 `rename_instance` 方法：

```rust
/// 重命名实例（同时更新 id 和 alias）
fn rename_instance(&mut self, old_id: &str, new_id: &str, alias: String) -> Result<(), AppError>;
```

### 6. `src/dao/memory_impl.rs` & `src/dao/sqlite_impl.rs`

实现 `rename_instance` 方法：
- 删除旧 id 的记录
- 插入新 id 的记录（alias 更新为新值）
- 若新 id 已存在，返回 `InstanceAlreadyExists` 错误

---

## Project Structure

```
cc-switch-tui/
├── src/
│   ├── app/state.rs      # ID 生成逻辑变更
│   ├── ui/list.rs        # 列表遍历逻辑变更
│   └── shell.rs          # 测试用例更新
├── tools/
│   └── migrate_instances_id.rs  # 新增迁移脚本
└── docs/
    └── ys-powers/specs/
        └── 2026-05-13-multi-alias-per-model-design.md
```

---

## Testing Strategy

### 1. 迁移脚本测试

```bash
# dry-run 测试（不修改数据）
cargo run --release --bin migrate_instances_id --dry-run

# 在临时数据库上验证
cp ~/.cc-switch-tui/db.sqlite /tmp/test.sqlite
cargo run --release --bin migrate_instances_id --db-path /tmp/test.sqlite --dry-run
```

### 2. 单元测试

```bash
cargo test
```

- `test_generate_aliases_content`: 验证新 ID 格式生成的 alias 正确
- `test_create_instance_duplicate`: 验证同 alias 重复创建被拒绝

### 3. 集成测试（手动）

1. 备份当前数据库
2. 运行迁移脚本
3. 启动 TUI，验证所有实例正常显示
4. 验证 alias 切换功能正常

---

## Boundaries

**Always do**:
- alias 必须以 `cl-` 开头
- alias 唯一性检查
- 迁移前自动备份
- 迁移后验证数据完整性

**Ask first**:
- 数据库 schema 变更
- 添加新的 DAO 实现

**Never do**:
- 自动删除旧数据库（保留备份）
- 在未验证的情况下强制覆盖已有 alias

---

## Open Questions

~~1. **迁移时机**: 是否需要在 TUI 首次启动时自动检测并提示用户运行迁移脚本？~~ → 已确认：用户手动运行迁移脚本

~~2. **别名变更时 ID 更新**: 当用户修改 alias 时，是否同步更新 id？~~ → **已确认：是，alias 变更时 id 随之改变**
