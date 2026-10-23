# Releasing

A release is a version bump merged into main. Every push to main runs `.github/workflows/release.yml`; when the version in `Cargo.toml` has no `vX.Y.Z` tag yet, it builds the app on GitHub, tags the merged commit and publishes the release. Merges that leave the version alone are not released.

## Steps

1. On a branch, set the new version in all four places: `[workspace.package]` in `Cargo.toml`, `app/src-tauri/tauri.conf.json`, `app/package.json` and `app/package-lock.json` (run `npm --prefix app version X.Y.Z --no-git-tag-version` for the two npm files). `make check` fails while they disagree.
2. Merge the PR.
3. Wait for the release workflow. A `version` job finds the untagged version. A `build` job with read-only access checks the versions agree, runs `make check` and builds the universal app with `make bundle-universal`, without caches. A separate `publish` job, which runs no repository code, attests the files, creates the tag and publishes the release with notes from the commits.
4. Edit the notes on GitHub if needed. A version with a suffix, such as `1.0.0-beta.1`, is published as a pre-release. If the workflow failed, fix the cause on a branch and merge, or rerun it from the Actions tab (`workflow_dispatch`, main only). The website's download button follows a published release at once.
5. If the app looks different, refresh the website's screenshots with `make site-screenshots` on a branch (website/README.md).

## What a release contains

| File | Use |
| --- | --- |
| `OrcBlock.dmg` | The same name in every release, so `https://github.com/orcrobert/orcblock/releases/latest/download/OrcBlock.dmg` always downloads the newest published version. Link the website here. |
| `OrcBlock_X.Y.Z_macos_universal.dmg` | The same file under a versioned name, for links to one version. |
| `OrcBlock.app.tar.gz` | The signed app for in-app updates. |
| `OrcBlock.app.tar.gz.sig` | Its minisign signature, base64 encoded as Tauri expects, with the version in the trusted comment. |
| `latest.json` | What the app reads to find an update: version, date, and the tarball's URL and signature for `darwin-aarch64` and `darwin-x86_64` (both the universal tarball). The app asks `releases/latest/download/latest.json`, so it never offers a pre-release. |
| `SHA256SUMS` | Checksums of both dmgs and the tarball. |

Each dmg and the tarball also have a build provenance attestation: `gh attestation verify OrcBlock.dmg -R orcrobert/orcblock` with `--signer-workflow orcrobert/orcblock/.github/workflows/release.yml --source-ref refs/heads/main` proves it was built by this repository's release workflow from main; the attestation names the commit, which is the one the tag points at. The tag itself is a lightweight tag the workflow creates.

The `latest/download` link only follows published releases that are not marked as pre-releases.

## Repository settings

The workflow depends on settings that live on GitHub, not in the repository:

- Branch protection on `main`: require pull requests, the `check` status and branches up to date before merging, with no bypass. CI runs on pull requests only, so this is what makes the tested merge result the one that lands.
- An environment named `release` with deployment branches limited to `main`. Add a required reviewer there to approve each release by hand before it is published.
- No tag ruleset that restricts creating `v*` tags: the workflow creates them with its own token. Blocking updates and deletions of `v*` tags is fine.

## Signing

The app is signed ad hoc, not with an Apple Developer ID, and it is not notarized. On first launch macOS asks people to allow it once under System Settings → Privacy & Security → Open Anyway; the release notes explain this. Without the ad hoc signature, a downloaded app would be reported as damaged.

The helper's SHA-256 is pinned into the app at build time, so the bundle is signed as a whole after building and the helper inside is never re-signed; `make bundle-universal` checks that it is byte-identical to the pinned file.

## Update signing

With in-app updates, a merged version bump is signed and offered to every installed app. The `release` environment therefore requires a reviewer (orcrobert, since 2026-10-23): the publish job, and with it the signing, waits until a person approves it on the run's page. Keep it that way; see Repository settings.

The app installs an update only if the tarball is signed by the key whose public half is `plugins.updater.pubkey` in `app/src-tauri/tauri.conf.json`, and only if the signed version matches `latest.json` (`requireSignedVersion`). `make bundle-universal` makes the tarball after `codesign`, so updated apps keep the ad hoc signature and the pinned helper.

- The private key and its password are secrets of the `release` environment: `TAURI_SIGNING_PRIVATE_KEY` (the contents of the `.key` file that `tauri signer generate` writes) and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. Only the `publish` job reads them, in a step that sees nothing else from the build but the tarball. It runs no repository code and signs with Ubuntu's `minisign`; a later step without the secrets checks the signature against the public key in the released commit's `tauri.conf.json`, read through the GitHub API, so a key that does not match the app fails the release instead of every update. The `build` job never sees them, because it runs npm install scripts and build scripts.
- Keep an offline copy of the key and password (a password manager). GitHub never shows a secret again.
- If the key is lost or leaks: generate a new pair with `npm --prefix app run tauri signer generate -- -w <path>`, put the public key in `tauri.conf.json`, replace both secrets and release. Installed apps still trust the old key, so they cannot update to that release; people reinstall from the dmg once. A leaked key means rotating at once: anyone holding it can publish an update that every installed app accepts with one click, which runs as each user who updates, and as root once they type their password into a prompt that update controls.

## Other platforms

Only the macOS daemon can enforce a block today. Windows and Linux downloads come once their enforcers exist (roadmap, "Later"); add a job per platform to the release workflow then, with a stable asset name such as `OrcBlock-windows-x64.msi`.

## If a release goes wrong

A release that failed before publishing left no tag (delete any half-made draft release on GitHub): fix the problem on a branch and merge, and the next push to main releases the same version. A published release is never moved or replaced; bump the version and merge a fix instead. Do not push `v*` tags by hand, since the workflow skips a version that is already tagged.
