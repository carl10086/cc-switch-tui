# cc-switch-tui tracing 日志设计文档

## 1. 设计目标

为 cc-switch-tui 引入结构化日志系统，方便在 TUI 独占终端的情况下调试业务逻辑（domain / dao / app 层）。日志输出到文件，避免与终端渲染冲突。

## 2. 核心原则

- **不改动业务逻辑**：只在关键位置插入 `tracing` 埋点
- **可配置**：通过 `RUST_LOG` 环境变量调整级别，默认 `INFO`
- **默认输出到文件**：当前目录 `app.log`，无 ANSI 颜色码
- **覆盖业务逻辑层**：不仅限于 TUI 事件，还包括 Dao 操作和状态转换

## 3. 依赖项

在 `Cargo.toml` 中新增：

```toml
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["fmt", "env-filter"] }
```

## 4. 日志初始化

在 `main.rs` 的 `main()` 函数开头初始化 tracing subscriber：

```rust
use tracing_subscriber::EnvFilter;

let log_file = std::fs::OpenOptions::new()
    .create(true)
    .append(true)
    .open("app.log")
    .expect("无法创建日志文件");

tracing_subscriber::fmt()
    .with_writer(move || log_file.try_clone().unwrap())
    .with_env_filter(
        EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("INFO"))
    )
    .with_ansi(false)
    .with_target(true)
    .init();
```

## 5. 埋点位置

| 位置 | 级别 | 内容 |
|------|------|------|
| `main.rs:main` 开头 | `info!` | `cc-switch-tui starting` |
| `main.rs:main` 结尾 | `info!` | `cc-switch-tui exiting` |
| `App::on_key` | `debug!` | `key_event = {:?}` |
| `App::handle_list` (n/e/d/q) | `debug!` | `state transition: List -> {:?}` |
| `App::submit_create` | `info!` | `create instance success: id={}` |
| `App::handle_edit` | `info!` | `update api_key: id={}` |
| `App::handle_delete_confirm` | `info!` | `delete instance: id={}` |
| `MemoryDaoImpl::create_instance` | `info!` | `dao create_instance: id={}` |
| `MemoryDaoImpl::update_instance` | `info!` | `dao update_instance: id={}` |
| `MemoryDaoImpl::delete_instance` | `info!` | `dao delete_instance: id={}` |
| `ui::draw` | `trace!` | `render frame, state={:?}` |

## 6. 使用方式

- 默认：`cargo run` → 只输出 `INFO` 及以上到 `app.log`
- 详细调试：`RUST_LOG=debug cargo run` → 输出 `DEBUG` 及以上
- 追踪渲染：`RUST_LOG=trace cargo run` → 输出所有级别（包括每帧渲染）

## 7. 后续扩展

- 日志文件路径可通过命令行参数或环境变量自定义
- 可考虑日志轮转，避免 `app.log` 无限增长
