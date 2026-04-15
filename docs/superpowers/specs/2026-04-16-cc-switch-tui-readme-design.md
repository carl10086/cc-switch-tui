# cc-switch-tui README 设计文档

## 1. 项目简介

**一句话简介：** Rust TUI 工具，管理并切换 Claude Code 的多个模型提供商。

## 2. 结构

```
1. 项目标题 + 一句话简介
2. 快速开始
   - 前置要求
   - 安装步骤
   - 初始配置
3. 核心概念
   - Provider
   - Instance
   - Alias
4. 功能列表
5. 键盘快捷键
```

## 3. 各章节内容

### 3.1 项目标题 + 一句话简介

```
cc-switch-tui
管理 Claude Code 模型提供商的 Rust TUI 工具
```

### 3.2 快速开始

**前置要求：**
- Rust 1.75+
- macOS / Linux（zsh）

**安装步骤：**
```bash
cargo install cc-switch-tui
```

**初始配置：**
- 首次运行会自动配置 `~/.zshrc`，添加 `source ~/.cc-switch-tui/aliases.zsh`
- 启动后通过 TUI 创建第一个 Provider Instance

### 3.3 核心概念

**Provider** — 模型提供商，如 MiniMax、Kimi

**Instance** — 用户创建的 Provider 配置实例，包含 API Key

**Alias** — 根据 Instance 生成的 shell alias，激活后切换环境变量

### 3.4 功能列表

- TUI 界面管理 Provider Instance
- 支持 MiniMax、Kimi 等多 Provider
- 一键生成并激活 shell alias
- SQLite 本地持久化存储
- 自动配置 zshrc

### 3.5 键盘快捷键

```
j/↓    下一项
k/↑    上一项
Enter  选择/确认
c      创建新 Instance
e      编辑选中 Instance
d      删除选中 Instance
q      退出
```

## 4. 视觉风格

- 标题用 `===` 和 `---` 分层
- 代码块统一用 ` ```bash ` 或 ` ``` `
- 简洁清晰，无截图

## 5. 文件位置

输出文件：`README.md`（项目根目录）
