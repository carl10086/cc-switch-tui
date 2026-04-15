# cc-switch-tui 发布自动化设计文档

## 1. 目标

实现 tag 推送后自动构建并发布 macOS 二进制文件到 GitHub Releases。

## 2. 平台支持

- macOS x86_64
- macOS arm64（Apple Silicon）

## 3. 文件结构

```
.github/
└── workflows/
    └── release.yml       # GitHub Actions 工作流
Makefile                   # 本地构建和发布命令
```

## 4. GitHub Actions 工作流

触发条件：推送 `v*` tag

构建矩阵：
- x86_64-apple-darwin
- aarch64-apple-darwin

上传构件：直接上传编译好的二进制文件到 Release

## 5. Makefile

```makefile
BINARY_NAME = cc-switch-tui
VERSION = 0.1.0

build:
	cargo build --release

tag:
	git tag v$(VERSION)
	@echo "Tag v$(VERSION) created. Run 'make release' to push and publish."

release: tag
	git push origin v$(VERSION)

install:
	cargo install --path .

clean:
	cargo clean
	rm -rf dist/
```

## 6. 发布流程

```bash
# 1. 修改版本号（手动编辑 Cargo.toml）
# 2. 创建 tag
make tag VERSION=0.1.0

# 3. 推送 tag，触发 GitHub Actions
make release
```

## 7. 发布产物

- `cc-switch-tui-macos-x86_64`
- `cc-switch-tui-macos-arm64`
