---
name: mascot
description: How the orc mascot is built and swapped - states, file layout, animation rules, reduced motion and how to drop in new art. Use when changing anything under app/src/lib/mascot or adding orc art.
paths: "app/src/lib/mascot/**"
---

Art direction and how the final sprites are made: docs/mascot.md.

## States
| State | When | Animation |
| --- | --- | --- |
| `sleeping` | no block running, Setup | 2 frames at 1 fps: breathing, drifting z, loops |
| `guarding` | a block is running | 8 frames at 3 fps: red eyes, one blink, loops |
| `drinking` | right after a potion | 6 frames at 4 fps: lifts the red potion, drinks, eyes close; plays once, holds, then `ondone` |

## Files
- `frames.ts`: the frames as 32 x 32 strings (`#` ink, `r` red, `p` potion purple, `.` transparent) with `fps` and `loop` per state. The current art is a placeholder; final art replaces this file only.
- `Orc.svelte`: takes `pose`, `scale` (screen px per art px) and `ondone`, plays the frames as crisp SVG rects in `--ink`, `--red` and `--potion`, so it follows the theme.
- `Vial.svelte`: one potion vial, full or empty, from `VIAL_FULL` / `VIAL_EMPTY`; sized by its container.
- `pixels.ts`: `pixelRuns` and the code-to-colour map shared by both.
- `OrcBox` (components) stages the potion: the shelf vial flies to `VIAL_IN_HAND`, `drinking` plays, then the orc sleeps and the empty vial climbs from `VIAL_AT_MOUTH` back to its slot. Keep the vial art in the frames identical to `VIAL_FULL`.

## Rules
- 1-bit look: ink, the accent red (eyes) and potion purple (the liquid) only, no anti-aliasing (`shape-rendering: crispEdges`), integer scale.
- Decorative: the svg has `aria-hidden="true"`; the caption and status strip carry the state in text.
- Under `prefers-reduced-motion: reduce`: first frame only; `drinking` still calls `ondone`.
- Nothing flashes more than three times a second.
