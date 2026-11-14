import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AppEntry, MediaInfo, Settings, SystemStats, ThemeSummary } from "../lib/types";
import type { Preset } from "./presets";
import { refreshPreviewTheme } from "./themes.svelte.js";

/** Everything the settings window shows, kept in sync with the Rust side. */
export const hub = $state({
  settings: null as Settings | null,
  apps: [] as AppEntry[],
  themes: [] as ThemeSummary[],
  error: "",
});

/** Runs one startup step; if it fails, the error says which step. */
async function step<T>(name: string, run: () => Promise<T>): Promise<T> {
  try {
    return await run();
  } catch (err) {
    throw new Error(`${name}: ${err instanceof Error ? err.message : String(err)}`);
  }
}

export async function start() {
  await step("ouvir settings-changed", () => listen<Settings>("settings-changed", (e) => (hub.settings = e.payload)));
  await step("ouvir apps-changed", () => listen<AppEntry[]>("apps-changed", (e) => (hub.apps = e.payload)));
  await step("ouvir theme-changed", () => listen("theme-changed", () => hub.settings && refreshPreviewTheme(hub.settings.theme)));
  // The lists are not needed to draw the window, so a failure there must not leave it blank.
  hub.settings = await step("get_settings", () => invoke<Settings>("get_settings"));
  hub.apps = await step("list_apps", () => invoke<AppEntry[]>("list_apps")).catch((err) => {
    hub.error = String(err);
    return [];
  });
  hub.themes = await step("list_themes", () => invoke<ThemeSummary[]>("list_themes")).catch((err) => {
    hub.error = String(err);
    return [];
  });
}

/** Changes some settings. The Rust side checks and saves them, then tells every window. */
export async function patch(changes: Partial<Settings>) {
  if (!hub.settings) return;
  const next = { ...$state.snapshot(hub.settings), ...changes };
  hub.settings = next; // instant feedback; replaced by the checked version below
  try {
    hub.settings = await invoke<Settings>("update_settings", { settings: next });
    hub.error = "";
  } catch (err) {
    hub.error = String(err);
    hub.settings = await invoke<Settings>("get_settings");
  }
}

/** Brings the apps pinned to the Windows taskbar into the favorites. Returns how many came. */
export async function importFavorites(): Promise<number> {
  try {
    const added = await invoke<number>("import_taskbar_pins");
    hub.settings = await invoke<Settings>("get_settings");
    hub.error = "";
    return added ?? 0;
  } catch (err) {
    hub.error = String(err);
    return 0;
  }
}

export function useTheme(id: string) {
  // A new theme starts clean: tweaks made on the old one do not carry over.
  return patch({ theme: id, accent: null, radius: null, opacity: null });
}

export function usePreset(preset: Preset) {
  return patch(preset.settings);
}

/** Sample widget contents for previews: the real ones only reach the dock window. */
export const SAMPLE_MEDIA: MediaInfo = { title: "Noite Feliz", artist: "Coral de Sorocaba", source: "Spotify", playing: true, thumbnail: null };
export const SAMPLE_SYSTEM: SystemStats = { cpu: 23, memory: 58, battery: 76, charging: false };

/** Sample apps for previews when nothing is open yet. */
export const SAMPLE_APPS: AppEntry[] = ["Explorador", "Navegador", "Código", "Música", "Chat", "Terminal"].map((name, i) => ({
  key: `sample-${i}`,
  name,
  exe: null,
  windows: i < 4 ? [{ hwnd: i, title: name }] : [],
  active: i === 1,
  pinned: i < 2,
}));

export function previewApps(count = 6): AppEntry[] {
  const real = hub.apps.slice(0, count);
  return real.length >= 3 ? real : SAMPLE_APPS.slice(0, count);
}
