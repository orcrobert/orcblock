---
name: ui-components
description: The Material 3 design tokens, components, theming and screen layout of the OrcBlock app UI (Svelte 5 + TypeScript in app/src). Use when adding or changing components, styles or screens.
paths: "app/src/**"
---

Look: minimal Material 3. One column, lots of air, rounded surfaces, one accent colour (a mossy orc green), the orc as the only illustration. Light, dark, and System (default, follows `prefers-color-scheme` live).

## Tokens (`app/src/lib/theme.css`)
- Colour roles, generated from seed `#4F6F3A` with the Material colour utilities and pasted as constants: `--md-sys-color-primary`, `on-primary`, `primary-container`, `on-primary-container`, `secondary(-container)`, `tertiary(-container)` (potion purple), `error(-container)`, `surface`, `surface-container-lowest` … `surface-container-highest`, `on-surface`, `on-surface-variant`, `outline`, `outline-variant`, `inverse-surface`, `inverse-on-surface`. Light values on `:root`, dark under `[data-theme="dark"]` and under `@media (prefers-color-scheme: dark)` for `[data-theme="system"]`.
- Type scale: `--md-sys-typescale-{display,headline,title,body,label}-{large,medium,small}` as `font` shorthands; font stack is the system UI font (SF Pro on macOS), numerals tabular for timers.
- Shape: `--md-sys-shape-corner-{extra-small,small,medium,large,extra-large,full}` (4, 8, 12, 16, 28, 9999 px).
- Elevation: `--md-sys-elevation-{0..3}` as box-shadows; prefer tonal surfaces over shadows.
- Motion: `--md-sys-motion-easing-standard`, `--md-sys-motion-duration-{short,medium}`; all disabled under `prefers-reduced-motion`.
- Spacing: `--space-{1..8}` (4, 8, 12, 16, 24, 32, 48, 64 px).
Never write a literal colour outside theme.css.

## Components (`app/src/lib/components`)
- `TopBar`: app title left, `ThemeMenu` (icon button opening a menu: System, Light, Dark with a check on the current one) right. Draggable region for the frameless window.
- `Button`: `variant` = `filled` (one per screen, the main action), `tonal`, `outlined`, `text`; `icon` optional; state layer on hover/focus/press via `::before` opacity tokens.
- `IconButton`: 40 px target, `aria-label` required.
- `TextField`: outlined, floating label, supporting/error text tied with `aria-describedby`.
- `Chip`: filter chips for duration presets; `aria-pressed`.
- `Card`: `surface-container-low`, corner large, no shadow.
- `SiteRow`: favicon (24 px, rounded small) or `Monogram` fallback (first letter on a tonal circle whose hue comes from the domain), cleaned domain (`title-medium`, ellipsis), remove `IconButton`; remove is disabled with a tooltip while the site is in the active block.
- `PotionShelf`: five flask icons, filled for available, outlined for used this month; label "3 sleep potions left this month".
- `Countdown`: `display-large` tabular `2:14:09`, linear progress underneath, `aria-live="polite"` text updated once a minute.
- `Dialog`: modal, focus trapped, Escape closes, scrim `--md-sys-color-scrim` at 32 %.
- `Snackbar`: bottom, `inverse-surface`, `role="status"`, auto-dismiss 4 s.
- `Orc` (in `lib/mascot`): see the mascot skill.

## Screens (`app/src/routes` are plain components switched by state, no router)
- `Setup`: helper not installed. Orc sleeping, one paragraph on why a helper is needed, filled "Install helper" button (asks for the admin password once).
- `Home` idle: Orc sleeping, "The path is open", sites `Card` with `TextField` + add and the list, duration chips (25 min, 1 h, 2 h, 4 h, Custom), filled "Start blocking".
- `Home` blocking: Orc guarding, "None shall pass", `Countdown`, the sites (add allowed, remove disabled), `PotionShelf` and a tonal "Drink a sleep potion" button that opens a confirm `Dialog` ("Use 1 of 3 potions to end this block now?").

## Rules
- One filled button per screen. Destructive or irreversible actions (potion) confirm in a `Dialog`.
- Domains always go through `formatDomain` (Unicode display of the stored ASCII form).
- Works at the minimum window size (360 × 560) without horizontal scroll.
- Every interactive element has a visible `:focus-visible` ring in `--md-sys-color-primary`.
- Add new components here in the same change that introduces them.
