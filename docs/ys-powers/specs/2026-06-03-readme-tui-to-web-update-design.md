# Spec: README TUI → Web 更新

## Objective

将项目 README.md 从描述旧 TUI 界面更新为描述当前 Web 界面，确保新用户拿到的是准确的文档。

## Tech Stack

- Markdown（无代码改动）

## Commands

无需构建/测试命令。验证方式：
```bash
# 本地预览
npx markdown-preview README.md
```

## Project Structure

```
README.md          ← 唯一改动目标
docs/              ← 链接保持不变
```

## Code Style

- 保持现有 README 的语气和格式风格
- 使用中文，技术术语保留英文
- 表格语法使用标准 GFM

## Testing Strategy

- 人工阅读检查链接有效性
- 确认无 broken markdown 语法

## Boundaries

- **Always:** 保持项目名 `cc-switch-tui` 不变
- **Always:** 保留现有文档链接（docs/ 目录结构不变）
- **Never:** 添加 TUI 历史信息（方案 A：完全替换）
- **Never:** 添加截图

## Success Criteria

- [ ] README 中无任何 "TUI" 字样
- [ ] 功能列表描述 Web 界面操作
- [ ] 快速开始描述：运行二进制 → 自动打开浏览器
- [ ] 删除键盘快捷键表格
- [ ] 核心概念章节更新为 Web 语境
