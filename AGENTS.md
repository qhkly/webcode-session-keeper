# AGENTS.md

This file provides guidance to the AI agent when working with code in this repository.

## Build & Run

- `npm run dev` runs `tauri dev` (starts Rust backend + opens dev webview).
- `npm run build` runs `tauri build` (produces release bundle under `src-tauri/target/release/bundle/`).
- There is **no frontend build step**. The `src/` directory is served directly as static files (`frontendDist: "../src"` in `tauri.conf.json`). Do not add a bundler or expect compiled output.
- React, ReactDOM, and Babel are vendored locally in `src/vendor/` and loaded via `<script>` tags. JSX is compiled in-browser by Babel standalone.

## Architecture

- **Tauri 2** (not 1) with `unstable` and `macos-private-api` features enabled.
- Frontend communicates with Rust via `window.Bridge` (`src/bridge.js`). Always use `Bridge.*` methods from components — never call `__TAURI__.core.invoke` directly.
- Webview labels follow a strict convention: `site-{id}` for site webviews, `tabbar` for the tab bar. This is used in Rust commands, bounds management, and event routing.
- Webview positions/sizes are managed manually in Rust (`update_all_bounds`) on every window resize event.
- macOS-specific Rust code (e.g. `TitleBarStyle::Transparent`) must be guarded with `#[cfg(target_os = "macos")]`.

## Conventions

- UI text is in Chinese (Simplified). Follow this when adding user-facing strings.
- Config is stored at `~/.config/webcode-session-keeper/config.json` (platform config dir, not app data).
- Releases are triggered by pushing tags matching `v*` (e.g. `git tag v0.2.0 && git push origin v0.2.0`).
- Linux CI requires system packages: `libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`, `libssl-dev`, `libgtk-3-dev`.
