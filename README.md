# cc-switch-tui

管理 Claude Code 模型提供商的 Rust TUI 工具。

## 快速开始

### 前置要求

- Rust 1.75+
- macOS / Linux（zsh）

### 安装

从 [Releases](https://github.com/carl10086/cc-switch-tui/releases) 下载二进制文件。

**macOS 安全提示**：首次运行可能会提示"无法验证开发者"。在终端执行以下命令即可运行：

```bash
xattr -d com.apple.quarantine cc-switch-tui-macos-arm64
./cc-switch-tui-macos-arm64
```

### 初始配置

首次运行时会自动在 `~/.zshrc` 末尾添加一行：

```zsh
source ~/.cc-switch-tui/aliases.zsh
```

然后重新加载 shell 或执行 `source ~/.zshrc`。

启动后通过 TUI 创建第一个 Instance。

## 核心概念

**Provider** — 模型提供商，目前支持 MiniMax、Kimi。

**Instance** — 用户创建的 Provider 配置实例，包含 API Key 和自定义别名。

**Alias** — 根据 Instance 生成的 shell alias，激活后切换环境变量。

## 功能列表

- TUI 界面管理 Provider Instance
- 支持 MiniMax、Kimi 等多 Provider
- 一键生成并激活 shell alias
- SQLite 本地持久化存储
- 自动配置 zshrc

## 键盘快捷键

| 按键   | 功能               |
|--------|--------------------|
| j/↑   | 上一项             |
| k/↓   | 下一项             |
| Enter | 选择/确认          |
| n     | 创建新 Instance    |
| e     | 编辑选中 Instance  |
| d     | 删除选中 Instance  |
| q     | 退出               |
