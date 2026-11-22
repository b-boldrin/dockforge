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

export interface Pin {
  name: string;
  path: string;
  /** 0 = favorite (always in group 1). 1–3 = moved to group 2, 3 or 4. */
  group: number;
}

export type Position = "bottom" | "top" | "left" | "right";

/** The part of a MyStyle bar an element flows in. */
export type Zone = "start" | "center" | "end";

interface ElementBase {
  /** Short ascii, unique on the bar. */
  id: string;
  zone: Zone;
  /** true = out of the flow, kept at `offset` px from the start of its zone; neighbours don't move. */
  free: boolean;
  /** px, used only when `free`. */
  offset: number;
  /** Own background; null = none (the bar shows through). */
  fill: string | null;
  /** 0–100, of the fill. */
  opacity: number;
  /** px, 0–40. */
  radius: number;
  /** px, 0–24. */
  padding: number;
  /** Border color; null = none. */
  border: string | null;
}

/** A field of app icons: it grows with its icons and pushes its neighbours aside. */
export interface IconsElement extends ElementBase {
  kind: "icons";
  /** Which apps the field shows: favorites and open, only favorites, or only the open ones. */
  source: "all" | "pinned" | "open";
  /** px, 24–72. */
  iconSize: number;
  /** px between icons, 0–24. */
  gap: number;
}

/** What the small modules share: how big they are and the color of their glyphs and text. */
interface ModuleBase extends ElementBase {
  /** px, 16–48: the height of the module. */
  size: number;
  /** Glyph and text color; null = the theme's. */
  color: string | null;
}

/** The Start button. */
export interface StartElement extends ModuleBase {
  kind: "start";
}

/** Date and time. */
export interface ClockElement extends ModuleBase {
  kind: "clock";
  show: "time" | "date" | "both";
  hour12: boolean;
  seconds: boolean;
  weekday: boolean;
}

/** The system volume: click to mute, wheel to change it. */
export interface AudioElement extends ModuleBase {
  kind: "audio";
  /** The number next to the speaker. */
  percent: boolean;
  /** A slider (horizontal bars only). */
  slider: boolean;
}

export type PowerAction = "shutdown" | "restart" | "sleep" | "lock" | "logout";

/** A button that opens the power menu. */
export interface PowerElement extends ModuleBase {
  kind: "power";
  /** The entries of the menu, in order. */
  actions: PowerAction[];
}

export type SystemItem = "wifi" | "sound" | "bluetooth";

/** The Windows status icons as one button that opens the quick settings (or the Settings app). */
export interface SystemElement extends ModuleBase {
  kind: "system";
  items: SystemItem[];
  opens: "quick" | "settings";
}

/** One thing on a MyStyle bar. A new kind adds one variant here and in settings.rs. */
export type BarElement = IconsElement | StartElement | ClockElement | AudioElement | PowerElement | SystemElement;

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
  fill: string | null;
  radius: number | null;
  opacity: number | null;
  /** Space between the apps in pixels, or null for the theme's own. */
  gap: number | null;
  /** The mark under an open app: the theme's own, or one of four standard ones. */
  indicator: "theme" | "none" | "dot" | "dash" | "bar" | "glow";
  pins: Pin[];
  saved: SavedBar[];
  startWithWindows: boolean;
  hideTaskbar: boolean;
  hubTheme: "light" | "dark";
  language: "pt" | "en";
  /** Animations: on, off, or follow the Windows setting. */
  motion: "on" | "system" | "off";
  layout: "float" | "bar" | "island";
  align: "start" | "center" | "end";
  favoritesEnabled: boolean;
  groupsEnabled: boolean;
  activeGroup: number;
  groupNames: string[];
  /** The elements of a bar made in MyStyle. null = the classic dock. */
  elements: BarElement[] | null;
}

/** A dock kept in "Minhas barras": its name and every setting except the favorites. */
export interface SavedBar {
  id: string;
  name: string;
  look: Partial<Settings>;
}
