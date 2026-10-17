# 0004: Tauri 2 with Svelte 5 for the desktop app

Status: accepted (2026-10-17)

## Context

The app needs a native-feeling macOS window with Material 3 styling, light and dark themes that follow the system, looping character animations (the orc), favicons, a tray item and an installer, written in Rust. Later it should run on Windows and Linux.

Options: native Rust GUI toolkits (iced, egui, Slint), a Swift/AppKit shell, or Tauri (system WebView + Rust backend).

## Decision

- **Tauri 2.** Rust backend in `app/src-tauri`, system WKWebView on macOS (no bundled Chromium; the app stays a few MB), bundles `.app` and `.dmg`, ships the daemon as a sidecar binary, supports a tray icon, and has a capability system that limits what the web side can call. Cross-platform when we get there.
- **Svelte 5 + TypeScript + Vite** for the UI: compiles to small vanilla JS, reactive state with runes, no virtual DOM. Material 3 is implemented as our own CSS tokens and a few components instead of a component library. Material Web is in maintenance mode, and the screens are few.
- Animations are CSS/SVG first (the placeholder orc), so no animation runtime is needed until final art requires one.

## Consequences

- iced/egui would have meant hand-building theming, text input and animation; Slint would have meant learning a DSL with a weaker animation story. A web UI makes the orc and Material styling cheap.
- Two toolchains (Cargo and npm). `make` hides that, and CI caches both.
- The Tauri command layer is the only bridge; the web side never talks to the daemon directly.
