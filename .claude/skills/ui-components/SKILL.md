---
name: ui-components
description: The Material 3 design tokens, components, theming and screen layout of the OrcBlock app UI (Svelte 5 + TypeScript in app/src). Use when adding or changing components, styles or screens.
paths: "app/src/**"
---

Look: quiet, utilitarian Material 3. One column, lots of air, neutral grey surfaces, one restrained green accent, large tabular numerals for time, and the orc only as a small line mark next to the state. Light, dark, and System (default, follows `prefers-color-scheme` live).

## Tokens (`app/src/lib/theme.css`)
- Colour roles, neutral surfaces with a single green accent (`#2E6A4B` light, `#9DD4B3` dark), pasted as constants: `--md-sys-color-primary`, `on-primary`, `primary-container`, `on-primary-container`, `secondary(-container)`, `error(-container)`, `surface`, `surface-container-lowest` … `surface-container-highest`, `on-surface`, `on-surface-variant`, `outline`, `outline-variant`, `inverse-surface`, `inverse-on-surface`. Light values on `:root`, dark under `[data-theme="dark"]` and under `@media (prefers-color-scheme: dark)` for `[data-theme="system"]`.
- Type scale: `--md-sys-typescale-{display,headline,title,body,label}-{large,medium,small}` as `font` shorthands; font stack is the system UI font (SF Pro on macOS), numerals tabular for timers.
- Shape: `--md-sys-shape-corner-{extra-small,small,medium,large,extra-large,full}` (4, 8, 12, 16, 28, 9999 px).
- Elevation: `--md-sys-elevation-{0..3}` as box-shadows; prefer tonal surfaces over shadows.
- Motion: `--md-sys-motion-easing-standard`, `--md-sys-motion-duration-{short,medium}`; all disabled under `prefers-reduced-motion`.
- Spacing: `--space-{1..8}` (4, 8, 12, 16, 24, 32, 48, 64 px).
Never write a literal colour outside theme.css.

## Components (`app/src/lib/components`)
- `TopBar`: app title left, `ThemeMenu` (icon button opening a menu: System, Light, Dark with a check on the current one) right. Draggable region for the frameless window.
- `Button`: `variant` = `filled` (one per screen, the main action), `tonal`, `outlined`, `text`; `icon` optional; `wide` fills the container at 48 px; state layer on hover/focus/press via `::before` opacity tokens.
- `IconButton`: 40 px target, `aria-label` required, `type="submit"` when it submits a form.
- `TextField`: M3 outlined, 56 px, label floats into a notch in the outline (fieldset/legend), optional `trailing` snippet for an icon button, supporting or error text always reserving its line.
- `Slider`: M3 slider over a native range input (keyboard and screen readers for free), `valuetext` required.
- `DurationPicker`: large readout (numbers in display-medium, units in title-large), "Ends at 17:42", and a `Slider` over `DURATION_STOPS` (15 min to 7 days).
- `SiteRow`: M3 two-line list item: 40 px `Avatar` (favicon, or the first letter on a neutral circle), the cleaned domain, "Added Oct 17" or "Blocked until the timer ends", a remove `IconButton` or a lock icon during a block. Rows sit in a `surface-container-low` list with dividers.
- `Countdown`: display-large tabular `2:14:09`, an M3 linear progress (active and remaining segments), "Blocking 3 sites · until 17:42", and `aria-live="polite"` text updated once a minute.
- `PotionShelf`: five short bars, filled for available; "3 of 5 left · more on Nov 1".
- `Dialog`: modal, focus trapped, Escape closes, scrim `--md-sys-color-scrim` at 32 %.
- `Snackbar`: bottom, `inverse-surface`, `role="status"`, auto-dismiss 4 s.
- `Orc` (in `lib/mascot`): a line mark; see the mascot skill.

## Screens (`app/src/routes` are plain components switched by state, no router)
- `Setup`: helper not installed. A panel with the mark on a primary-container tile, "Install the helper", one paragraph on why, and a wide filled "Install helper" button (asks for the admin password once).
- `Home` idle: a panel with the state line (mark + "Idle"), the `DurationPicker` and a wide filled "Start blocking" (disabled with a hint while the list is empty); below, "Sites" with the add field and the list.
- `Home` blocking: the panel shows the state line (mark + "Guarding") and the `Countdown`; the sites list locks removal but still accepts new sites; "Sleep potions" shows the `PotionShelf` and a wide tonal "End early with a potion" that opens a confirm `Dialog` ("Use 1 of 3 potions to end this block now?").

## Rules
- One filled button per screen. Destructive or irreversible actions (potion) confirm in a `Dialog`.
- Domains always go through `formatDomain` (Unicode display of the stored ASCII form).
- Works at the minimum window size (360 × 560) without horizontal scroll.
- Every interactive element has a visible `:focus-visible` ring in `--md-sys-color-primary`.
- Add new components here in the same change that introduces them.
