# The orc

OrcBlock's orc is a mark, not a cartoon: a calm, geometric orc head drawn as a single-weight line icon. It sits next to the block state ("Idle", "Guarding") and on the app icon. It is the app's only illustration and its only playful note, so it stays small, quiet and precise. Think of an SF Symbol or a Material Symbol that happens to be an orc.

The app ships with a hand-drawn placeholder (`app/src/lib/mascot/Orc.svelte`). This page is the brief for the final mark, with or without AI.

## What the app needs

| Asset | Where | Notes |
| --- | --- | --- |
| `guarding` | next to "Guarding" while a block runs, 20-32 px | eyes open, brows angled down; a blink every few seconds is the only motion |
| `sleeping` | next to "Idle" and on the Setup screen | the same head with closed, curved eyes |
| `icon` | app icon, 1024 × 1024 | the same head in white or near-white on a deep green rounded square (macOS icon grid), no text |
| `menubar` (later) | the menu bar item | a template image: black on transparent, 18 × 18 pt |

All of them are the same head: same outline, ears, tusks and proportions.

## Style rules

- Line icon on a 24 × 24 grid, 1.5-2 px stroke, round caps and joins, no fills except the pupils. Follows `currentColor`, so it takes the theme's colours.
- Geometric and calm: a softened shield-shaped head, two pointed ears, two small lower tusks, a heavy brow line. No teeth rows, scars, helmets or weapons.
- Readable at 16 px. If a detail disappears at that size, remove it.
- Original design. No film creatures (Uruk-hai, Azog) and no Tolkien names or logos.

## Making it with AI

1. **Generate candidates** in a model that outputs vectors. Recraft is best here: choose the "Icon" or "Line art" style and export SVG. Midjourney or ChatGPT work too, but give you PNGs that you then vectorise (Recraft's vectoriser, Vectorizer.ai or Illustrator's Image Trace).
2. **Clean it up** in Figma or Illustrator: snap it to a 24 px grid, set one stroke width, outline nothing, and delete stray points. The final SVG should be a handful of paths.
3. **Make the second state** by duplicating the head and swapping only the eyes.
4. **Send me the SVGs** (`guarding.svg`, `sleeping.svg`, `icon.png`). I put the paths into `Orc.svelte` and run `npm --prefix app run tauri icon icon.png` for the app icons.

Prompt for the mark:
> Minimal line icon of an original fantasy orc head, front view, geometric and calm, softened shield-shaped head, two pointed ears, two small lower tusks, heavy brow, simple dot eyes. Single-weight 2 px stroke, round caps, no fill, no shading, 24 px icon grid, in the style of SF Symbols and Material Symbols, black on white, no text.

Sleeping variant:
> The same icon as the reference, identical outline, with the eyes closed as two small downward curves and no brows.

App icon:
> macOS app icon, rounded square, deep forest green background with a subtle top-to-bottom tonal shift, centred white line-art orc head identical to the reference icon, generous padding, flat, no text, no glossy effects.

## Sizes

- Mark SVGs: under 2 KB each, paths only, `stroke="currentColor"`.
- Icon: a 1024 × 1024 PNG.
