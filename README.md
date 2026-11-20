# DockForge

An open-source, themeable dock for Windows 10 and 11. Think *Wallpaper Engine, but for docks*: the community makes the themes, and installing one takes a click.

> **Status: v0.3 preview.** A working dock with favorites, groups, modules, ready-made docks and a settings window. It has only been compiled and screenshot-tested so far; it does not replace the Windows taskbar yet.

## What works today

**The dock**
- Three layouts: *floating* (sized to its icons), *edge to edge* (like the Windows 10 taskbar) or *island* (a small pill with the app in front and the widgets; the other apps appear when the pointer is over it)
- Any screen edge, and for floating docks and islands the start, middle or end of that edge (so a dock can live in a corner)
- Favorites that stay when closed (click to start them), then the other open apps, with an optional line between them. Favorites can be imported from the Windows taskbar, or turned off
- Groups, switched from anywhere with **Alt+1 … Alt+9** (four groups to start, up to nine). Group 1 has the favorites and everything open; groups 2 to 4 start empty. Move the app in front to a group with **Alt+Shift+number**, drag its icon onto a group dot, or use the right-click menu
- Optional app names next to the icons, and apps at the start, middle or end of an edge-to-edge bar
- Click: bring to front / minimize, cycle through the app's windows, or only bring to front (your choice)
- Right-click menu: the app's windows, new window, add to / remove from favorites, move to group, close, close all
- Hides the Windows taskbar while DockForge is open (on by default; turn it off in Sistema → *Esconder a barra de tarefas do Windows*). It comes back when DockForge closes, and on the next start if DockForge was killed
- A dock that is always visible keeps its strip of the screen, so maximized windows end where it begins (can be turned off)
- Gets out of the way of anything in full screen (games, videos, F11): the dock is hidden until full screen ends
- Auto-hide (on by default): the dock slides out through the screen edge and comes back when the mouse touches it. By default it only hides while the window in front is maximized, and stays in view over smaller windows and the desktop
- Click-through: the empty space around the dock (room for the hover zoom and the tooltips) does not block clicks
- Modules: a **clock** and a **quick panel** (volume and shortcuts to the Windows settings). Integrations (now playing, system stats, pomodoro, world clock) were removed for now, to be rethought
- Motion: icons pop in when they appear, a starting app bounces, the group that receives an app blinks. All of it is off when Windows is set to reduce motion

**The settings window** (tray icon → *Abrir o DockForge…*, or start DockForge again)
- Ready-made docks: ten combinations of theme, place on screen and modules, applied with one click and still fully editable
- Store with live previews of every installed theme, settings for your dock on the left, favorites and groups, start with Windows, language
- Twelve built-in themes, among them the looks people most often give their Windows taskbar: Barra clássica (flat, with app names), Transparente, Barra flutuante (rounded, off the edges), Segmentada (apps and widgets in separate blocks), Retrô cinza (90s bevels) and Prateleira (frosted glass shelf); any theme dropped in `%APPDATA%\DockForge\themes\`
- Live reload: save a theme's `styles.css` and the dock updates within a second

## Download

Get the latest version from [Releases](https://github.com/b-boldrin/dockforge/releases/latest):

- `DockForge_<version>_setup_x64.exe`: installer (recommended).
- `DockForge_<version>_x64.msi`: the same, as an MSI.
- `DockForge_<version>_portable_x64.zip`: no installer; unzip and open `DockForge.exe`. Needs the
  Microsoft Edge WebView2 Runtime, which Windows 10 and 11 usually already have.

The builds are not signed yet, so Windows SmartScreen may warn on first run ("More info" → "Run anyway").

To publish a new version: bump the version in `package.json`, `src-tauri/Cargo.toml` and
`src-tauri/tauri.conf.json`, and push to `main`. GitHub Actions sees a version with no release yet, builds it and publishes it
with the tag `v<version>`.

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
2. **Dock MVP** (now): ~~favorites, groups, auto-hide, layouts, island, widgets, settings window~~; still to do: multi-monitor, window previews
3. **Community**: in-app gallery from the themes repository, integrations again (now playing, system, agent usage for Claude Code, OpenCode and Codex), sandboxed JS widgets
4. **v1.0**: signed installer, winget, website
5. **Ecosystem**: visual theme editor, publish themes from the app (optional GitHub sign-in)

## Known limitations

- Store (UWP) apps show a letter instead of their icon, each window is its own entry, and they cannot be pinned yet.
- Only the monitor the dock is on is used.
- Alt+1 … Alt+4 and Alt+Shift+1 … Alt+Shift+4 are taken from other programs while groups are on (turn groups off to give them back).
- Importing favorites reads the shortcuts Windows keeps for the taskbar: Store apps and File Explorer have no program path there and are skipped, and the order is alphabetical.
- The island only lives on the top or bottom edge.
- The settings window is only in Portuguese for now; the language option changes the dock and the menus.
- Desktop blur behind the dock (acrylic) is not implemented; `backdrop-filter` in themes only blurs the dock's own content.

## License

MIT. Themes in dockforge-themes have their own licenses.
