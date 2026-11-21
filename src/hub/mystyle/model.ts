// The MyStyle bar as plain data and the pure functions that change it. No Svelte in here: the
// editor (editor.svelte.ts) owns the state, this file only says what each change means.

import type { BarElement, IconsElement, Zone } from "../../lib/types";

export const ZONES: Zone[] = ["start", "center", "end"];
export const ZONE_LABEL: Record<Zone, string> = { start: "Início", center: "Meio", end: "Fim" };
export const SOURCE_LABEL: Record<IconsElement["source"], string> = { all: "Todos", pinned: "Favoritos", open: "Abertos" };
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
export const KINDS: Record<BarElement["kind"], KindInfo> = {
  icons: { label: "Ícones", name: (el) => `Ícones · ${SOURCE_LABEL[el.source]}` },
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
