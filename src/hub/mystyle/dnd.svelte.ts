// Dragging in MyStyle: a tile from the library, a field on the canvas, a row in the layers list.
// Done with pointer events on purpose: the webview's own drag and drop is taken over by the
// window on Windows. One session at a time; the canvas and the layers only draw `drag`.

import type { Zone } from "../../lib/types";
import { editor } from "./editor.svelte";
import { ZONES, type Drop } from "./model";

export const drag = $state({
  on: false,
  label: "",
  x: 0,
  y: 0,
  /** Where it would land if released now. */
  drop: null as Drop | null,
});

type Span = (rect: DOMRect) => [number, number];
const spanOf = (): Span => {
  const side = editor.draft?.position === "left" || editor.draft?.position === "right";
  return (r) => (side ? [r.top, r.bottom] : [r.left, r.right]);
};

const dockEl = () => document.querySelector<HTMLElement>(".ms-desk .df-dock");
const flowFields = (zone: HTMLElement | null) => (zone ? [...zone.querySelectorAll<HTMLElement>(":scope > .df-field:not([data-free])")] : []);

/** The drop under a point of the canvas: which zone, and in front of which field. */
function canvasDrop(x: number, y: number): Drop | null {
  const dock = dockEl();
  if (!dock) return null;
  const side = editor.draft?.position === "left" || editor.draft?.position === "right";
  const span = spanOf();
  const box = dock.getBoundingClientRect();
  // Only near the bar: released out on the wallpaper, nothing happens.
  const room = 40;
  if (x < box.left - room || x > box.right + room || y < box.top - room || y > box.bottom + room) return null;
  const p = side ? y : x;
  const zones = new Map(ZONES.map((z) => [z, dock.querySelector<HTMLElement>(`.df-zone[data-zone="${z}"]`)] as const));
  let zone: Zone | null = null;
  for (const [z, el] of zones) {
    if (!el) continue;
    const [a, b] = span(el.getBoundingClientRect());
    if (b - a > 1 && p >= a && p <= b) {
      zone = z;
      break;
    }
  }
  if (!zone) {
    // Over an empty part of the bar: left third, middle or right third.
    const [a, b] = span(box);
    const mid = (a + b) / 2;
    zone = Math.abs(p - mid) < (b - a) * 0.12 ? "center" : p < mid ? "start" : "end";
  }
  let before: string | null = null;
  for (const field of flowFields(zones.get(zone) ?? null)) {
    const [a, b] = span(field.getBoundingClientRect());
    if (p < (a + b) / 2) {
      before = field.dataset.el ?? null;
      break;
    }
  }
  return { zone, before };
}

/** The drop under a point of the layers list: above or below the row it is over. */
function layerDrop(row: HTMLElement, y: number): Drop | null {
  const id = row.dataset.layer;
  const zone = row.dataset.zone as Zone | undefined;
  if (!id || !zone) return null;
  const r = row.getBoundingClientRect();
  if (y < r.top + r.height / 2) return { zone, before: id };
  const next = row.nextElementSibling as HTMLElement | null;
  return { zone, before: next?.dataset.zone === zone ? (next.dataset.layer ?? null) : null };
}

function dropAt(x: number, y: number): Drop | null {
  const row = document.elementFromPoint(x, y)?.closest<HTMLElement>("[data-layer]");
  return row ? layerDrop(row, y) : canvasDrop(x, y);
}

/** Client rectangle of the insertion line for a drop, so the canvas can draw it. */
export function dropLine(drop: Drop): { left: number; top: number; width: number; height: number } | null {
  const dock = dockEl();
  if (!dock) return null;
  const side = editor.draft?.position === "left" || editor.draft?.position === "right";
  const span = spanOf();
  const box = dock.getBoundingClientRect();
  const zone = dock.querySelector<HTMLElement>(`.df-zone[data-zone="${drop.zone}"]`);
  const flows = flowFields(zone);
  let m: number;
  const target = drop.before ? dock.querySelector<HTMLElement>(`[data-el="${CSS.escape(drop.before)}"]`) : null;
  if (target) {
    const i = flows.indexOf(target);
    const start = span(target.getBoundingClientRect())[0];
    m = i > 0 ? (span(flows[i - 1].getBoundingClientRect())[1] + start) / 2 : start - 3;
  } else if (flows.length) {
    m = span(flows[flows.length - 1].getBoundingClientRect())[1] + 3;
  } else if (zone) {
    const [a, b] = span(zone.getBoundingClientRect());
    m = drop.zone === "end" ? b - 3 : drop.zone === "center" ? (a + b) / 2 : a + 3;
  } else {
    const [a, b] = span(box);
    m = drop.zone === "end" ? b - 8 : drop.zone === "center" ? (a + b) / 2 : a + 8;
  }
  return side
    ? { left: box.left + 3, top: m - 1.5, width: box.width - 6, height: 3 }
    : { left: m - 1.5, top: box.top + 3, width: 3, height: box.height - 6 };
}

export interface DragSpec {
  label: string;
  /** A free element: how far the pointer has moved since the press, in screen pixels. */
  onMove?: (dx: number, dy: number) => void;
  /** Released after a drag: where it would land (null = nowhere). */
  onDrop?: (drop: Drop | null) => void;
  /** Released without having moved: it was a click. */
  onClick?: () => void;
  /** The thing being dragged on the canvas, dimmed while it is. */
  dim?: HTMLElement | null;
}

/** Starts from a pointerdown. Moving 4px or more makes it a drag. */
export function startDrag(start: PointerEvent, spec: DragSpec) {
  const x0 = start.clientX;
  const y0 = start.clientY;
  let began = false;

  const end = (e: PointerEvent | null, cancelled: boolean) => {
    window.removeEventListener("pointermove", move);
    window.removeEventListener("pointerup", up);
    window.removeEventListener("pointercancel", cancel);
    window.removeEventListener("keydown", key, true);
    if (began) {
      document.body.style.cursor = "";
      if (spec.dim) spec.dim.style.opacity = "";
      if (!cancelled && spec.onDrop && e) spec.onDrop(dropAt(e.clientX, e.clientY));
    } else if (!cancelled) {
      spec.onClick?.();
    }
    editor.settle();
    drag.on = false;
    drag.drop = null;
  };
  const move = (e: PointerEvent) => {
    if (!began) {
      if (Math.hypot(e.clientX - x0, e.clientY - y0) < 4) return;
      began = true;
      document.body.style.cursor = "grabbing";
      if (spec.dim) spec.dim.style.opacity = "0.35";
      drag.on = !!spec.onDrop;
      drag.label = spec.label;
    }
    drag.x = e.clientX;
    drag.y = e.clientY;
    if (spec.onMove) spec.onMove(e.clientX - x0, e.clientY - y0);
    if (spec.onDrop) drag.drop = dropAt(e.clientX, e.clientY);
  };
  const up = (e: PointerEvent) => end(e, false);
  const cancel = () => end(null, true);
  const key = (e: KeyboardEvent) => {
    if (e.key !== "Escape") return;
    e.stopPropagation();
    end(null, true);
  };
  window.addEventListener("pointermove", move);
  window.addEventListener("pointerup", up);
  window.addEventListener("pointercancel", cancel);
  window.addEventListener("keydown", key, true);
}
