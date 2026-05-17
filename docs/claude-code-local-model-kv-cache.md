# Claude Code + 本地模型 + llama.cpp KV Cache 配置指南

## 背景

本地后端（llama.cpp、llama-server、LM Studio）在提示词前缀稳定时性能最佳。Claude Code 会在每次请求中注入动态变化的 metadata 或 git context，这会导致 KV cache miss，迫使后端重新 prefill 大段 system prompt。

**目标**：让提示词前缀更稳定，提升 KV cache 命中率，减少重复 prefill。

## 启动 llama-server

```bash
/path/to/llama-server \
  --model /path/to/your-model.gguf \
  --jinja \
  --reasoning auto \
  --threads 12 \
  --n-gpu-layers 99 \
  --flash-attn on \
  --mlock \
  --cache-type-k q8_0 \
  --cache-type-v q8_0 \
  --cache-ram 24576 \
  --ctx-checkpoints 128 \
  --checkpoint-every-n-tokens 1024 \
  --slot-prompt-similarity 0.01 \
  --host 127.0.0.1 \
  --port 8080 \
  --parallel 1 \
  --cont-batching \
  --metrics \
  --slots
```

> **提示**：如果使用 TurboQuant build，可将 value cache 改为 `--cache-type-v turbo4`

### llama-server 关键参数说明

| 参数 | 说明 |
|------|------|
| `--flash-attn on` | 启用 Flash Attention，提升推理速度 |
| `--cache-type-k q8_0` | K cache 量化格式 |
| `--cache-type-v q8_0` | V cache 量化格式 |
| `--cache-ram 24576` | RAM 中缓存大小（MB） |
| `--ctx-checkpoints 128` | 上下文检查点数量 |
| `--checkpoint-every-n-tokens 1024` | 每 N tokens 保存检查点 |
| `--slot-prompt-similarity 0.01` | 插槽相似度阈值，低值利于 cache 复用 |
| `--parallel 1` | 限制并发请求数 |
| `--slots` | 启用插槽机制 |

## 启动 Claude Code

```bash
ANTHROPIC_BASE_URL=http://127.0.0.1:8080 \
ANTHROPIC_API_KEY=no-key \
ANTHROPIC_MODEL=your-model.gguf \
CLAUDE_CODE_ATTRIBUTION_HEADER=0 \
CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1 \
DISABLE_TELEMETRY=1 \
DISABLE_ERROR_REPORTING=1 \
claude --bare \
  --model your-model.gguf \
  --dangerously-skip-permissions \
  --exclude-dynamic-system-prompt-sections \
  --settings '{"includeGitInstructions":false}' \
  --allowedTools "WebFetch,Read,Edit,Write,Bash(curl:*)"
```

### 核心参数（实现 KV Cache 复用的关键）

这三个参数共同作用，使 system prompt 保持静态化，从而实现 KV cache 高命中率：

| 参数 | 作用 |
|------|------|
| `--exclude-dynamic-system-prompt-sections` | 将 cwd、env、memory、git status 从 system prompt 移到用户消息，避免动态信息污染 |
| `--settings '{"includeGitInstructions":false}'` | 禁用内置 git 指令和 git status 快照 |
| `--dangerously-skip-permissions` | 绕过权限检查（本地沙箱环境使用） |

**原理**：
1. `--exclude-dynamic-system-prompt-sections` 将每机器相关的动态信息移出 system prompt
2. `--settings '{"includeGitInstructions":false}'` 移除内置 git 指令，进一步减少动态内容
3. 两者结合确保 system prompt 在多次请求间保持稳定
4. llama-server 可以命中 KV cache，避免重复 prefill

### 环境变量说明

| 变量 | 说明 |
|------|------|
| `ANTHROPIC_BASE_URL` | 本地 llama-server 地址 |
| `ANTHROPIC_API_KEY` | 设为 `no-key`（本地无需认证） |
| `ANTHROPIC_MODEL` | 模型文件名 |
| `CLAUDE_CODE_ATTRIBUTION_HEADER` | 禁用归属头 |
| `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` | 禁用非必要流量 |
| `DISABLE_TELEMETRY` | 禁用遥测 |
| `DISABLE_ERROR_REPORTING` | 禁用错误报告 |

### 其他 Claude Code 参数说明

| 参数 | 说明 |
|------|------|
| `--bare` | 最小化启动模式 |
| `--allowedTools` | 允许使用的工具列表 |

## 验证结果

在 M3 Max MacBook Pro + Qwen3.6 27B 模型上测试多轮请求：

```bash
# 首次请求
prompt eval time = 271184.95 ms / 34947 tokens

# 后续请求
selected slot by LCP similarity, sim_best = 0.989
restored context checkpoint
prompt eval time = 5136.44 ms / 441 tokens
```

**效果**：

- 首次请求：34,947 tokens
- 后续请求：441 tokens（降低 98.7%）
- llama-server 日志显示 `restored context checkpoint`，证明 cache 命中

### 测试场景

| 场景 | 结果 |
|------|------|
| 多轮对话 | KV cache 正常复用 |
| Tool calling（截图等） | 正常工作 |
| 生成单文件 HTML 游戏 | 正常 |

## 参考

- 原文：https://datamoat.org/articles/claude-code-local-model-kv-cache/
- 问题来源：Reddit LocalLLaMA PSA
