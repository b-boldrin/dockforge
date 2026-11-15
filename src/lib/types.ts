// Shapes shared with the Rust side (src-tauri/src/model.rs and settings.rs). Keep both in sync.

export interface WindowInfo {
  hwnd: number;
  title: string;
}

export interface AppEntry {
  /** Stable key: the executable path, or a per-window key for hosted apps. */
  key: string;
  name: string;
  exe: string | null;
  /** Windows in z-order, the most recently used first. Empty for a pinned app that is closed. */
  windows: WindowInfo[];
  /** True if one of this app's windows was in front before the dock was clicked. */
  active: boolean;
  pinned: boolean;
}

export interface ThemePayload {
  id: string;
  name: string;
  css: string;
}

export interface ThemeSummary {
  id: string;
  name: string;
  author: string;
  tags: string[];
  built_in: boolean;
}

export interface MediaInfo {
  title: string;
  artist: string;
  source: string;
  playing: boolean;
  thumbnail: string | null;
}

export interface SystemStats {
  cpu: number;
  memory: number;
  /** null on a computer without a battery. */
  battery: number | null;
  charging: boolean;
}

export interface Pin {
  name: string;
  path: string;
  /** 0 = favorite (always in group 1). 1–3 = moved to group 2, 3 or 4. */
  group: number;
}

export type Position = "bottom" | "top" | "left" | "right";

export interface Settings {
  version: number;
  theme: string;
  position: Position;
  iconSize: number;
  magnify: number;
  appScale: number;
  autoHide: boolean;
  hideOnlyMaximized: boolean;
  reserveSpace: boolean;
  hideDelay: number;
  showMode: "both" | "pinned" | "open";
  clickAction: "focus-min" | "cycle" | "focus";
  separator: boolean;
  /** Each open app's name next to its icon (horizontal docks). */
  labels: boolean;
  accent: string | null;
  radius: number | null;
  opacity: number | null;
  /** Space between the apps in pixels, or null for the theme's own. */
  gap: number | null;
  /** The mark under an open app: the theme's own, or one of four standard ones. */
  indicator: "theme" | "none" | "dot" | "dash" | "bar" | "glow";
  pins: Pin[];
  startWithWindows: boolean;
  hideTaskbar: boolean;
  language: "pt" | "en";
  /** Animations: on, off, or follow the Windows setting. */
  motion: "on" | "system" | "off";
  layout: "float" | "bar" | "island";
  align: "start" | "center" | "end";
  favoritesEnabled: boolean;
  groupsEnabled: boolean;
  activeGroup: number;
  groupNames: string[];
  widgetMedia: boolean;
  /** Kinds of players the "now playing" widget ignores. */
  mediaBlocked: Array<"spotify" | "browsers" | "others">;
  widgetClock: boolean;
  widgetSystem: boolean;
}
