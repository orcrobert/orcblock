#!/bin/sh
# Fails unless the workspace, the Tauri config and the npm package carry the same version,
# and, given a tag such as v0.2.0, unless the tag names that version.
set -eu

root=$(cd "$(dirname "$0")/.." && pwd)

json_version() { sed -n 's/^  "version": "\(.*\)",$/\1/p' "$1" | head -n 1; }

cargo=$(sed -n '/^\[workspace\.package\]/,/^\[/s/^version = "\(.*\)"$/\1/p' "$root/Cargo.toml")
[ -n "$cargo" ] || { echo "release-version: no [workspace.package] version in Cargo.toml" >&2; exit 1; }

status=0
for file in app/src-tauri/tauri.conf.json app/package.json app/package-lock.json; do
	found=$(json_version "$root/$file")
	if [ "$found" != "$cargo" ]; then
		echo "release-version: $file has version '$found', Cargo.toml has '$cargo'" >&2
		status=1
	fi
done

if [ $# -gt 0 ] && [ "$1" != "v$cargo" ]; then
	echo "release-version: tag '$1' does not match version $cargo; tag v$cargo or bump the version" >&2
	status=1
fi

exit $status
