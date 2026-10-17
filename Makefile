CARGO_DENY_VERSION := 0.20.2
TARGET ?= $(shell rustc -vV | sed -n 's/^host: //p')
DATA_DIR := $(CURDIR)/data
SOCKET := $(DATA_DIR)/orcblockd.sock

.PHONY: check fmt fmt-check lint test deny deps hooks tools daemon-dev dev sidecar bundle install-helper

check: fmt-check lint test deny deps

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

hooks:
	git config core.hooksPath scripts/hooks

tools:
	cargo install cargo-deny --version $(CARGO_DENY_VERSION) --locked
