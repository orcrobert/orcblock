# The orc

OrcBlock's orc is a 1-bit pixel sprite, ORC-01: a stocky, original fantasy orc seen from the front, drawn in the theme's ink with red eyes while it guards and a purple sleep potion when it drinks. It lives in the orc box on the left of the window and is the app's only illustration.

The app ships with a placeholder sprite (`app/src/lib/mascot/frames.ts`). This page is the brief for the final art.

## What the app needs

All frames are 32 x 32 art pixels, shown at 5x. Colours are codes, not RGB, so the theme can recolour them: `#` ink, `r` accent red, `p` potion purple, `.` transparent.

| Animation | Frames | Speed | Notes |
| --- | --- | --- | --- |
| `sleeping` | 2 | 1 fps, loops | idle and on Setup: eyes closed, chest rises and falls, a small "z" drifts up |
| `guarding` | 8 | 3 fps, loops | a block is running: red eyes (`r`), arms crossed or hand on a weapon hilt, one blink on the last frame |
| `drinking` | 6 | 4 fps, plays once | after a potion: the vial is in the right hand on frame 1, rises to the mouth, tips, eyes grow heavy and close |
| vial | 1 (full and empty) | - | 4 x 6 art px; the shelf shows five of them and one flies into the orc's hand |

The drinking sequence is staged by `OrcBox.svelte`: a full vial falls from the shelf into the hand, the swap to the sprite happens on the first drinking frame, and after the last frame the orc sleeps while the empty vial climbs back to its slot. For that to line up, the final art must keep the vial identical in `VIAL_FULL` and in the drinking frames, and `VIAL_IN_HAND` and `VIAL_AT_MOUTH` in `frames.ts` must give the vial's top-left corner on the first and last drinking frames.

## Style rules

- 1-bit: ink on transparent, red only for the guarding eyes, purple only for potion liquid. No grey, no anti-aliasing, no outline in paper colour except to separate the vial from the body.
- Front view, centred, feet on the bottom row, same silhouette in every frame so the animations swap cleanly.
- Original design: heavy brow, small lower tusks, pointed ears, simple armour. No film creatures (Uruk-hai, Azog) and no Tolkien names or logos.
- Readable at 1x: if a detail vanishes at 32 px, remove it.

## Making it with AI

Claude can draw the placeholder, but a good sprite comes from a tool built for pixel art.

1. **PixelLab** (pixellab.ai) is the best fit: it generates true pixel art at a fixed size, keeps a character consistent across poses and animates it ("animate with text" / skeleton). Create a 32 x 32 character, 1 colour plus transparency, front view.
2. **Optional concept first** in ChatGPT or Midjourney if you want to explore the look, then feed the best image to PixelLab as a style reference.
3. **Clean up in Aseprite** (or Pixelorama, free): reduce to the palette above (black, one red, one purple), fix stray pixels, line up the frames and export each animation as a horizontal PNG strip, 32 px per frame.
4. **Send me the strips** (`sleeping.png`, `guarding.png`, `drinking.png`, `vial.png`). I convert them into `frames.ts` (the generator maps black to `#`, red to `r`, purple to `p`) and set the vial points.

Prompts for PixelLab:

> Original fantasy orc warrior, front view, stocky, heavy brow, two small lower tusks, pointed ears, simple leather armour, 1-bit pixel art, black on transparent, 32x32, no shading, no anti-aliasing.

- Guarding: "standing guard, arms crossed, glowing red eyes, subtle idle breathing, blinks once".
- Sleeping: "same orc asleep standing, eyes closed, head slightly lowered, slow breathing".
- Drinking: "same orc holding a small potion vial in the right hand, lifts it to the mouth, drinks, eyes close".

## App icon

The icon is the guarding orc's head from `frames.ts` as large crisp pixels, light ink with red eyes on a dark squircle (`app/src-tauri/icons/source/icon.svg`), no text. It is dark in both system appearances: an icon that follows the appearance needs an Icon Composer `.icon` file compiled by Xcode 26's `actool`, which the build does not require.

When the sprite changes, redraw the SVG from the new guarding frame and run `npm --prefix app run tauri icon <svg> -o <tmp dir>`, then copy over only the files already in `app/src-tauri/icons`.
