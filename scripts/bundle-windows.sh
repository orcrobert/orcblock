#!/bin/sh
# Builds the Windows installer: the daemon as a sidecar, then the per-machine
# NSIS installer, and checks that the app pins the helper it bundles. Run on
# Windows from Git Bash with the MSVC toolchain, as CI does.
set -eu

target=x86_64-pc-windows-msvc
sidecar=app/src-tauri/binaries/orcblockd-$target.exe
release=target/$target/release

cargo build -p orcblockd --release --locked --target "$target"
mkdir -p app/src-tauri/binaries
cp "$release/orcblockd.exe" "$sidecar"

ORCBLOCK_HELPER_TARGET=$target npm --prefix app run tauri build -- \
	--target "$target" --bundles nsis --config src-tauri/tauri.bundle.json -- --locked

hash=$(sha256sum "$sidecar" | cut -c1-64)
grep -aq "$hash" "$release/orcblock-app.exe" || {
	echo "bundle-windows: the app does not pin the helper it bundles" >&2
	exit 1
}
ls "$release"/bundle/nsis/*-setup.exe
