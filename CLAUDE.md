# 项目说明

这个使用类似 cc-switch 的 tui 项目， 来切换 多个 claude code 的模型提供商.

<IMPORTANT>
`docs/codebase/ARCHITECTURE.md` 是理解本项目架构的核心文档。它定义了代码库的分层结构、数据流、关键抽象、入口点、错误处理策略以及新代码的放置规范。任何涉及新增模块、重构或理解现有代码的决策，都必须先参考该文档。保持 ARCHITECTURE.md 与代码同步是维护项目可维护性的前提。
</IMPORTANT>

# currentDate
Today's date is 2026/06/01.

---

## Claude Code 自定义模型与上下文配置（关键决策参考）

### 自定义模型配置方法

Claude Code 支持通过环境变量使用第三方 Anthropic-compatible API（如 MiniMax、OpenRouter、Kimi 等）：

```json
{
  "env": {
    "ANTHROPIC_BASE_URL": "https://api.minimax.io/anthropic",
    "ANTHROPIC_AUTH_TOKEN": "your-token",
    "ANTHROPIC_MODEL": "MiniMax-M2.7",
    "ANTHROPIC_CUSTOM_MODEL_OPTION": "kimi-k2.6"
  }
}
```

**关键环境变量：**

| 变量 | 作用 |
|------|------|
| `ANTHROPIC_BASE_URL` | 第三方 API 端点 |
| `ANTHROPIC_AUTH_TOKEN` | 第三方平台的 API Key |
| `ANTHROPIC_MODEL` | 当前使用的模型名称 |
| `ANTHROPIC_CUSTOM_MODEL_OPTION` | 自定义模型选项 |
| `ANTHROPIC_DEFAULT_OPUS_MODEL` / `ANTHROPIC_DEFAULT_SONNET_MODEL` | 默认模型映射 |

### 上下文（Context）大小设置 — 关键发现

**`CLAUDE_CODE_MAX_CONTEXT_TOKENS` 必须配合 `DISABLE_COMPACT=1` 使用。**

这是通过反编译 Claude Code 二进制确认的内部逻辑（v2.1.112+）：

```javascript
function getMaxContextTokens(model, features) {
  // Path 1: ONLY fires if BOTH env vars are set
  if (truthy(process.env.DISABLE_COMPACT) && process.env.CLAUDE_CODE_MAX_CONTEXT_TOKENS) {
    return parseInt(process.env.CLAUDE_CODE_MAX_CONTEXT_TOKENS, 10);
  }
  // ...其他路径
  return 200_000;  // 回退值
}
```

**配置示例：**

```json
{
  "env": {
    "DISABLE_COMPACT": "1",
    "CLAUDE_CODE_MAX_CONTEXT_TOKENS": "1000000",
    "CLAUDE_CODE_AUTO_COMPACT_WINDOW": "1000000"
  }
}
```

**`DISABLE_COMPACT=1` 的含义：**
- 禁用 Claude Code 的自动压缩（AutoCompact）功能
- 默认情况下，上下文到 ~167K（200K 窗口）时会自动有损压缩历史记录
- 禁用后，需手动运行 `/compact` 或监控使用情况，避免超限

### 已知限制与 Bug

1. **第三方提供商的 Context Window 检测失效**（Issue #46416）
   - `getModelCapability()` 被 `isFirstPartyAnthropicBaseUrl()` 限制
   - 第三方 URL 返回 `undefined`，回退到硬编码的 200K
   - 结果：即使模型支持 1M，AutoCompact 仍可能在 ~187K 触发

2. **`[1m]` 后缀被重置**（Issue #50083，截至 2026-05-26）
   - VS Code 扩展每次启动会 strip `[1m]` 后缀
   - 依赖 `model` 字段设置 `claude-opus-4-6[1m]` 不可靠
   - 环境变量方式不受影响

3. **`CLAUDE_CODE_AUTO_COMPACT_WINDOW` 被硬编码上限限制**（Issue #57964）
   - 实现为 `Math.min(modelWindow, configured)`
   - 如果 CC 认为模型窗口是 200K，环境变量被强制限制为 200K

4. **第三方模型加载插件报 400**（Issue #63471）

5. **`--resume` 时自定义模型 ID 解析问题**（Issue #63376 / #62353）

### 对项目设计的启示

- **Provider-scoped 配置**：上下文相关变量（`CLAUDE_CODE_MAX_CONTEXT_TOKENS`、`CLAUDE_CODE_AUTO_COMPACT_WINDOW`）必须按 provider 隔离，不应放入 common config。不同 provider 支持的窗口大小不同（200K vs 1M）。
- **不要依赖 `model` 字段的 `[1m]` 后缀**：会被 Claude Code 启动时静默重置。
- **环境变量是可靠的覆盖方式**：`DISABLE_COMPACT=1` + `CLAUDE_CODE_MAX_CONTEXT_TOKENS` 组合在 v2.1.145+ 仍有效（2026-05-26 验证）。


