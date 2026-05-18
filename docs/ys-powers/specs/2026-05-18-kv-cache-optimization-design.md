# KV Cache 优化功能设计

## 1. 目标

为每个 provider 实例提供 KV Cache 优化开关，使本地 llama.cpp 模型能实现更高的 KV cache 命中率，减少重复 prefill，提升响应速度。

## 2. 实现方案

### 2.1 数据模型变更

**`src/domain/instance.rs`**

在 `ProviderInstance` 结构体中增加字段：

```rust
pub struct ProviderInstance {
    // ... 现有字段 ...
    /// 是否启用 KV Cache 优化（默认 false）
    pub kv_cache_enabled: bool,
}
```

**`src/dao/sqlite_impl.rs`**

- `instances` 表增加 `kv_cache_enabled INTEGER DEFAULT 0` 列
- 在 `insert_instance` 和 `update_instance` 方法中处理该字段

### 2.2 UI 变更

**`src/ui/edit.rs`**

- 在编辑详情界面（`EditInfoPanel`）增加 checkbox
- 标签：`启用 KV Cache 优化（本地模型）`
- 默认值：`false`（关闭）
- 位置：在 alias、API Key、OpenCode Model 字段下方

### 2.3 Alias 生成逻辑变更

**`src/shell.rs`（新建或扩展）**

当 `kv_cache_enabled = true` 时，`cl-xxx` alias 追加以下参数：

```bash
--exclude-dynamic-system-prompt-sections \
--settings '{"includeGitInstructions":false}'
```

现有 `cl-xxx` alias 生成逻辑在 `src/opencode_config.rs` 的 `build_opencode_aliases` 函数中，需要提取 `cl-xxx` 部分并增强。

### 2.4 KV Cache 优化参数说明

| 参数 | 作用 |
|------|------|
| `--exclude-dynamic-system-prompt-sections` | 将动态信息移出 system prompt，保持前缀稳定 |
| `--settings '{"includeGitInstructions":false}'` | 禁用内置 git 指令，减少动态内容 |

这两个参数共同作用，使 system prompt 静态化，从而实现 KV cache 高命中率。

## 3. 文件变更清单

| 文件 | 变更类型 | 说明 |
|------|----------|------|
| `src/domain/instance.rs` | 修改 | 增加 `kv_cache_enabled` 字段 |
| `src/dao/sqlite_impl.rs` | 修改 | 表结构和方法增加该字段处理 |
| `src/ui/edit.rs` | 修改 | 编辑界面增加 checkbox |
| `src/shell.rs` | 新增/修改 | `cl-xxx` alias 生成逻辑增加参数追加 |
| `src/opencode_config.rs` | 修改 | 协调 alias 生成 |

## 4. 边界条件

- `kv_cache_enabled = false`（默认）：行为与现在完全一致
- `kv_cache_enabled = true`：仅对本地 llama.cpp provider 有实际效果，用户需自行判断
- 字段不存在（旧数据库迁移）：默认视为 `false`

## 5. 测试策略

- 手动测试：开启/关闭开关，验证生成的 alias 内容正确
- 验证 `cl-xxx` alias 在两种状态下都能正常执行
