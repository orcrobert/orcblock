CARGO_DENY_VERSION := 0.20.2
TARGET ?= $(shell rustc -vV | sed -n 's/^host: //p')
MAC_TARGETS := aarch64-apple-darwin x86_64-apple-darwin
UNIVERSAL := universal-apple-darwin
DATA_DIR := $(CURDIR)/data
SOCKET := $(DATA_DIR)/orcblockd.sock
DAEMON_ARGS ?=

.PHONY: check fmt fmt-check lint test deny deps versions ui-check hooks tools daemon-dev dev sidecar bundle sidecar-universal bundle-universal install-helper site-dev site-check site-build site-screenshots

check: fmt-check lint test deny deps versions ui-check

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

lint:
	cargo clippy --workspace --all-targets --locked -- -D warnings

test:
	cargo test --workspace --locked

deny:
	cargo deny --workspace check

deps:
	sh scripts/check-deps.sh

versions:
	sh scripts/release-version.sh

ui-check:
	npm --prefix app run check
	npm --prefix app test
	npm --prefix app run format:check

hooks:
	git config core.hooksPath scripts/hooks

tools:
	cargo install cargo-deny --version $(CARGO_DENY_VERSION) --locked

daemon-dev:
	cargo run -p orcblockd -- serve --dev --data-dir $(DATA_DIR) --socket $(SOCKET) $(DAEMON_ARGS)

dev:
	ORCBLOCK_SOCKET=$(SOCKET) npm --prefix app run tauri dev

sidecar:
	cargo build -p orcblockd --release --locked
	mkdir -p app/src-tauri/binaries
	cp target/release/orcblockd app/src-tauri/binaries/orcblockd-$(TARGET)

bundle: sidecar
	npm --prefix app run tauri build -- --config src-tauri/tauri.bundle.json

sidecar-universal:
	rustup target add $(MAC_TARGETS)
	mkdir -p app/src-tauri/binaries
	for target in $(MAC_TARGETS); do \
		cargo build -p orcblockd --release --locked --target $$target || exit 1; \
		cp target/$$target/release/orcblockd app/src-tauri/binaries/orcblockd-$$target; \
	done
	lipo -create -output app/src-tauri/binaries/orcblockd-$(UNIVERSAL) $(foreach target,$(MAC_TARGETS),target/$(target)/release/orcblockd)
	codesign --force --sign - --options runtime app/src-tauri/binaries/orcblockd-$(UNIVERSAL)

RELEASE_DIR := target/$(UNIVERSAL)/release/bundle
RELEASE_APP := $(RELEASE_DIR)/macos/OrcBlock.app

# The app is signed ad hoc as a whole after bundling: letting the bundler
# sign would re-sign the helper and break the hash pinned into the app.
bundle-universal: sidecar-universal
	ORCBLOCK_HELPER_TARGET=$(UNIVERSAL) npm --prefix app run tauri build -- --target $(UNIVERSAL) --bundles app --config src-tauri/tauri.bundle.json -- --locked
	codesign --force --sign - $(RELEASE_APP)
	codesign --verify --strict $(RELEASE_APP)
	cmp app/src-tauri/binaries/orcblockd-$(UNIVERSAL) $(RELEASE_APP)/Contents/MacOS/orcblockd
	grep -aq "$$(shasum -a 256 $(RELEASE_APP)/Contents/MacOS/orcblockd | cut -c1-64)" $(RELEASE_APP)/Contents/MacOS/orcblock-app || \
		{ echo "the app does not pin the helper it bundles" >&2; exit 1; }
	COPYFILE_DISABLE=1 tar --no-mac-metadata -czf $(RELEASE_DIR)/macos/OrcBlock.app.tar.gz -C $(RELEASE_DIR)/macos OrcBlock.app
	rm -rf $(RELEASE_DIR)/dmg && mkdir -p $(RELEASE_DIR)/dmg/stage
	cp -R $(RELEASE_APP) $(RELEASE_DIR)/dmg/stage/
	ln -s /Applications $(RELEASE_DIR)/dmg/stage/Applications
	hdiutil create -volname OrcBlock -srcfolder $(RELEASE_DIR)/dmg/stage -fs HFS+ -format UDZO -ov $(RELEASE_DIR)/dmg/OrcBlock.dmg
	rm -rf $(RELEASE_DIR)/dmg/stage

install-helper:
	cargo build -p orcblockd --release --locked
	sudo target/release/orcblockd install

site-dev:
	npm --prefix website run dev

site-check:
	npm --prefix website run check
	npm --prefix website test
	npm --prefix website run format:check

site-build:
	npm --prefix website run build

site-screenshots:
	npm --prefix website run screenshots
