// Development only: answers the commands the settings window sends to the Rust side, so
// `npm run dev` + http://localhost:1420/?view=hub works in a plain browser (no Tauri window).
// main.ts imports this only when import.meta.env.DEV is true and there is no Tauri runtime, so
// the production bundle never contains it. Settings live in memory; reloading the page resets them.

import { mockIPC } from "@tauri-apps/api/mocks";
import type { AppEntry, Settings, ThemePayload, ThemeSummary } from "./lib/types";

// The real themes, read from the Rust side's folder, so previews look like the app.
const manifests = import.meta.glob("../src-tauri/themes/*/theme.json", { eager: true, query: "?raw", import: "default" }) as Record<string, string>;
const sheets = import.meta.glob("../src-tauri/themes/*/styles.css", { eager: true, query: "?raw", import: "default" }) as Record<string, string>;

const idOf = (path: string) => path.split("/").slice(-2)[0];
const themes: ThemeSummary[] = Object.entries(manifests).map(([path, raw]) => {
  const m = JSON.parse(raw);
  return { id: idOf(path), name: m.name, author: m.author?.name ?? "DockForge", tags: m.tags ?? [], built_in: true };
});
const css = Object.fromEntries(Object.entries(sheets).map(([path, raw]) => [idOf(path), raw]));

let settings: Settings = {
  version: 2,
  theme: "natal-cristal",
  position: "bottom",
  iconSize: 48,
  magnify: 1.3,
  appScale: 1,
  autoHide: true,
  hideOnlyMaximized: true,
  reserveSpace: true,
  hideDelay: 600,
  showMode: "both",
  clickAction: "focus-min",
  separator: true,
  labels: false,
  accent: null,
  fill: null,
  radius: null,
  opacity: null,
  gap: null,
  indicator: "theme",
  pins: [
    { name: "Explorador", path: "C:\\Windows\\explorer.exe", group: 0 },
    { name: "Navegador", path: "C:\\Apps\\browser.exe", group: 0 },
    { name: "Código", path: "C:\\Apps\\code.exe", group: 0 },
  ],
  saved: [],
  startWithWindows: false,
  hideTaskbar: true,
  hubTheme: "light",
  language: "pt",
  motion: "on",
  layout: "float",
  align: "center",
  favoritesEnabled: true,
  groupsEnabled: false,
  activeGroup: 0,
  groupNames: ["Principal", "Estudo", "Jogos", "Criação"],
  elements: null,
};

const app = (name: string, pinned: boolean, running: boolean, active = false): AppEntry => ({
  key: `C:\\Apps\\${name.toLowerCase()}.exe`,
  name,
  exe: `C:\\Apps\\${name.toLowerCase()}.exe`,
  windows: running ? [{ hwnd: name.length, title: name }] : [],
  active,
  pinned,
});
const apps: AppEntry[] = [
  app("Explorador", true, true),
  app("Navegador", true, true, true),
  app("Código", true, false),
  app("Música", false, true),
  app("Chat", false, true),
  app("Terminal", false, true),
];

mockIPC(
  (cmd, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case "get_settings":
        return settings;
      case "update_settings":
        settings = { ...(args.settings as Settings) };
        return settings;
      case "reset_settings":
        return settings;
      case "list_apps":
        return apps;
      case "list_themes":
        return themes;
      case "load_theme":
      case "get_theme": {
        const id = (args.id as string | undefined) ?? settings.theme;
        const payloadOut: ThemePayload = { id, name: themes.find((t) => t.id === id)?.name ?? id, css: css[id] ?? "" };
        return payloadOut;
      }
      case "get_icon":
        return null;
      case "plugin:app|version":
        return "dev";
      default:
        return null;
    }
  },
  { shouldMockEvents: true },
);
