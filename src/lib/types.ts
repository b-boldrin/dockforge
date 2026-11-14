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

export interface Pin {
  name: string;
  path: string;
  group: number;
}

export type Position = "bottom" | "top" | "left" | "right";

export interface Settings {
  theme: string;
  position: Position;
  iconSize: number;
  magnify: number;
  autoHide: boolean;
  hideDelay: number;
  showMode: "both" | "pinned" | "open";
  clickAction: "focus-min" | "cycle" | "focus";
  separator: boolean;
  accent: string | null;
  radius: number | null;
  opacity: number | null;
  pins: Pin[];
  startWithWindows: boolean;
  language: "pt" | "en";
  layout: "float" | "bar";
  groupsEnabled: boolean;
  activeGroup: number;
  groupNames: string[];
  widgetMedia: boolean;
  widgetClock: boolean;
}
