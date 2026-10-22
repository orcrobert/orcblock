---
name: ui-components
description: The Swiss brutalist design tokens, components, theming and screen layout of the OrcBlock app UI (Svelte 5 + TypeScript in app/src). Use when adding or changing components, styles or screens.
paths: "app/src/**"
---

Look: Swiss industrial print brutalism. Follow the industrial-brutalist-ui skill (archetype 2.1); this file records how OrcBlock applies it.

## Frame
- One window, 900 x 520 by default, resizable only between 840 x 490 and 1080 x 620 (`app/src-tauri/tauri.conf.json`). Nothing grows or scrolls except the target list inside its compartment (and, inside its dialog, the app picker's grid); `html`/`body` never scroll or bounce.
- Two compartments split by a 2 px rule: left (56 %, 500-600 px) is control: status strip, readout + orc, preset or progress strip, one action. Right is targets: header, prompt, help line, list.
- Every zone is bounded by visible 1 px or 2 px rules. No border-radius, no shadows, no gradients except hatch patterns and the progress ticks.

## Tokens (`app/src/lib/theme.css`)
- Colour: `--paper`, `--ink`, `--ink-muted`, `--rule`, `--red` (the only accent), `--on-red`, `--hatch`, and `--potion`, a purple used for potion liquid and nothing else. Light is paper `#F4F4F0` with carbon ink and red `#D81414` (darker than the skill's `#E61919` so small red text meets WCAG AA); dark is the same system inverted (`#121212`, `#EAEAEA`, red `#FF2A2A`). System follows `prefers-color-scheme` live. Never mix light and dark in one view; never write a literal colour outside theme.css.
- Type: system fonts only. `--type-macro` (SF Pro 900, 96-116 px with the window, line-height 0.85, tracking -0.05em) for the one big readout; `--type-headline` for dialog and setup titles; `--type-label` for buttons; `--type-micro` and `--type-data` (SF Mono) for everything else, uppercase with `0.08em` tracking via the `.micro` class.
- Size: `--row` 40 px (list rows, preset cells), `--gutter` 14 px, `--rule-thin` 1 px, `--rule-thick` 2 px.
- Texture: a global SVG grain on `body::after`; hatch fills for empty and disabled states.

## Components (`app/src/lib/components`)
- `TopBar`: `[ ORCBLOCK ]`, `REV x.y.z`, `ThemeSwitch` (cycles SYS / LGT / DRK). Drag region; left padding clears the traffic lights.
- `StatusBar`: `UNIT / ORC-01`, a square status light and `STATUS: IDLE | ARMED | HELPER MISSING` (red when armed), `DRY-RUN` badge in dev.
- `Button`: full-bleed cell button, `solid` (the one main action per screen), `alert` (red, the potion), `plain`. Label left, `>>>` right. Disabled is hatched.
- `DurationPicker`: `[ DURATION ]`, the macro readout `H:MM`, `ENDS 17:42`, and an 8-cell strip `− 25M 1H 2H 4H 8H 1D +` (minus and plus step through `DURATION_STOPS`, 15 min to 7 days). Double-clicking the readout (or activating it with the keyboard or VoiceOver) turns it into an input in the same type with a red underline: `1:30`, `90`, `90m`, `2h`, `1h30`, `2d 4h` (`parseDuration`, 1 min to 7 days); Enter or blur sets it, Escape cancels, out-of-range shows `OUT OF RANGE // 1M TO 7D` in red.
- `Countdown`: `[ TIME REMAINING ]`, macro `H:MM` with the seconds smaller and red, `ENDS 17:42 // N TARGETS // 42%`, and a full-width red progress strip with 10 % ticks. From 10 hours up the readout shrinks to 72 % of the macro size (`--macro-size`) so 167:59 fits.
- `SiteRow`: `01`, a grayscale favicon (or an empty square), the uppercase domain, the date added and `[ × ]`, or `LOCKED` in red during a block.
- `AppRow`: like `SiteRow` for an app: its icon, the name, an `APP` tag, the date and `[ × ]` or `LOCKED`.
- `ListSwitcher`: the targets header: a tick (whether the shown list is in the next block; only with several lists, hidden during a block), `[ LIST NAME ] ▾` and the count (`LOCKED // 07` when the list is in the running block). The ▾ opens a menu over the list: each list with its tick (or a red `■` for lists in the running block), name and count, a hint line, and `+ NEW LIST`, `RENAME`, `DELETE` (each through a `Dialog`). The shown list and the unticked lists live in `localStorage` (`lib/lists.ts`); new lists start ticked.
- `AppIcon`: an app's icon from `app_icon`, grayscale, or a two-letter monogram in a hairline square.
- `AppPicker`: a wide `Dialog`: a filter prompt (Enter adds the top match), a grid of installed apps with `OPEN NOW` first, and `BROWSE IN FINDER… >>>`. Apps already listed are not offered; the order lives in `lib/picker.ts`. Typing in the filter is the fast path; each tile is still a Tab stop. Icons are fetched once per app per session.
- `PotionVials`: five 1-bit pixel vials at the top of the orc box, purple liquid for each potion left, empty outlines for used ones; always visible on Home.
- `PotionShelf`: a `<dl>` beside the potion button during a block: `SLEEP POTIONS`, `3/5 LEFT`, `REFILL 2026-11-01`.
- `Dialog`: square (or `wide` for a grid), 2 px ink border, hatched backdrop, red tag strip `[ CONFIRM // SLEEP POTION ]`, headline title, two cell buttons.
- `Snackbar`: full-width red strip at the bottom, `ERR // message`, 5 s.
- `OrcBox`: the orc's compartment: vials on top, the sprite, `ORC-01 // ASLEEP | ON GUARD | DRINKING`. `drink(slot)` stages the potion: the vial falls into the hand, the orc drinks and falls asleep, the empty vial climbs back.
- `Orc` and `Vial` (in `lib/mascot`): see the mascot skill.

## Screens (`app/src/routes`, switched by state, no router)
- `Setup`: left: `HELPER MISSING`, macro `HELPER REQUIRED`, sleeping orc, `INSTALL HELPER >>>`. Right: `[ WHAT IT DOES ]` with four numbered facts.
- `Home` idle: left: `IDLE`, the duration picker beside the orc box (potion vials, sleeping orc, caption), `ARM // START BLOCK >>>` (`ARM // ALL LISTS` or `ARM // SOCIAL + WORK` with several lists; disabled while the ticked lists hold no targets; when listed apps are open it first asks `CLOSE OPEN APPS?`). Right: targets: sites, then apps; `+ APP` opens the picker, and dropping a `.app` on the window adds it (the list gets a dashed red outline while dragging).
- `Shield`: a second, borderless 420 x 300 window (label `shield`, same page; `main.ts` picks the route by label) on top of everything when the daemon closes a blocked app: red tag strip `[ BLOCKED // ORC-01 ]`, the guarding orc, `<APP> IS BLOCKED` with the name in red, `TIME LEFT 0:19:00`, a red strip draining over 6 s (still with reduced motion) and `OK >>>`. Escape, OK or the timer closes it; a new closed app restarts the timer, the pointer over the shield holds it (the strip pauses) until it leaves, and any key but Escape holds it until OK, so keyboard and VoiceOver users have time to read it. The title is a live region, so a second closed app is announced. The red strip is its drag handle.
- `Home` blocking: left: `ARMED`, the countdown beside the guarding orc, potions and `DRINK >>>` behind a confirm dialog; after a potion the orc plays `drinking`, then sleeps. Right: targets with removal locked on lists in the block (adding still allowed and blocked at once); other lists stay editable.

## Rules
- One solid button per screen; the potion is the only red button and always confirms.
- Text colours meet WCAG AA (4.5:1) on paper in both themes; check new tokens before using them for text.
- Domains display in uppercase via CSS; the stored form is never changed.
- Semantic tags where they fit: `<output>` for the readout, `<dl>`/`<data>` for potions, `<figure>` for the orc.
- Every interactive element shows a 2 px red `:focus-visible` outline.
- Add new components here in the same change that introduces them.
