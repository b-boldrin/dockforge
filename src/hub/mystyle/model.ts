// The MyStyle bar as plain data and the pure functions that change it. No Svelte in here: the
// editor (editor.svelte.ts) owns the state, this file only says what each change means.

import type { AudioElement, BarElement, ClockElement, IconsElement, PowerAction, PowerElement, SystemElement, SystemItem, Zone } from "../../lib/types";

export const ZONES: Zone[] = ["start", "center", "end"];
export const ZONE_LABEL: Record<Zone, string> = { start: "Início", center: "Meio", end: "Fim" };
export const SOURCE_LABEL: Record<IconsElement["source"], string> = { all: "Todos", pinned: "Favoritos", open: "Abertos" };
export const POWER_LABEL: Record<PowerAction, string> = { shutdown: "Desligar", restart: "Reiniciar", sleep: "Suspender", lock: "Bloquear", logout: "Sair da conta" };
export const POWER_ACTIONS = Object.keys(POWER_LABEL) as PowerAction[];
export const SYSTEM_LABEL: Record<SystemItem, string> = { wifi: "Wi-Fi", sound: "Som", bluetooth: "Bluetooth" };
export const SYSTEM_ITEMS = Object.keys(SYSTEM_LABEL) as SystemItem[];
/** Same cap as the Rust side (MAX_ELEMENTS in settings.rs). */
export const MAX_ELEMENTS = 16;

/** Where an element is dropped: into this zone, in front of `before` (null = at the end of it). */
export interface Drop {
  zone: Zone;
  before: string | null;
}

/** Everything the library knows about one kind of element. A new kind is one more entry. */
export interface KindInfo {
  label: string;
  /** Name shown in the layers list. */
  name: (el: BarElement) => string;
}
export type Kind = BarElement["kind"];
export const KINDS: Record<Kind, KindInfo> = {
  icons: { label: "Ícones", name: (el) => (el.kind === "icons" ? `Ícones · ${SOURCE_LABEL[el.source]}` : "Ícones") },
  clock: { label: "Data e hora", name: () => "Data e hora" },
  audio: { label: "Áudio", name: () => "Áudio" },
  power: { label: "Energia", name: () => "Energia" },
  system: { label: "Configurações", name: () => "Configurações" },
};

/** Any change to one element: every property of every kind, all optional. */
export type ElementPatch = Partial<
  Omit<IconsElement, "kind"> & Omit<ClockElement, "kind"> & Omit<AudioElement, "kind"> & Omit<PowerElement, "kind"> & Omit<SystemElement, "kind">
>;

/** The number that sets how big an element is, and its limits: the icon size of a field of
 *  icons, the height of a module. */
export const sizeOf = (el: BarElement): number => (el.kind === "icons" ? el.iconSize : el.size);
export const sizeRange = (el: BarElement): [number, number] => (el.kind === "icons" ? [24, 72] : [16, 48]);
export const sized = (el: BarElement, n: number): ElementPatch => {
  const [min, max] = sizeRange(el);
  const value = Math.round(clamp(n, min, max));
  return el.kind === "icons" ? { iconSize: value } : { size: value };
};

export const newId = () => "e" + Date.now().toString(36) + Math.random().toString(36).slice(2, 5);

export function newIcons(over: Partial<IconsElement> = {}): IconsElement {
  return {
    kind: "icons",
    id: newId(),
    zone: "center",
    free: false,
    offset: 0,
    fill: null,
    opacity: 100,
    radius: 12,
    padding: 0,
    border: null,
    source: "all",
    iconSize: 44,
    gap: 8,
    ...over,
  };
}

const base = (zone: Zone) => ({ id: newId(), zone, free: false, offset: 0, fill: null, opacity: 100, radius: 12, padding: 0, border: null });

export const newClock = (over: Partial<ClockElement> = {}): ClockElement => ({
  ...base("end"),
  kind: "clock",
  size: 32,
  color: null,
  show: "both",
  hour12: false,
  seconds: false,
  weekday: false,
  ...over,
});
export const newAudio = (over: Partial<AudioElement> = {}): AudioElement => ({ ...base("end"), kind: "audio", size: 28, color: null, percent: false, slider: false, ...over });
export const newPower = (over: Partial<PowerElement> = {}): PowerElement => ({ ...base("end"), kind: "power", size: 28, color: null, actions: [...POWER_ACTIONS], ...over });
export const newSystem = (over: Partial<SystemElement> = {}): SystemElement => ({ ...base("end"), kind: "system", size: 28, color: null, items: [...SYSTEM_ITEMS], opens: "quick", ...over });

/** A new element of a kind, with the kind's own defaults; `zone` is where it goes. */
export function newElement(kind: Kind, zone: Zone, iconSize: number): BarElement {
  switch (kind) {
    case "icons":
      return newIcons({ zone, iconSize });
    case "clock":
      return newClock({ zone });
    case "audio":
      return newAudio({ zone });
    case "power":
      return newPower({ zone });
    case "system":
      return newSystem({ zone });
  }
}

/** What a new bar starts with: the favorites, then what is open, both at the start. */
export function defaultElements(iconSize = 44): BarElement[] {
  return [newIcons({ source: "pinned", zone: "start", iconSize }), newIcons({ source: "open", zone: "start", iconSize })];
}

/** The elements as they sit on the bar from start to end: zone by zone, each in its own order. */
export const visualOrder = (els: BarElement[]): BarElement[] => ZONES.flatMap((z) => els.filter((e) => e.zone === z));

/** Moves one element into `drop.zone`, in front of `drop.before`, or last in the zone. */
export function moveElement(els: BarElement[], id: string, drop: Drop): BarElement[] {
  const el = els.find((e) => e.id === id);
  if (!el) return els;
  const rest = els.filter((e) => e.id !== id);
  let at = drop.before && drop.before !== id ? rest.findIndex((e) => e.id === drop.before) : -1;
  if (at < 0) {
    const last = rest.map((e) => e.zone).lastIndexOf(drop.zone);
    at = last < 0 ? rest.length : last + 1;
  }
  rest.splice(at, 0, { ...el, zone: drop.zone });
  return rest;
}

/** Puts an element in the flow, same as a move but for one that is not on the bar yet. */
export function insertElement(els: BarElement[], el: BarElement, drop: Drop): BarElement[] {
  return moveElement([...els, el], el.id, drop);
}

/** Swaps an element with its neighbour of the same kind of position in the zone: a flow element
 *  skips the free ones, which take no room (dir -1 = earlier, 1 = later). */
export function nudgeElement(els: BarElement[], id: string, dir: -1 | 1): BarElement[] {
  const el = els.find((e) => e.id === id);
  if (!el) return els;
  const peers = els.filter((e) => e.zone === el.zone && e.free === el.free);
  const next = peers[peers.indexOf(el) + dir];
  if (!next) return els;
  const out = [...els];
  const a = out.indexOf(el);
  const b = out.indexOf(next);
  [out[a], out[b]] = [out[b], out[a]];
  return out;
}

export const clamp = (n: number, min: number, max: number) => Math.min(max, Math.max(min, n));
