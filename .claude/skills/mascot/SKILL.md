---
name: mascot
description: How the orc mascot is built and swapped - states, file layout, animation rules, reduced motion and how to drop in new art. Use when changing anything under app/src/lib/mascot or adding orc art.
paths: "app/src/lib/mascot/**"
---

Art direction and AI prompts: docs/mascot.md.

## States
| State | When | Look |
| --- | --- | --- |
| `sleeping` | no block running, Setup | closed, curved eyes |
| `guarding` | a block is running | open eyes, angled brows, an occasional blink |

## Files
- `Orc.svelte` takes `state` and `size` and draws the mark as inline SVG on a 24 px grid with `stroke="currentColor"`, so the parent's text colour themes it. It is the only mascot component.
- Final art (docs/mascot.md) replaces the paths in `Orc.svelte`; keep the 24 px viewBox and a single stroke width.

## Rules
- Decorative: the svg has `aria-hidden="true"`; state changes are announced by text elsewhere.
- Under `prefers-reduced-motion: reduce`, no motion at all.
- The blink is the only animation; nothing flashes more than three times a second.
- It stays a small mark (16-32 px in the UI). No large illustrations, scenes or colour fills.
