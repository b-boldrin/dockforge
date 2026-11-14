# DockForge

An open-source, themeable dock for Windows 10 and 11. Think *Wallpaper Engine, but for docks*: the community makes the themes, and installing one takes a click.

> **Status: v0.2 preview.** A working dock with pinned apps, groups, widgets and a settings window. It does not replace the Windows taskbar yet.

## What works today

**The dock**
- Two layouts: *floating* (sized to its icons) or *edge to edge* (like the Windows 10 taskbar, clock at the end)
- Any screen edge: bottom, top, left or right
- Pinned apps that stay when closed (click to start them), then the other open apps, with an optional line between them
- Groups: four sets of pinned apps, switched from anywhere with **Alt+1 … Alt+4**
- Click: bring to front / minimize, cycle through the app's windows, or only bring to front (your choice)
- Right-click menu: the app's windows, new window, pin/unpin, close, close all
- Auto-hide that slides through the screen edge and comes back when the mouse touches it
- Click-through: the empty space around the dock (room for the hover zoom and the tooltips) no longer blocks clicks
- Widgets: **now playing** (anything in Windows' media controls: Spotify, YouTube Music in a browser, most players) with cover and buttons, and a clock

**The settings window** (tray icon → *Abrir o DockForge…*, or start DockForge again)
- Store with live previews of every installed theme, settings for your dock on the left, pinned apps per group, start with Windows, language
- Four built-in themes: Forge Glass, Midnight Pill, Natal Cristal and Noite de Natal; any theme dropped in `%APPDATA%\DockForge\themes\`
- Live reload: save a theme's `styles.css` and the dock updates within a second

## Run it (Windows)

Requirements: [Rust](https://rustup.rs) (stable, MSVC), [Node.js](https://nodejs.org) 22+, and the WebView2 runtime (already on Windows 11).

```powershell
git clone https://github.com/dockforge/dockforge
cd dockforge
npm install
npm run tauri dev      # run with hot reload
npm run tauri build    # installer in src-tauri/target/release/bundle/
```

On macOS or Linux the app also runs, with sample apps instead of real windows, which is handy for working on the interface and themes.

## How it is built

| Part | Where | What it does |
| --- | --- | --- |
| Window list, focus, icons, events | `src-tauri/src/platform/windows.rs` | Win32 API through Microsoft's `windows` crate. `EnumWindows` + the same filters the taskbar uses; `SetWinEventHook` to hear about changes; shell image lists for icons |
| Themes | `src-tauri/src/themes.rs` | Finds themes, rewrites `url()` to embedded data so only files inside the theme are used, drops `@import` |
| Commands, tray, watchers | `src-tauri/src/lib.rs` | Tauri commands the dock calls, debounced app-list updates, theme hot reload |
| Pinned apps and groups | `src-tauri/src/dock.rs`, `settings.rs` | Merges running apps with pins; every setting from the web side is clamped before it is saved |
| Now playing | `src-tauri/src/platform/windows.rs` (end) | `GlobalSystemMediaTransportControlsSessionManager`, the API behind the Windows volume flyout |
| Dock UI | `src/lib/Dock.svelte`, `src/lib/base.css` | Svelte 5. Fits the native window to the dock size and reports the dock's rectangle for click-through |
| Settings window | `src/hub/` | Opened as a second window (`index.html?view=hub`). Theme previews are limited with CSS `@scope`, so no theme can restyle the window |

**Approach:** the dock is a normal transparent window drawn on top of the desktop. It does not inject code into Explorer, so Windows updates do not break it and antivirus software has nothing to flag.

**Security model for themes:** themes are CSS only. The web view runs under a strict Content-Security-Policy (no remote scripts, styles, images or connections) and the web side has no file system or shell access. Theme CSS is applied as a constructed stylesheet, so no inline styles are needed. JavaScript widgets come in phase 3, in a sandbox described in the design doc.

## Making themes

Themes live in a separate repository: **[dockforge-themes](https://github.com/dockforge/dockforge-themes)**. Start with its `THEMING.md`.

## Roadmap

1. ~~Technical prototype~~
2. **Dock MVP** (now): ~~pinned apps, groups, auto-hide, layouts, widgets, settings window~~; still to do: multi-monitor, optional hiding of the Windows taskbar, window previews
3. **Community**: in-app gallery from the themes repository, agent usage widget (Claude Code, OpenCode, Codex), island, sandboxed JS widgets
4. **v1.0**: signed installer, winget, website
5. **Ecosystem**: visual theme editor, publish themes from the app (optional GitHub sign-in)

## Known limitations

- Store (UWP) apps show a letter instead of their icon, each window is its own entry, and they cannot be pinned yet.
- Only the monitor the dock is on is used.
- Alt+1 … Alt+4 are taken from other programs while groups are on (turn groups off to give them back).
- The settings window is only in Portuguese for now; the language option changes the dock and the menus.
- Desktop blur behind the dock (acrylic) is not implemented; `backdrop-filter` in themes only blurs the dock's own content.

## License

MIT. Themes in dockforge-themes have their own licenses.
