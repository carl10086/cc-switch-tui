# Intent: aliases.zsh 给 cl-* function 体加 subshell 包裹 + 删除 unset

## TL;DR

修改 `cc-switch-tui` 工具生成的 `~/.cc-switch-tui/aliases.zsh` 中 `cl-*` function 的体，把：

```zsh
function cl-mini {
  unset ANTHROPIC_AUTH_TOKEN ANTHROPIC_BASE_URL ... DISABLE_COMPACT
  export ANTHROPIC_AUTH_TOKEN=sk-cp-...
  export ANTHROPIC_BASE_URL=https://api.minimaxi.com/anthropic
  ...
  [[ $CC_SWITCH_PROXY_URL == http://localhost:* ]] && export ANTHROPIC_BASE_URL="$CC_SWITCH_PROXY_URL"
  __cc_switch_print_env cl-mini
  command claude "$@"
}
```

改成：

```zsh
function cl-mini {
  (
    export ANTHROPIC_AUTH_TOKEN=sk-cp-...
    export ANTHROPIC_BASE_URL=https://api.minimaxi.com/anthropic
    ...
    [[ $CC_SWITCH_PROXY_URL == http://localhost:* ]] && export ANTHROPIC_BASE_URL="$CC_SWITCH_PROXY_URL"
    __cc_switch_print_env cl-mini
    command claude "$@"
  )
}
```

两处变化：
1. 删除函数体开头的 `unset ...` 行
2. 函数体外层包 `(...)` 形成 subshell

`oc-*` / `ys-proxy` / `__cc_switch_print_env` / sentinel `CC_SWITCH_PROXY_URL` 全部不动。

---

## Outcome

任意 `cl-*` alias 调用结束后，**父 shell 不残留** `ANTHROPIC_AUTH_TOKEN` / `ANTHROPIC_BASE_URL` / `API_TIMEOUT_MS` / `CC_SWITCH_ALIAS` 等变量；同时：

- `ys-proxy` wrapper 仍然生效（sentinel `CC_SWITCH_PROXY_URL` 在 subshell 内可被读到，条件 override 仍生效）
- 用户在 `~/.zshrc` 自己 export 的相关变量（如 `MINIMAX_API_KEY`）不被破坏
- `"$@"` 仍透传给 `claude`
- `__cc_switch_print_env` 诊断输出仍正常打印到 stderr

---

## 范围内 (In Scope)

- `src/shell.rs::format_function`（仅 cl-* 部分的输出）
- 渲染出来的 `cl-km1` / `cl-km2` / `cl-mini` 三条 function
- 函数体外层加 `(...)` 形成 subshell
- 函数体开头删除 `unset ...` 行
- 函数体内缩进 +2 空格以体现 subshell 嵌套

## 范围外 (Out of Scope)

- `oc-*` aliases — 不在本次范围（与 CC_SWITCH_PROXY_URL sentinel 无关，独立问题）
- `ys-proxy` wrapper — 不动（必须保留 function 形态，因为需要 `$1` 动态展开 URL）
- `CC_SWITCH_PROXY_URL` sentinel 模式 — 不动
- `__cc_switch_print_env` helper 定义 — 不动
- env 变量名 / 值 / 顺序 — 不变
- 不写功能测试 / 不更新 README
- 不回写用户当前的 `~/.cc-switch-tui/aliases.zsh`——工具下次运行自行覆盖

---

## 关键设计决策与原因

### 决策 1：cl-* 仍保持 `function` 形态（不改成 alias）

**原因**：

考虑过把 `function cl-X { export ... }` 改成 alias 形式 `alias cl-X='ANTHROPIC_AUTH_TOKEN=... claude "$@"'`，实测后发现不可行：

- ys-proxy wrapper 通过 `CC_SWITCH_PROXY_URL="..." $alias_name "$@"` 调用 cl-*；cl-* 必须能在函数体内读取这个 sentinel 并条件覆盖 `ANTHROPIC_BASE_URL`。alias 字符串是**纯文本替换**，写不下条件判断。
- ys-proxy 自身需要 `$alias_name` 动态展开 URL。**zsh alias 不能访问 `${1}`**——实测验证：
  ```zsh
  $ alias testalias='echo ${1}'
  $ testalias hello
  echo ${1} hello        ← ${1} 字面量，不是 "hello"
  ```

