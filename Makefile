CARGO_DENY_VERSION := 0.20.2
TARGET ?= $(shell rustc -vV | sed -n 's/^host: //p')
DATA_DIR := $(CURDIR)/data
SOCKET := $(DATA_DIR)/orcblockd.sock

.PHONY: check fmt fmt-check lint test deny deps ui-check hooks tools daemon-dev dev sidecar bundle install-helper

check: fmt-check lint test deny deps ui-check

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

ui-check:
	npm --prefix app run check
	npm --prefix app test
	npm --prefix app run format:check

hooks:
	git config core.hooksPath scripts/hooks

tools:
	cargo install cargo-deny --version $(CARGO_DENY_VERSION) --locked

daemon-dev:
	cargo run -p orcblockd -- serve --dev --data-dir $(DATA_DIR) --socket $(SOCKET)

dev:
	ORCBLOCK_SOCKET=$(SOCKET) npm --prefix app run tauri dev

sidecar:
	cargo build -p orcblockd --release --locked
	mkdir -p app/src-tauri/binaries
	cp target/release/orcblockd app/src-tauri/binaries/orcblockd-$(TARGET)

bundle: sidecar
	npm --prefix app run tauri build -- --config src-tauri/tauri.bundle.json

install-helper:
	cargo build -p orcblockd --release --locked
	sudo target/release/orcblockd install
