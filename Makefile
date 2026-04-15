BINARY_NAME = cc-switch-tui
VERSION ?= 0.1.0
DIST_DIR = dist

.PHONY: build build-all tag release publish clean help

help: ## Show this help
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-12s %s\n", $$1, $$2}' $(MAKEFILE_LIST)

build: ## Build for current platform
	@mkdir -p $(DIST_DIR)
	cargo build --release
	@cp target/release/$(BINARY_NAME) $(DIST_DIR)/$(BINARY_NAME)-macos-$$(uname -m)

build-all: ## Build for x86_64 and arm64 (requires cross)
	@mkdir -p $(DIST_DIR)
	cross build --release --target x86_64-apple-darwin
	cross build --release --target aarch64-apple-darwin
	@cp target/x86_64-apple-darwin/release/$(BINARY_NAME) $(DIST_DIR)/$(BINARY_NAME)-macos-x86_64
	@cp target/aarch64-apple-darwin/release/$(BINARY_NAME) $(DIST_DIR)/$(BINARY_NAME)-macos-arm64
	@echo "Built binaries in $(DIST_DIR)/:"
	@ls -la $(DIST_DIR)/

tag: ## Create git tag (VERSION=0.1.0 make tag)
	git tag v$(VERSION)
	@echo "Tag v$(VERSION) created. Push with 'make release'"

release: ## Push tag to trigger GitHub Actions (if enabled)
	@echo "Pushing tag v$(VERSION)..."
	git push origin v$(VERSION)

publish: ## Create GitHub release and upload binaries (requires gh CLI)
	@if [ ! -d "$(DIST_DIR)" ] || [ -z "$$(ls -A $(DIST_DIR))" ]; then \
		echo "No binaries found. Run 'make build-all' first."; \
		exit 1; \
	fi
	@echo "Creating release v$(VERSION)..."
	gh release create v$(VERSION) --title "v$(VERSION)" --generate-notes
	@echo "Uploading binaries..."
	@for f in $(DIST_DIR)/*; do \
		echo "  Uploading $$f..."; \
		gh release upload v$(VERSION) $$f; \
	done
	@echo "Done! Release: https://github.com/$$(gh repo --jq .owner.login)/$$(gh repo --jq .name)/releases/tag/v$(VERSION)"

clean: ## Clean build artifacts
	cargo clean
	rm -rf $(DIST_DIR)/

all: build-all publish ## Full release flow: build + publish
