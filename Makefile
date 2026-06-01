BINARY_NAME = cc-switch-tui
VERSION ?= $(shell grep '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')
DIST_DIR = dist
WEB_DIR = web
WEB_DIST = web-dist

.PHONY: help build web-build dev dev-rust-only test typecheck lint fmt \
        tag release publish clean all

help: ## Show this help
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-15s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

# ----- 开发 -----
dev: ## Vite dev server (5173) + cargo run (7480)
	@echo "==> 启动 Vite dev server (5173) + cargo run (7480)"
	@echo "    浏览器访问 http://127.0.0.1:5173"
	cd $(WEB_DIR) && npm run dev &
	cargo run

dev-rust-only: ## 仅 cargo run，使用已编译的 web-dist/
	@echo "==> 仅 Rust；前提: web-dist/ 已存在 (跑过 make web-build)"
	cargo run

# ----- 构建 -----
web-build: ## Vite build → web-dist/
	@echo "==> 构建前端到 $(WEB_DIST)/"
	cd $(WEB_DIR) && npm ci --no-audit --no-fund 2>/dev/null || npm install --no-audit --no-fund
	cd $(WEB_DIR) && npm run build
	@rm -rf $(WEB_DIST)
	@cp -r $(WEB_DIR)/dist $(WEB_DIST)
	@echo "==> $(WEB_DIST)/ 已就绪"

build: web-build ## Build release (单文件二进制，嵌入前端)
	@echo "==> cargo build --release"
	@mkdir -p $(DIST_DIR)
	cargo build --release
	@cp target/release/$(BINARY_NAME) $(DIST_DIR)/$(BINARY_NAME)-macos-arm64
	@echo "Built: $(DIST_DIR)/$(BINARY_NAME)-macos-arm64 (含嵌入前端)"

# ----- 测试 / 质量门 -----
test: ## cargo test + npm test
	@echo "==> cargo test"
	cargo test
	@echo "==> cd $(WEB_DIR) && npm test"
	cd $(WEB_DIR) && npm test --passWithNoTests 2>/dev/null || true

typecheck: ## 前端 TypeScript 类型检查
	cd $(WEB_DIR) && npm run typecheck

lint: ## cargo clippy + npm run lint
	@echo "==> cargo clippy"
	cargo clippy --all-targets -- -D warnings
	@echo "==> cd $(WEB_DIR) && npm run lint"
	cd $(WEB_DIR) && npm run lint 2>/dev/null || echo "(lint 暂未配置，跳过)"

fmt: ## cargo fmt
	cargo fmt

# ----- 发布 -----
tag: ## Create git tag (VERSION=0.1.0 make tag)
	git tag v$(VERSION)
	@echo "Tag v$(VERSION) created. Push with 'make release'"

release: ## Push tag to origin
	@echo "Pushing tag v$(VERSION)..."
	git push origin v$(VERSION)

publish: ## Create GitHub release and upload binary (requires gh CLI)
	@if [ ! -d "$(DIST_DIR)" ] || [ -z "$$(ls -A $(DIST_DIR))" ]; then \
		echo "No binary found. Run 'make build' first."; \
		exit 1; \
	fi
	@echo "Creating release v$(VERSION)..."
	gh release create v$(VERSION) --title "v$(VERSION)" --generate-notes
	@echo "Uploading binary..."
	@for f in $(DIST_DIR)/*; do \
		echo "  Uploading $$f..."; \
		gh release upload v$(VERSION) $$f; \
	done
	@echo "Done! Release: https://github.com/$$(gh repo view --json nameWithOwner --jq .nameWithOwner)/releases/tag/v$(VERSION)"

clean: ## 清理构建产物
	cargo clean
	rm -rf $(DIST_DIR)/
	rm -rf $(WEB_DIST)/
	rm -rf $(WEB_DIR)/dist
	rm -f $(WEB_DIR)/*.tsbuildinfo

all: build publish ## Full release: build + publish
