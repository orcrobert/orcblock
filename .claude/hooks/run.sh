#!/bin/sh
# Runs one agenthooks hook, rebuilding the binary first when its sources changed.
# A failed build exits 2 so the guards fail closed instead of silently allowing everything.
set -eu

root=${CLAUDE_PROJECT_DIR:-$(cd "$(dirname "$0")/../.." && pwd)}
bin="$root/.bin/agenthooks"
src="$root/tools/agenthooks"

if [ ! -x "$bin" ] || [ -n "$(find "$src" \( -name '*.rs' -o -name Cargo.toml \) -newer "$bin" 2>/dev/null | head -n 1)" ]; then
	mkdir -p "$root/.bin"
	if ! (cd "$root" && cargo build --quiet --release -p agenthooks && cp target/release/agenthooks "$bin") >&2; then
		echo "agenthooks: build failed; fix tools/agenthooks before continuing" >&2
		exit 2
	fi
fi

exec "$bin" "$@"
