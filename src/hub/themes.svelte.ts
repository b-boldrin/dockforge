import { invoke } from "@tauri-apps/api/core";
import { createSheet, scoped, setSheet } from "../lib/themeSheet";
import type { ThemePayload } from "../lib/types";

// Every theme shown in the settings window gets its own stylesheet, limited with @scope to
// the previews that carry its id. That way the store shows each theme for real, and no theme
// can restyle the settings window itself.
const sheets = new Map<string, CSSStyleSheet>();

export function loadPreviewTheme(id: string) {
  if (sheets.has(id)) return;
  const sheet = createSheet();
  sheets.set(id, sheet);
  refreshPreviewTheme(id);
}

export function refreshPreviewTheme(id: string) {
  const sheet = sheets.get(id);
  if (!sheet) return;
  invoke<ThemePayload>("load_theme", { id })
    .then((t) => setSheet(sheet, scoped(t.css, `.df-preview[data-theme="${CSS.escape(id)}"]`), id))
    .catch((err) => console.error(`Preview of ${id} failed`, err));
}