因此 cl-* 与 ys-proxy 都必须保持 function 形态。原 intent 文档里"alias 替代 function"的解法被否决。

代价：放弃了"用 alias 形态本身带来稳定性"的论点；但 subshell 包裹解决了"function 污染父 shell"这个核心痛点，代价可接受。

### 决策 2：用 subshell `(...)` 包裹 function 体

**原因**：

`(...)` 创建子 shell 进程，子 shell 内的所有 `export` / `unset` / 变量赋值**只**作用于子 shell 及其子进程；子 shell 退出后，所有状态变化随之销毁，**父 shell 完全不变**。这是 POSIX shell 的隔离原语，bash / zsh 都支持。

实测（用户已手动验证通过）：

```zsh
$ function cl-mini { (
    export ANTHROPIC_BASE_URL=https://api.minimaxi.com/anthropic
    command claude "$@"
  ) }
$ cl-mini --version
$ env | grep ANTHROPIC_BASE_URL
                                        # ← 空，父 shell 无残留
```

**额外验证（subshell 内能读父 shell 的 sentinel）**：

```zsh
$ CC_SWITCH_PROXY_URL=http://localhost:7480/ys-proxy/cl-mini
$ function cl-mini { (
    export ANTHROPIC_BASE_URL=https://api.minimaxi.com/anthropic
    [[ $CC_SWITCH_PROXY_URL == http://localhost:* ]] && \
      export ANTHROPIC_BASE_URL="$CC_SWITCH_PROXY_URL"
    command claude "$@"
  ) }
$ cl-mini --version
# claude 子进程看到的 ANTHROPIC_BASE_URL = http://localhost:7480/ys-proxy/cl-mini ✅
# 父 shell 的 CC_SWITCH_PROXY_URL 仍为原值 ✅
```

### 决策 3：删除 `unset ...` 行

**原因**：

旧的 `unset ANTHROPIC_AUTH_TOKEN ANTHROPIC_BASE_URL ... DISABLE_COMPACT` 是防御性清理，针对的是"前一次 function 形态调用污染父 shell 后，本次调用要擦干净"这种场景。

subshell 形态下：
- subshell 是新 fork 的进程，**只继承**父 shell 的 env
- 父 shell 现在已经不被污染了（决策 2 生效），所以 subshell 继承的也是干净的
- 不再需要 unset

**额外收益**：保留 unset 反而可能在 subshell 里**意外擦掉**用户在父 shell 设的值。例如 `~/.zshrc:172` 的 `export MINIMAX_API_KEY=sk-...` 被 inherit 进 subshell 后又被 `unset MINIMAX_API_KEY` 抹掉——这正是原 intent 决策 3 想避免的反模式。删除 unset 同时关闭了这条隐蔽 bug 通道。

### 决策 4：sentinel `CC_SWITCH_PROXY_URL` + `ys-proxy` 完整保留

**原因**：

- subshell fork 时 inherit 父 shell 全部 env，所以 subshell 内能读到父 shell 的 `CC_SWITCH_PROXY_URL`（决策 2 已实测）。
- `[[ $CC_SWITCH_PROXY_URL == http://localhost:* ]] && export ANTHROPIC_BASE_URL="$CC_SWITCH_PROXY_URL"` 这条条件 override 在 subshell 内仍正常执行。
- ys-proxy 自身形态（function + `$1` 动态展开 + prefix-env 注入 sentinel）完全不动；它本就不污染父 shell（prefix-env 形式是 POSIX 标准的临时 env 注入）。

**用户原话确认**："ys-proxy + sentinel 后面肯定不会退场，但是会有新的方案"——本次 intent 不动 sentinel 架构，新方案是独立的后续 intent。

### 决策 5：`__cc_switch_print_env` helper 完整保留

**原因**：

