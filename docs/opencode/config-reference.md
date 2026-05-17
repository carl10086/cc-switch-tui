# OpenCode 配置参考文档

> 来源：OpenCode 官方文档 (https://opencode.ai/docs/zh-cn)
> 整理日期：2026-05-16

---

## 1. 配置加载优先级

OpenCode 按以下顺序加载配置，**后加载的配置会覆盖先加载的配置**：

| 优先级 | 配置来源 | 说明 |
|--------|----------|------|
| 1 (最低) | Remote | 组织通过 `.well-known/opencode` 端点提供的默认配置 |
| 2 | Global | 用户全局配置 `~/.config/opencode/opencode.json` |
| 3 | **Custom** | **通过 `OPENCODE_CONFIG` 环境变量指定的自定义配置文件** |
| 4 (最高标准) | Project | 项目根目录的 `opencode.json`（从当前目录向上搜索到最近的 Git 目录） |
| - | `.opencode` 目录 | 代理、命令、插件等 |
| - | Inline | 运行时覆盖 |

---

## 2. 配置文件路径

### 2.1 全局配置

```
~/.config/opencode/opencode.json
```

### 2.2 项目配置

```
<PROJECT_ROOT>/opencode.json
```

OpenCode 从当前目录开始向上搜索，直到找到最近的 Git 仓库根目录。

### 2.3 自定义配置路径（关键）

通过 **`OPENCODE_CONFIG`** 环境变量指定自定义配置文件：

```bash
export OPENCODE_CONFIG=/path/to/custom-opencode.json
opencode
```

该配置加载优先级介于**全局配置和项目配置之间**。

---

## 3. 配置结构

### 3.1 基础结构

```json
{
  "$schema": "https://opencode.ai/config.json",
  "model": "anthropic/claude-sonnet-4-20250514",
  "provider": {
    "myprovider": {
      "npm": "@ai-sdk/openai-compatible",
      "name": "My AI Provider",
      "options": {
        "baseURL": "https://api.myprovider.com/v1",
        "apiKey": "{env:ANTHROPIC_API_KEY}",
        "headers": {
          "Authorization": "Bearer custom-token"
        }
      },
      "models": {
        "my-model-name": {
          "name": "My Model Display Name",
          "limit": {
            "context": 200000,
            "output": 65536
          }
        }
      }
    }
  }
}
```

### 3.2 Provider 配置字段

| 字段 | 类型 | 说明 |
|------|------|------|
| `npm` | string | AI SDK 包名，如 `@ai-sdk/openai-compatible` |
| `name` | string | Provider 显示名称 |
| `options.baseURL` | string | API 端点 URL |
| `options.apiKey` | string | API Key，支持 `{env:VAR}` 语法 |
| `options.headers` | object | 自定义请求头 |
| `models` | object | 可用模型列表 |
| `models.<id>.name` | string | 模型显示名称 |
| `models.<id>.limit.context` | number | 上下文窗口限制 |
| `models.<id>.limit.output` | number | 输出 token 限制 |

---

## 4. 环境变量语法

在 `opencode.json` 中使用 `{env:VARIABLE_NAME}` 引用环境变量：

```json
{
  "model": "{env:OPENCODE_MODEL}",
  "provider": {
    "anthropic": {
      "options": {
        "apiKey": "{env:ANTHROPIC_API_KEY}"
      }
    }
  }
}
```

如果环境变量未设置，默认值为空字符串。

---

## 5. Credentials 管理

### 5.1 存储位置

```
~/.local/share/opencode/auth.json
```

格式：

```json
{
  "provider-id": {
    "type": "api",
    "key": "sk-xxx"
  }
}
```

### 5.2 CLI 命令

```bash
# 登录/添加凭证
opencode auth login

# 列出已认证 provider
opencode auth list
opencode auth ls

# 登出/清除凭证
opencode auth logout
```

### 5.3 加载来源

OpenCode 启动时按以下顺序加载认证信息：

1. `auth.json` 文件中的凭证
2. 环境变量中定义的 key
3. 项目 `.env` 文件中的 key

---

## 6. 模型选择

### 6.1 配置文件中指定

```json
{
  "model": "provider-id/model-id"
}
```

### 6.2 命令行参数

```bash
opencode -m provider-id/model-id
opencode --model provider-id/model-id
```

### 6.3 选择优先级

1. `--model` / `-m` 命令行参数（最高）
2. `opencode.json` 中的 `model` 字段
3. 最后使用的模型
4. 内部默认模型（最低）

---

## 7. 与 cc-switch-tui 集成建议

### 7.1 方案 A：OPENCODE_CONFIG + 独立配置文件（推荐）

利用 `OPENCODE_CONFIG` 环境变量，为每个 alias 维护独立的配置文件：

```zsh
function oc-kimi {
  export OPENCODE_CONFIG="$HOME/.cc-switch-tui/opencode/kimi.json"
  command opencode "$@"
}

function oc-mini {
  export OPENCODE_CONFIG="$HOME/.cc-switch-tui/opencode/minimax.json"
  command opencode "$@"
}
```

**优点**：
- 完全隔离，每个 alias 独立配置
- 无需文件复制操作，仅通过环境变量切换
- 与 Claude Code 的 alias 机制一致

### 7.2 方案 B：环境变量 + 单一配置文件

在单一 `opencode.json` 中使用 `{env:...}` 语法：

```json
{
  "provider": {
    "kimi": {
      "options": {
        "apiKey": "{env:KIMI_API_KEY}"
      }
    }
  }
}
```

```zsh
function oc-kimi {
  export OPENCODE_MODEL="kimi/kimi-for-coding"
  export KIMI_API_KEY="sk-xxx"
  command opencode "$@"
}
```

**优点**：配置集中管理；**缺点**：所有 provider 配置同时存在，无法完全隔离。

### 7.3 方案 C：-m 参数 + 预配置

在全局 `opencode.json` 中预配置所有 provider，通过 `-m` 切换：

```zsh
function oc-kimi {
  command opencode -m kimi/kimi-for-coding "$@"
}
```

**缺点**：API key 仍需预存，无法通过 alias 隔离。

---

## 8. 关键发现汇总

| 特性 | 支持情况 | 说明 |
|------|----------|------|
| `OPENCODE_CONFIG` | ✅ | 可指定自定义配置文件路径 |
| `{env:VAR}` | ✅ | 配置文件中可引用环境变量 |
| `-m provider/model` | ✅ | 命令行指定模型 |
| `XDG_CONFIG_HOME` | ❓ | 未明确文档化 |
| 多 profile 切换 | ✅ | 通过 `OPENCODE_CONFIG` 实现 |
| 环境变量覆盖 API key | ✅ | 通过 `{env:...}` 或 `.env` 文件 |

---

## 9. 参考链接

- 官方文档：https://opencode.ai/docs/zh-cn
- 配置文档：https://opencode.ai/docs/zh-cn/config
- Provider 文档：https://opencode.ai/docs/zh-cn/providers
- CLI 文档：https://opencode.ai/docs/zh-cn/cli
