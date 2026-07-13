# cc-switch-tui

管理 Claude Code 模型提供商的 Rust Web 工具。

## 功能列表

- **Web 管理** — 浏览器界面管理 Provider Instance
- **多 Provider** — 支持 MiniMax、Kimi 等多 Provider
- **多 Alias** — 同一模型支持多个配置实例
- **Shell Function 隔离** — 使用函数代替 alias，环境变量隔离更好
- **claude 参数透传** — alias 支持传递任意参数给 claude
- **KV Cache 优化** — 优化本地模型的 prompt cache 命中率
- **OpenCode 支持** — 生成 OpenCode 配置文件，支持 OpenCode 模型选择
- **兼容 CMUX** — 防止切换时丢失认证信息
- **CC_SWITCH_ALIAS** — 注入环境变量用于 statusline 显示
- **SQLite 持久化** — 本地持久化存储配置
- **自动配置 zshrc** — 自动注入 source 命令

## 快速开始

### 前置要求

- Rust 1.75+
- macOS / Linux（zsh）

### 安装

从 [Releases](https://github.com/carl10086/cc-switch-tui/releases) 下载二进制文件。

```bash
# macOS 安全提示：首次运行可能会提示"无法验证开发者"
xattr -d com.apple.quarantine cc-switch-tui-macos-arm64
./cc-switch-tui-macos-arm64
```

### 初始配置

首次运行时会自动在 `~/.zshrc` 末尾添加一行：

```bash
source ~/.cc-switch-tui/aliases.zsh
```

然后重新加载 shell 或执行 `source ~/.zshrc`。

启动后会自动打开浏览器，通过 Web 界面创建第一个 Instance。

## PM2 常驻运行

如果你希望 `cc-switch-tui` 作为后台常驻进程运行（例如配合 PM2 管理），可以使用项目自带的 `ecosystem.config.js`。

### 前置步骤

先编译 release 二进制：

```bash
cargo build --release
# 或全量构建（含前端）
make build
```

### 启动 / 停止 / 重启

```bash
# 启动
pm2 start ecosystem.config.js

# 查看状态
pm2 status cc-switch-tui

# 查看日志
pm2 logs cc-switch-tui

# 重启
pm2 restart ecosystem.config.js

# 停止
pm2 stop ecosystem.config.js

# 移除进程
pm2 delete ecosystem.config.js
```

PM2 配置中已默认设置 `CC_SWITCH_NO_OPEN=1`，因此常驻启动时**不会**自动打开浏览器。

### 开机自启

```bash
pm2 save
pm2 startup
```

执行 `pm2 startup` 后按终端提示完成系统级启动脚本注册即可。

## 核心概念

**Provider** — 模型提供商，目前支持 MiniMax、Kimi。

**Instance** — 用户创建的 Provider 配置实例，包含 API Key、自定义别名和可选的 KV Cache 优化开关。

**Alias** — 根据 Instance 生成的 shell function，激活后切换环境变量。

## 进阶功能

### KV Cache 优化

针对本地 llama.cpp 模型优化 cache 命中率。开启后 `cl-xxx` alias 会追加以下参数：

```bash
--exclude-dynamic-system-prompt-sections --settings '{"includeGitInstructions":false}'
```

详见 [KV Cache 配置指南](docs/claude-code-local-model-kv-cache.md)。

### OpenCode 支持

自动生成 OpenCode 配置文件，支持 OpenCode 模型选择。

详见 [OpenCode 配置参考](docs/opencode/config-reference.md)。

## 相关文档

- [KV Cache 配置指南](docs/claude-code-local-model-kv-cache.md)
- [OpenCode 配置参考](docs/opencode/config-reference.md)
