# Releasing

A release is a version bump merged into main. Every push to main runs `.github/workflows/release.yml`; when the version in `Cargo.toml` has no `vX.Y.Z` tag yet, it builds the app on GitHub, tags the merged commit and publishes the release. Merges that leave the version alone are not released.

## Steps

1. On a branch, set the new version in all four places: `[workspace.package]` in `Cargo.toml`, `app/src-tauri/tauri.conf.json`, `app/package.json` and `app/package-lock.json` (run `npm --prefix app version X.Y.Z --no-git-tag-version` for the two npm files). `make check` fails while they disagree.
2. Merge the PR.
3. Wait for the release workflow. A `version` job finds the untagged version. A `build` job with read-only access checks the versions agree, runs `make check` and builds the universal app with `make bundle-universal`, without caches. A separate `publish` job, which runs no repository code, attests the files, creates the tag and publishes the release with notes from the commits.
4. Edit the notes on GitHub if needed. If the workflow failed, fix the cause on a branch and merge, or rerun it from the Actions tab (`workflow_dispatch`, main only).

## What a release contains

| File | Use |
| --- | --- |
| `OrcBlock.dmg` | The same name in every release, so `https://github.com/orcrobert/orcblock/releases/latest/download/OrcBlock.dmg` always downloads the newest published version. Link the website here. |
| `OrcBlock_X.Y.Z_macos_universal.dmg` | The same file under a versioned name, for links to one version. |
| `SHA256SUMS` | Checksums of both dmgs. |

Each dmg also has a build provenance attestation: `gh attestation verify OrcBlock.dmg -R orcrobert/orcblock` proves it was built by this repository's release workflow from the tagged commit.

The `latest/download` link only follows published releases that are not marked as pre-releases.

## Signing

The app is signed ad hoc, not with an Apple Developer ID, and it is not notarized. On first launch macOS asks people to allow it once under System Settings → Privacy & Security → Open Anyway; the release notes explain this. Without the ad hoc signature, a downloaded app would be reported as damaged.

The helper's SHA-256 is pinned into the app at build time, so the bundle is signed as a whole after building and the helper inside is never re-signed; `make bundle-universal` checks that it is byte-identical to the pinned file.

## Other platforms

Only the macOS daemon can enforce a block today. Windows and Linux downloads come once their enforcers exist (roadmap, "Later"); add a job per platform to the release workflow then, with a stable asset name such as `OrcBlock-windows-x64.msi`.

## If a release goes wrong

A release that failed before publishing left no tag: fix the problem on a branch and merge, and the next push to main releases the same version. A published release is never moved or replaced; bump the version and merge a fix instead. Do not push `v*` tags by hand, since the workflow skips a version that is already tagged.
