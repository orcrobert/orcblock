---
name: mascot
description: How the orc mascot is built and swapped - states, file layout, animation rules, reduced motion and how to drop in new art. Use when changing anything under app/src/lib/mascot or adding orc art.
paths: "app/src/lib/mascot/**"
---

Art direction and AI prompts: docs/mascot.md.

## States
| State | When | Feel |
| --- | --- | --- |
| `sleeping` | no block running | curled up, slow breathing, drifting "z" |
| `guarding` | a block is running | standing in the path with a club, gentle sway, occasional blink |
| `drinking` | right after a potion is spent | drinks, wobbles, falls asleep (plays once, then `sleeping`) |

## Files
- `Orc.svelte` takes `state` and `size` and renders the current art; it is the only component the rest of the UI uses.
- `art/` holds one asset per state. The built-in placeholders are inline SVGs with CSS keyframes (`sleeping.svg`, `guarding.svg`). Final art can be animated SVG, a sprite-sheet PNG (`<state>.png` + frame count in `frames.ts`) or Lottie JSON if an ADR adds the player.
- Colours in the SVGs use `currentColor` and CSS variables (`--orc-skin`, `--orc-ink`) so the orc follows the theme.

## Rules
- Decorative: wrapper has `aria-hidden="true"`; state changes are announced by text elsewhere.
- Under `prefers-reduced-motion: reduce`, show the first frame only.
- Loops are slow (3-6 s) and subtle; nothing flashes more than three times a second.
- Size budget: each state under 60 KB.
- New art replaces a file in `art/` with the same name; `Orc.svelte` should not need changes unless the format changes.
