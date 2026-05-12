---
name: make-release
description: cc-switch-tui 发布 runbook：版本号决策 → 前置检查 → bump/commit/build/tag/publish 一步完成
---

## 安全红线

- **仅适用于 cc-switch-tui 项目**（root `Cargo.toml` 中 `[package].name = "cc-switch-tui"`）。不匹配立刻退出，不做任何 git/file/网络操作。
- 不擅自决定版本号 —— 必须给推荐 + 等用户选定
- 前置检查失败时不擅自 `git stash` / `git checkout` / `git pull`
- 任何步骤失败时**不要**建议 `git push --delete origin v{X}` 或 `gh release delete`
- 改 `Cargo.toml` 用 Edit 工具，**不要**用 `sed -i ''`（macOS/Linux 语法差异坑过）
- 不信任 `make publish` 输出的 "Done!" —— 必须 `gh release view` 二次验证

## Workflow

复制此清单并逐项勾选：

```
- [ ] 0. 作用域检查 → verify: [package].name == "cc-switch-tui"
- [ ] 1. 版本号决策 → verify: 用户从 A/B/C 选定 v{new}
- [ ] 2. 前置检查（7 项）→ verify: 全绿
- [ ] 3. 呈现完整计划 → verify: 用户回复"执行/跑/goon/ok"
- [ ] 4. bump Cargo.toml + cargo check → verify: Cargo.toml/lock 同步到 {new}
- [ ] 5. commit + push → verify: origin/main 含 chore commit
- [ ] 6. make build → verify: dist/cc-switch-tui-macos-arm64 存在
- [ ] 7. make tag + make release → verify: origin 有 v{new} tag
- [ ] 8. make publish → verify: gh release view v{new} 含 asset
```

### 0. 作用域检查

```bash
awk '/^\[package\]/{p=1;next} /^\[/{p=0} p && /^name/{print;exit}' Cargo.toml
# 期望输出: name = "cc-switch-tui"
```

不匹配 → 立刻输出并退出：

> 本 command 仅适用于 cc-switch-tui 项目。当前项目不匹配，已退出。

### 1. 版本号决策

1. 读当前版本：
   ```bash
   grep '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/'
   ```
2. 读自上次 tag 以来的 commits：
   ```bash
   git log --oneline v{current}..HEAD
   ```
3. 按 conventional commits 前缀推断推荐：

   | commits 内容 | 推荐 |
   |---|---|
   | 全是 `fix:` / `docs:` / `chore:` / `ci:` | **patch** |
   | 含 `feat:` / `refactor:`，无 `BREAKING CHANGE` | **minor** |
   | 含 `feat!:` 或 body 有 `BREAKING CHANGE` | **major** |

4. 给用户三选一，**等用户回复后再继续**：

   ```
   自 v{current} 起的 commits：
   - ...

   推荐 {patch|minor|major} bump：
   - A. v{recommended}（推荐）
   - B. v{next-level}（次一档）
   - C. 自定（请告诉我）
   ```

### 2. 前置检查清单

并行跑，**任一失败 → 停 + 报告 + 等用户处理，不擅自修复**。

| # | 检查 | 命令 | 通过条件 |
|---|---|---|---|
| 1 | 在 main 分支 | `git symbolic-ref --short HEAD` | 输出 `main` |
| 2 | 工作区 clean | `git status --porcelain` | 空输出 |
| 3 | 与 origin 同步 | `git fetch && git rev-list HEAD..origin/main --count` | 输出 `0` |
| 4 | gh 已登录 | `gh auth status` | exit 0 |
| 5 | package 名匹配 | `awk '/^\[package\]/{p=1;next} /^\[/{p=0} p && /^name/{print;exit}' Cargo.toml` | 输出 `name = "cc-switch-tui"` |
| 6 | Makefile target 齐全 | `grep -E '^(build\|tag\|release\|publish):' Makefile \| wc -l` | 输出 `4` |
| 7 | tag 未存在 | `git tag -l v{new}` | 空输出 |

### 3. 呈现完整计划

把以下 5 步骤的完整命令清单展示给用户，**等回复"执行/跑/goon/ok"后才进入 Step 4**。

### 4. bump Cargo.toml + cargo check

```bash
# 用 Edit 工具：Cargo.toml 中 version = "{current}" → version = "{new}"
cargo check
```

验证：`grep '^version' Cargo.toml` 输出 `version = "{new}"`，且 `Cargo.lock` 中 `cc-switch-tui` 块的 version 也是 {new}。

### 5. commit + push

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to {new}"
git push origin main
```

验证：`git log -1 --oneline` 显示 chore commit；push 输出含 `{old_sha}..{new_sha}  main -> main`。

### 6. make build

```bash
make build
```

验证：

```bash
ls -lh dist/cc-switch-tui-macos-arm64
```

存在且 size > 0。

### 7. make tag + make release

```bash
make tag        # git tag v{new}
make release    # git push origin v{new}
```

验证：push 输出含 `* [new tag]  v{new} -> v{new}`。

### 8. make publish + 二次验证

```bash
make publish
```

**忽略** `make publish` 末行的 `Done! Release: ...`（即使修过仍是 echo，不算可靠证据）。**必须**跑：

```bash
gh release view v{new} --json assets,url --jq '.url, (.assets[].name)'
```

期望两行输出：
1. release URL（含 `/releases/tag/v{new}`）
2. asset 名 `cc-switch-tui-macos-arm64`

## 异常情况处理

| 失败点 | 当前状态 | 应对 |
|---|---|---|
| Step 0 作用域不匹配 | 未做任何写操作 | 立刻退出，告诉用户这个 command 是 cc-switch-tui 专用 |
| Step 2 任一检查 fail | 未做任何写操作 | 报告失败项 + 让用户决定，**不要**自动 fix |
| Step 4-5 失败 | 仅本地改动 | 建议 `git checkout -- Cargo.toml Cargo.lock` 回滚 |
| Step 5 push 失败 | commit 本地存在 | `git fetch && git log HEAD..origin/main`；建议 `git pull --rebase origin main` 后重试 push（不要自动跑 pull） |
| Step 6 build 失败 | 无副作用 | 修编译错误后从 Step 6 重跑 |
| Step 7 tag 已推 + Step 8 失败 | tag 在 origin，无 release | 建议手动 `gh release create v{new} --generate-notes && gh release upload v{new} dist/*`。**不要**建议 delete tag |
| Step 8 release 已建 + upload 失败 | release 在但缺 binary | 单独 `gh release upload v{new} dist/*` 重试 |

## 执行后自检

- [ ] 作用域检查通过，没在错的项目里跑
- [ ] 版本号由用户确认，没擅自决定
- [ ] 前置检查 7 项全绿（不是绕过的）
- [ ] 用户已确认完整计划再进入执行
- [ ] Cargo.toml 用 Edit 工具改的，不是 sed
- [ ] `gh release view v{new}` 二次验证通过，不只看 `make publish` 的 Done 行
- [ ] 没有任何 destructive 操作（delete tag / delete release / force-push）

## 备注

- 当前 Makefile 只产 macOS arm64 一个二进制，没跨平台编译。如果未来加了 linux/win 产物，Step 6 验证那行要更新。
- `make publish` 用 `--generate-notes`，自动从 `v{prev}..v{new}` 的 PR/commit 拼 release body。手写 notes 让用户单独跑 `gh release edit`。
