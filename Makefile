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
