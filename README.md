# webcode-session-keeper

Session Keeper is a Tauri 2 desktop app for keeping frequently used web sessions alive in one lightweight window. It embeds a fixed tab bar webview at the top and one real external webview per site underneath, so each site can keep its own cookies and login state without iframe restrictions.

## Features

- Single Tauri window with multiple child webviews.
- Built-in tabs for Google Calendar, Tencent Docs, Cloudflare, Chrome Web Store console, and extension reviews.
- Periodic keep-alive heartbeat that checks each webview host and runs a credentialed `HEAD` fetch against the current page.
- Status dots for active, expired, unknown, and error states.
- Local JSON config stored at `~/.config/webcode-session-keeper/config.json`.
- Vanilla HTML/CSS/JS frontend with React and Babel loaded from local vendor scripts.

## Development

Install dependencies:

```bash
npm install
```

Run locally:

```bash
npm run dev
```

Build the desktop app:

```bash
npm run build
```

The macOS build output is written under `src-tauri/target/release/bundle/`.

## Configuration

On first launch the app creates:

```text
~/.config/webcode-session-keeper/config.json
```

Edit that file to replace placeholder URLs with the exact pages you want to keep alive, or add sites from the tab bar.

## Automated Builds

GitHub Actions builds installable artifacts for macOS, Linux, and Windows. The workflow can be started manually from GitHub Actions, and it also runs when a tag matching `v*` is pushed.

Create a tagged release:

```bash
git tag v0.1.0
git push origin v0.1.0
```

Tagged builds upload all platform artifacts to a GitHub Release.
