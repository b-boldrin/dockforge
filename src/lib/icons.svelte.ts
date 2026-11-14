import { invoke } from "@tauri-apps/api/core";

/** exe path -> data URL, or null when Windows gave no icon. Shared by every dock on the page. */
export const icons = $state<Record<string, string | null>>({});
const requested = new Set<string>();

export function requestIcon(exe: string | null) {
  if (!exe || requested.has(exe)) return;
  requested.add(exe);
  invoke<string | null>("get_icon", { exe })
    .then((url) => (icons[exe] = url))
    .catch(() => (icons[exe] = null));
}

/** A stable color per app name for apps whose icon could not be read. */
export function hue(name: string) {
  let h = 0;
  for (const ch of name) h = (h * 31 + ch.charCodeAt(0)) % 360;
  return h;
}

export function initial(name: string) {
  return (name.trim()[0] ?? "?").toUpperCase();
}
