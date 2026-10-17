#!/bin/sh
# Fails when a direct dependency is not on the allowlist in docs/decisions/0001-dependency-policy.md,
# or when a crate declares a dependency without going through [workspace.dependencies].
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)
policy="$root/docs/decisions/0001-dependency-policy.md"

allowed=$(sed -n '/^<!-- allowlist:start -->$/,/^<!-- allowlist:end -->$/p' "$policy" | sed -n 's/^- `\([^`]*\)`.*/\1/p')
[ -n "$allowed" ] || { echo "check-deps: no allowlist found in $policy" >&2; exit 1; }

status=0
fail() { echo "check-deps: $1" >&2; status=1; }

workspace_deps=$(sed -n '/^\[workspace\.dependencies\]/,/^\[/p' "$root/Cargo.toml" | grep -v 'path *=' | sed -n 's/^\([A-Za-z0-9_-]*\) *=.*/\1/p')
for dep in $workspace_deps; do
	printf '%s\n' "$allowed" | grep -qx "$dep" || fail "crate $dep is not allowlisted; add an ADR (docs/decisions/0001-dependency-policy.md)"
done

for manifest in $(cd "$root" && git ls-files '*Cargo.toml' | grep -v '^Cargo.toml$'); do
	awk -v file="$manifest" '
		/^\[/ { section = $0; next }
		section ~ /^\[(dev-|build-)?dependencies\]$/ && /^[A-Za-z0-9_-]+ *=/ {
			name = $1
			if ($0 !~ /workspace *= *true/ && $0 !~ /path *=/) { printf "check-deps: %s: %s must use { workspace = true }\n", file, name; bad = 1 }
		}
		END { exit bad }
	' "$root/$manifest" >&2 || status=1
done

for package in app website; do
	[ -f "$root/$package/package.json" ] || continue
	npm_deps=$(node -e '
		const p = require(process.argv[1]);
		console.log(Object.keys({ ...p.dependencies, ...p.devDependencies }).join("\n"));
	' "$root/$package/package.json")
	for dep in $npm_deps; do
		printf '%s\n' "$allowed" | grep -qx "npm:$dep" || fail "npm package $dep in $package/ is not allowlisted; add an ADR (docs/decisions/0001-dependency-policy.md)"
	done
done

exit $status
