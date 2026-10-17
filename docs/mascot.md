# The orc

OrcBlock's mascot is a small, friendly orc in the spirit of Middle-earth's orcs, but an original character drawn for this app: think a chunky, round-shouldered guard who is more grumpy-cute than scary. Like Claude's little mascot, he should read clearly at 64 px, be built from a few simple shapes, and carry the app's personality on his own.

The app ships with placeholder art (simple animated SVGs) so everything works today. This page is the brief for making the real art, with or without AI.

## States the app needs

| File (in `app/src/lib/mascot/art/`) | Shown when | Pose and motion | Loop |
| --- | --- | --- | --- |
| `sleeping` | no block is running | curled up or slumped against a rock, club on the ground, eyes closed; slow breathing, a "z" drifting up | 4-6 s, seamless |
| `guarding` | a block is running | standing square in a narrow path, club planted, arms crossed or holding a small round shield; slight weight shift, a blink every few seconds | 3-5 s, seamless |
| `drinking` (later) | right after a sleep potion is used | uncorks a small purple potion, drinks, wobbles, sits down and dozes | plays once, about 2 s, ends on the first frame of `sleeping` |
| `icon` | the app icon and menu bar | head and shoulders, front-facing, on a rounded square | still |

Keep the same character in every state: same proportions, colours, tusks, ears, outfit.

## Style rules

- Flat vector look: 3-5 colours, soft shading at most, no textures or gradients that break at small sizes.
- Palette that fits the app: moss-green skin (around `#6B8F4E`), dark outline/ink (`#1F2A1A`), leather brown, one accent for the potion (purple, around `#7E57C2`).
- Transparent background, square canvas, the character centred with padding; facing three-quarters to the left in every state.
- Original design. Do not imitate film designs (Peter Jackson's Uruk-hai, Azog, etc.) or include Tolkien names or logos; "an orc" is generic, a copy of a specific film creature is not.

## The easiest path (recommended)

1. **Design the character once.** Use an image model that can output vector art, such as Recraft (it exports SVG directly). ChatGPT image generation, Midjourney or Ideogram also work, but give you PNGs to vectorise afterwards. Generate a character sheet (prompt below), pick the one you like, and regenerate until front, side and three-quarter views agree.
2. **Make the poses.** Generate `sleeping` and `guarding` from the chosen design by attaching it as a reference image ("same character as the reference image"). Get SVGs (Recraft) or vectorise PNGs (Recraft's vectoriser, Vectorizer.ai, or Illustrator's Image Trace).
3. **Animate.** Pick one:
   - **Rive** (rive.app, free tier). Import the SVG, rig a few bones (body, head, eyelids, the "z"), make one looping animation per state, and connect them in a state machine. Export a `.riv` file. Best result for the least effort, and it can blend from guarding to sleeping. Needs one small runtime package in the app (an ADR, I will handle it).
   - **Lottie** (LottieFiles Creator in the browser, or After Effects + Bodymovin). Export `.json` per state. Also needs a small player package.
   - **Sprite sheet** (no runtime). Turn a still into a short clip with an image-to-video model (Kling, Runway, Veo or Sora: "subtle idle breathing loop, camera locked, character stays in place"), remove the background (Runway, or unscreen), export 24-48 frames, and pack them into one PNG row with a free tool such as TexturePacker or ezgif. Tell me the frame count.
   - **Animated SVG**. Hand the SVGs to me and I will animate eyelids, breathing and the "z" with CSS, as the placeholders do. No extra package, smallest files, but the motion stays simple.
4. **Drop the files in** `app/src/lib/mascot/art/` with the names above (any of `.svg`, `.riv`, `.json`, `.png`) and tell me which route you took; `Orc.svelte` is the only file that changes.

## Prompts

Character sheet:
> Character design sheet of an original cute fantasy orc mascot for a minimalist productivity app. Chunky round body, short legs, big hands, small lower tusks, pointed ears, heavy brow, grumpy but lovable expression. Moss-green skin, simple brown leather tunic and belt, small wooden club. Flat vector illustration, clean dark outlines, 4 colours, no gradients, no texture. Front view, three-quarter view and side view, standing, neutral pose, plain white background, consistent proportions.

Sleeping:
> The same orc character as the reference image, fast asleep, slumped sitting against a small rock, chin on chest, eyes closed, club lying on the ground beside him, a small "z" above his head. Flat vector mascot illustration, clean dark outlines, same colours as the reference, transparent background, three-quarter view facing left, centred with padding, square canvas.

Guarding:
> The same orc character as the reference image, standing firmly in the middle of a narrow stone path, feet apart, club planted on the ground in front of him with both hands on the handle, stern determined frown, "none shall pass" attitude but still cute. Flat vector mascot illustration, clean dark outlines, same colours as the reference, transparent background, three-quarter view facing left, centred with padding, square canvas.

Drinking:
> The same orc character as the reference image, sitting down, drinking from a tiny round glass potion bottle with glowing purple liquid, eyes getting sleepy, slight wobble. Flat vector mascot illustration, same style and colours as the reference, transparent background, three-quarter view facing left, square canvas.

App icon:
> App icon: head and shoulders of the same orc character as the reference image, front-facing, slightly smug grin with small tusks, on a rounded-square moss-green background, flat vector, bold simple shapes that stay readable at 32 px, no text.

## Sizes and formats

- SVG: under 60 KB per state, shapes grouped and named (`body`, `head`, `eyelids`, `club`, `z`) so they can be animated.
- PNG sprite sheet: 512 × 512 per frame, transparent, under 600 KB per sheet.
- Rive/Lottie: under 150 KB per file.
- Icon: a 1024 × 1024 PNG; `npm --prefix app run tauri icon path/to/icon.png` generates every size.