- helper 定义在 aliases.zsh 顶部（file scope），subshell 内调用它无作用域问题
- `echo "[cc-switch-tui] $alias_name: ..." >&2` 输出到 stderr，claude 进程结束后用户能看到——时序与之前一致
- 诊断价值保留

### 决策 6：函数体内缩进 +2 空格

**原因**：

体现 subshell 嵌套层级，让源码和生成的 `.zsh` 文件都易读。改动面小，纯样式。

### 决策 7：`command claude "$@"` 保留

**原因**：

subshell 里 `command` 仍能跳过同名 alias；不影响功能也无副作用。原 intent 决策 5 讨论过"去掉以求纯净"，本 intent 不强求，保留现状。

---

## 不做 (Non-Goals)

- 不诊断 / 不修复 `function` 形态下**其他**已知 bug——本 intent 只解决"env 污染父 shell"这一项
- 不动 `cc-switch-tui` 的 TUI 界面
- 不动 `oc-*` aliases
- 不动 ys-proxy / sentinel 架构
- 不替用户在 `.zshrc` 加 `source` 行
- 不新增 env 变量、不替换第三方 auth provider

---

## 后续会问但本 intent 不阻塞

| # | 问题 | 决定方式 |
|---|---|---|
| Q1 | `oc-*` aliases 是否也按 subshell 形式改造（甚至改 alias）？ | 下一个 intent（独立决策，因为 oc-* 不读 sentinel，方案可能更激进） |
| Q2 | ys-proxy 的"新方案"是什么？ | 用户原话已确认会有，但具体形态未定，独立 intent |
| Q3 | `__cc_switch_print_env` 是否要从 helper 改成 `cl-X --status` 子命令？ | 下一个 intent（与 oc-* 一起评估） |

---

## 验收方法 (Definition of Done)

任一条 `cl-*` alias：

```sh
# 准备干净环境
$ unset ANTHROPIC_AUTH_TOKEN ANTHROPIC_BASE_URL API_TIMEOUT_MS CC_SWITCH_PROXY_URL \
        CC_SWITCH_ALIAS CLAUDE_CODE_MAX_CONTEXT_TOKENS CLAUDE_CODE_AUTO_COMPACT_WINDOW \
        DISABLE_COMPACT CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC \
        CMUX_PRESERVE_CLAUDE_AUTH_SELECTION_ENV

# 验收 1：claude 能跑通
$ cl-mini --version
# 期望：能连到 https://api.minimaxi.com/anthropic

# 验收 2：跑完后父 shell 干净（关键）
$ env | grep -E '^(ANTHROPIC|API_TIMEOUT_MS|CC_SWITCH_ALIAS)'
# 期望：空

# 验收 3：ys-proxy 仍能 override BASE_URL
$ ys-proxy cl-mini --version
# 期望：claude 进程的 ANTHROPIC_BASE_URL 等于 http://localhost:7480/ys-proxy/cl-mini
# （需 cc-switch-tui 服务在 7480 端口运行；否则走 provider 默认 URL）

# 验收 4：用户 ~/.zshrc 设的值不破坏
$ env | grep MINIMAX_API_KEY
# 期望：仍然是 ~/.zshrc:172 的值
```

`cl-km1` / `cl-km2` / `cl-mini` 三条重复上述测试全部通过 = DoD。

---

## 上下文引用

- 工具源码：`/Users/yusizhen/soft/projects/cc-switch-tui/src/shell.rs::format_function`（line 190）
- 渲染入口：`src/shell.rs::render_aliases`（line 10）
- 用户已手动验证 ✅（生成文件 `/Users/yusizhen/.cc-switch-tui/aliases.zsh` 已按本方案改写并跑通）
- 实测脚本：`/tmp/subshell-verify.zsh`（TEST 1 / 2 / 3）
- 备份：原文件备份于 `/tmp/aliases.zsh.bak.<timestamp>`
- 后续阶段：`/spec` 写设计、`/build` 实现 + 测试

---

## 不写代码 / 不写文档更新

本 intent 仅描述下一步要让 claude code 改的东西（`src/shell.rs::format_function` 的输出模板），不替 claude code 做设计。