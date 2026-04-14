# cc-switch-tui TUI 交互设计文档

## 1. 设计目标

构建一个基于 Rust `ratatui + crossterm` 的终端用户界面，用于管理 Claude Code 的 Provider 实例。当前阶段 TUI 仅作为**实例管理器**，负责新增、编辑、删除实例，不处理"切换"逻辑和 env.sh 生成。

## 2. 核心原则

- **只做管理**：列出、新增、编辑 api-key、删除实例
- **无"当前激活"状态**：TUI 不维护哪个实例正在生效的状态
- **按 provider 分组展示**：列表左侧按 provider 分组列出所有已创建实例
- **右侧信息面板**：展示当前高亮实例的完整环境变量和详情
- **底部帮助栏**：始终显示可用按键提示

## 3. 界面布局

```
┌─────────────────────────┬──────────────────────┐
│ [minimax]               │ ID: minimax-M2.7     │
│   MiniMax M2.7 Highspeed│ Provider: MiniMax    │
│                         │ Model: M2.7 Highspeed│
│                         │ API Key: tes*******  │
│                         │ BASE_URL: ...        │
│                         │ MODEL: ...           │
│                         │ TIMEOUT: 3000000     │
│                         │                      │
│                         │ n:新建 e:编辑 d:删除 │
│                         │ ↑↓:移动 q:退出       │
└─────────────────────────┴──────────────────────┘
```

### 3.1 左侧：实例列表
- 按 provider 分组（类似目录树或带分组标题的列表）
- 只展示**已创建的实例**
- 内置但未创建实例的 provider **不显示**
- 光标高亮当前选中的实例

### 3.2 右侧：信息面板
- 展示当前高亮实例的元数据：
  - `ID`（template_id-model_id）
  - `Provider` 名称
  - `Model` 名称
  - `API Key`（部分隐藏，如 `tes*******`）
  - 完整的环境变量键值对（default_env + env_overrides 合并后）

### 3.3 底部：帮助栏
- 常驻显示当前可用按键：
  - `↑/↓` 移动
  - `n` 新建
  - `e` 编辑 api-key
  - `d` 删除
  - `q` 退出

## 4. 按键操作

| 按键 | 行为 |
|------|------|
| `↑/↓` | 在实例列表中上下移动 |
| `n` | 进入新建实例向导 |
| `e` | 编辑当前高亮实例的 api-key（弹出独立编辑页） |
| `d` | 删除当前高亮实例（弹出确认对话框 `[Y/N]`） |
| `q` | 退出 TUI |
| `Esc` | 在弹窗/向导中取消并返回上一页 |

## 5. 新建实例流程（按 `n`）

```
[ProviderList] → 按 n
       │
       ▼
[Provider 选择页]
   下拉列表展示内置 Provider 模板
       │ Enter 确认
       ▼
[Model 选择页]
   下拉列表展示该 Provider 的 Models
       │ Enter 确认
       ▼
[API Key 输入页]
   单行文本输入框
       │ Enter 确认
       ▼
[创建成功] → 返回主界面，新实例出现在列表中
```

- 创建时的 `id` 自动生成为 `template_id-model_id`
- 如果该组合已存在，提示"实例已存在"并返回

## 6. 编辑实例流程（按 `e`）

```
[主界面] → 按 e
       │
       ▼
[编辑页面]
   显示当前实例信息 + API Key 输入框
   用户修改 API Key
       │ Enter 保存 / Esc 取消
       ▼
[返回主界面]
```

- 仅允许修改 `api_key`
- `provider` 和 `model` 不可修改（如需要变更，应删除后重建）

## 7. 删除实例流程（按 `d`）

```
[主界面] → 按 d
       │
       ▼
[确认对话框]
   "确定删除 minimax 的实例吗？ [Y/N]"
       │ Y 确认 / N 或 Esc 取消
       ▼
[返回主界面]
```

- 删除后从列表中移除
- 取消删除则列表状态不变

## 8. 状态管理

### App 状态机

```rust
pub enum AppState {
    /// 主界面：实例列表 + 信息面板
    List,
    /// 新建向导：选择 Provider
    CreateProvider,
    /// 新建向导：选择 Model
    CreateModel { template_id: String },
    /// 新建向导：输入 API Key
    CreateApiKey { template_id: String, model_id: String },
    /// 编辑页面
    Edit { instance_id: String },
    /// 删除确认对话框
    DeleteConfirm { instance_id: String },
}
```

### 输入状态

- `List`：列表光标索引 `list_index`
- `CreateProvider`：`provider_index` 在下拉列表中的位置
- `CreateModel`：`model_index` 在下拉列表中的位置
- `CreateApiKey`：`input` 字符串 + 光标位置
- `Edit`：`input` 字符串 + 光标位置（预填充当前 api-key）

## 9. 数据依赖

TUI 层通过 `Dao` trait 与数据层交互：
- 启动时从 `MemoryDaoImpl` 读取所有实例和模板
- 新增时调用 `dao.create_instance()`
- 编辑时调用 `dao.get_instance()` 读取 + 自定义更新逻辑（MemoryDaoImpl 暂不支持 `update`，可直接替换内部 HashMap 的值）
- 删除时调用 `dao.delete_instance()`

## 10. 依赖项

在 `Cargo.toml` 中新增：

```toml
[dependencies]
# 已有依赖...
crossterm = "0.27"
ratatui = "0.26"
```

## 11. 模块结构

```
src/
├── main.rs              # 入口
├── lib.rs               # 模块导出
├── app.rs               # App 状态、事件循环、页面路由
├── ui/
│   ├── mod.rs           # UI 渲染入口
│   ├── list.rs          # 主界面列表 + 信息面板
│   ├── create.rs        # 新建向导页面
│   ├── edit.rs          # 编辑页面
│   └── popup.rs         # 删除确认对话框
└── event.rs             # 键盘事件处理
```

## 12. 后续扩展

- TUI 中集成 `apply_config()` 的 env.sh 生成逻辑
- 增加"当前激活实例"的标记和切换操作
- 增加搜索/过滤实例功能
