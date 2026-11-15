import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AppEntry, MediaInfo, SavedBar, Settings, SystemStats, ThemeSummary } from "../lib/types";
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

/** The part of the dock that a saved bar keeps: every setting but the favorites and the list itself. */
export function lookOf(s: Settings): Partial<Settings> {
  const look: Record<string, unknown> = { ...$state.snapshot(s) };
  delete look.pins;
  delete look.saved;
  delete look.version;
  return look as Partial<Settings>;
}

const sameLook = (a: unknown, b: unknown) => JSON.stringify(a) === JSON.stringify(b);

/** Puts a dock as it is now into "Minhas barras", at the top. A look already there is not repeated. */
export function keepBar(name: string) {
  if (!hub.settings) return;
  const look = lookOf(hub.settings);
  if (hub.settings.saved.some((b) => sameLook(b.look, look))) return;
  const id = `bar-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 6)}`;
  return patch({ saved: [{ id, name, look }, ...$state.snapshot(hub.settings.saved)] });
}

export function useTheme(id: string) {
  // A new theme starts clean: tweaks made on the old one do not carry over.
  const name = hub.themes.find((t) => t.id === id)?.name ?? id;
  return patch({ theme: id, accent: null, radius: null, opacity: null }).then(() => keepBar(name));
}

export function usePreset(preset: Preset) {
  return patch(preset.settings).then(() => keepBar(preset.name));
}

/** Applies a kept bar. Its look replaces the dock's look; the favorites stay as they are. */
export function applyBar(bar: SavedBar) {
  return patch(bar.look);
}

/** Saves what the bar editor changed: the name and the look, only when the user asks for it. */
export function saveBarEdit(id: string, name: string, look: Partial<Settings>) {
  if (!hub.settings) return;
  const clean = name.trim().slice(0, 40) || "Sem nome";
  return patch({ saved: $state.snapshot(hub.settings.saved).map((b) => (b.id === id ? { ...b, name: clean, look } : b)) });
}

export function removeBar(id: string) {
  if (!hub.settings) return;
  return patch({ saved: $state.snapshot(hub.settings.saved).filter((b) => b.id !== id) });
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
