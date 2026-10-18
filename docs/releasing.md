# Releasing

A release is a version bump merged into main. Every push to main runs `.github/workflows/release.yml`; when the version in `Cargo.toml` has no `vX.Y.Z` tag yet, it builds the app for macOS and Windows on GitHub, tags the merged commit and publishes the release. Merges that leave the version alone are not released.

## Steps

1. On a branch, set the new version in all four places: `[workspace.package]` in `Cargo.toml`, `app/src-tauri/tauri.conf.json`, `app/package.json` and `app/package-lock.json` (run `npm --prefix app version X.Y.Z --no-git-tag-version` for the two npm files). `make check` fails while they disagree.
2. Merge the PR.
3. Wait for the release workflow. A `version` job finds the untagged version. A `build` job with read-only access checks the versions agree, runs `make check` and builds the universal app with `make bundle-universal`, without caches; a `build-windows` job checks the versions, runs clippy and the tests on Windows and builds the installer with `scripts/bundle-windows.sh`, also without caches. A separate `publish` job, which runs no repository code, attests the files, creates the tag and publishes the release with notes from the commits.
4. Edit the notes on GitHub if needed. A version with a suffix, such as `1.0.0-beta.1`, is published as a pre-release. If the workflow failed, fix the cause on a branch and merge, or rerun it from the Actions tab (`workflow_dispatch`, main only). The website's download button follows a published release at once.
5. If the app looks different, refresh the website's screenshots with `make site-screenshots` on a branch (website/README.md).

## What a release contains

| File | Use |
| --- | --- |
| `OrcBlock.dmg` | The same name in every release, so `https://github.com/orcrobert/orcblock/releases/latest/download/OrcBlock.dmg` always downloads the newest published version. Link the website here. |
| `OrcBlock_X.Y.Z_macos_universal.dmg` | The same file under a versioned name, for links to one version. |
| `OrcBlock-setup.exe` | The Windows installer under a stable name, for `https://github.com/orcrobert/orcblock/releases/latest/download/OrcBlock-setup.exe`. |
| `OrcBlock_X.Y.Z_windows_x64-setup.exe` | The same installer under a versioned name. |
| `SHA256SUMS` | Checksums of all four files. |

Each dmg and installer also has a build provenance attestation: `gh attestation verify OrcBlock.dmg -R orcrobert/orcblock` with `--signer-workflow orcrobert/orcblock/.github/workflows/release.yml --source-ref refs/heads/main` proves it was built by this repository's release workflow from main; the attestation names the commit, which is the one the tag points at. The tag itself is a lightweight tag the workflow creates.

The `latest/download` link only follows published releases that are not marked as pre-releases.

## Repository settings

The workflow depends on settings that live on GitHub, not in the repository:

- Branch protection on `main`: require pull requests, the `check` status and branches up to date before merging, with no bypass. CI runs on pull requests only, so this is what makes the tested merge result the one that lands.
- An environment named `release` with deployment branches limited to `main`. Add a required reviewer there to approve each release by hand before it is published.
- No tag ruleset that restricts creating `v*` tags: the workflow creates them with its own token. Blocking updates and deletions of `v*` tags is fine.

## Signing

The app is signed ad hoc, not with an Apple Developer ID, and it is not notarized. On first launch macOS asks people to allow it once under System Settings → Privacy & Security → Open Anyway; the release notes explain this. Without the ad hoc signature, a downloaded app would be reported as damaged.

The Windows installer is not signed either, so SmartScreen warns on first run until people choose **More info → Run anyway**; the release notes explain this too.

The helper's SHA-256 is pinned into the app at build time, so the bundle is signed as a whole after building and the helper inside is never re-signed; `make bundle-universal` checks that it is byte-identical to the pinned file.

## Other platforms

macOS and Windows (x64) are released. A Linux download comes once its enforcer exists (roadmap, "Later"); add a job to the release workflow then, with a stable asset name, its own checksum file and a merge step in `publish` like `build-windows`.

## If a release goes wrong

A release that failed before publishing left no tag (delete any half-made draft release on GitHub): fix the problem on a branch and merge, and the next push to main releases the same version. A published release is never moved or replaced; bump the version and merge a fix instead. Do not push `v*` tags by hand, since the workflow skips a version that is already tagged.
