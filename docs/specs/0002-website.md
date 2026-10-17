# 0002: Download website

Status: approved (maintainer's goal, 2026-10-17)

## Problem
OrcBlock has a release on GitHub but no page to send people to. A newcomer needs to see what it does, what it looks like and where to download it, without reading the README.

## Behaviour
1. One page that looks like the app: the same paper and ink, hazard red, potion purple, macro type, hairline rules and grain, in light (the default) and dark, with a two-state version of the app's theme switch. The site does not follow the system theme.
2. Hero: what OrcBlock does in plain words, the guarding orc (poke him for a quip), a ticking demo countdown and a download block that puts the visitor's system first. macOS downloads the newest published release; Windows and Linux say "coming soon" with their logos.
3. The real app in a macOS window frame, in the visitor's theme.
4. How it works in four steps; "escape attempts" that the orc denies; the five sleep potions as the app's own vials, with a working demo where the orc drinks; the orc's three poses; a short FAQ; downloads per system; footer links to GitHub, README, releases, roadmap, privacy, security and licence.
5. A privacy page in plain words. A 404 page.

## Acceptance criteria
- [x] The macOS download links to `releases/latest/download/OrcBlock.dmg`
- [x] Windows and Linux buttons are disabled and say coming soon, with OS logos
- [x] The visitor's platform is detected and shown first (unit tested)
- [x] The orc, vials, tokens and icon come from `app/`, not copies
- [x] Light by default and dark on request, matching the app
- [x] Screenshots regenerate with one command and are documented
- [x] Responsive down to 320 px; reduced motion stops the animations
- [x] Deployed to Cloudflare Pages only when the site or what it reuses changes

## Design
`website/`: Vite + Svelte 5, the same allowlisted packages as the app, no new dependencies. Aliases `$app` → `app/src/lib` and `$icons` → `app/src-tauri/icons`. Two entry pages (`index.html`, `privacy/index.html`). `scripts/screenshots.mjs` drives a local Chromium-based browser over the DevTools protocol with a stand-in for the Tauri bridge. `.github/workflows/website.yml`: a build job without secrets, a deploy job with the Cloudflare token that runs no repository code.

## Bypass and safety
None: the site cannot reach the daemon. The deploy token is scoped to Cloudflare Pages and only exposed to the deploy job on `main`.

## Slices
1. The site, its pipeline and docs (one commit, at the maintainer's request).

## Test plan
Unit tests for platform detection; svelte-check and prettier in CI; manual check in light, dark and at phone width.

## Out of scope
Analytics, a blog, localisation, a changelog page, Windows and Linux downloads.

## Open questions
None.
