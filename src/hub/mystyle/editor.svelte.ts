// The MyStyle editor's state. One draft (Settings-shaped) plus what is selected; every edit
// goes through apply(), which pushes the previous snapshot on the undo stack. Undo and redo
// are just moves between the two stacks. The draft is private to this editor: nothing reaches
// the real dock until "Usar na minha dock", and nothing is kept until "Salvar".

import { hub, lookOf, showToast } from "../store.svelte.js";
import type { BarElement, IconsElement, Settings } from "../../lib/types";
import { MAX_ELEMENTS, clamp, defaultElements, insertElement, moveElement, newId, newIcons, nudgeElement, type Drop } from "./model";

const HISTORY = 100;
/** Edits with the same key within this time (a slider being dragged) are one undo step. */
const MERGE_MS = 700;

interface Snap {
  json: string;
  selected: string;
}
interface Parked extends Snap {
  name: string;
  undo: Snap[];
  redo: Snap[];
  baseline: string;
}

export type Tool = "select" | "hand";

class Editor {
  draft = $state<Settings | null>(null);
  name = $state("");
  /** An element id, or "bar". */
  selected = $state("bar");
  tool = $state<Tool>("select");
  zoom = $state(1);
  pan = $state({ x: 0, y: 0 });
  /** "new" or the id of the saved bar being edited. */
  key = $state<string | null>(null);

  private undoStack = $state.raw<Snap[]>([]);
  private redoStack = $state.raw<Snap[]>([]);
  private baseline = $state("");
  private lastKey = "";
  private lastAt = 0;
  /** Edits parked while another bar is open, so switching bars does not lose them. */
  private parked = new Map<string, Parked>();
  /** The canvas registers what the toolbar needs from it. */
  canvas: { zoomBy: (step: number) => void; fit: () => void } | null = null;

  canUndo = $derived(this.undoStack.length > 0);
  canRedo = $derived(this.redoStack.length > 0);
  elements = $derived<BarElement[]>(this.draft?.elements ?? []);
  current = $derived(this.elements.find((e) => e.id === this.selected) ?? null);
  dirty = $derived(this.draft != null && this.key != null && this.baseline !== this.fingerprint());

  private fingerprint() {
    return JSON.stringify([this.name, $state.snapshot(this.draft)]);
  }
  private snap(): Snap {
    return { json: JSON.stringify($state.snapshot(this.draft)), selected: this.selected };
  }
  private restore(s: Snap) {
    this.draft = JSON.parse(s.json) as Settings;
    this.selected = this.draft.elements?.some((e) => e.id === s.selected) ? s.selected : "bar";
  }

  // ---- which bar is open -------------------------------------------------------
  /** Opens a saved bar (or a new one). The bar already open is left as it is, edits included. */
  show(barId: string | null, fresh = false) {
    const bar = barId ? hub.settings?.saved.find((b) => b.id === barId) : null;
    const key = bar ? bar.id : "new";
    if (this.draft && this.key === key && !fresh) return;
    if (this.draft && this.key && !(fresh && this.key === key) && this.dirty) {
      this.parked.set(this.key, { ...this.snap(), name: this.name, undo: this.undoStack, redo: this.redoStack, baseline: this.baseline });
    }
    if (fresh) this.parked.delete(key);
    const back = this.parked.get(key);
    this.parked.delete(key);
    this.key = key;
    this.lastKey = "";
    if (back) {
      this.restore(back);
      this.name = back.name;
      this.undoStack = back.undo;
      this.redoStack = back.redo;
      this.baseline = back.baseline;
      return;
    }
    this.build(bar?.id ?? null);
  }

  private build(barId: string | null) {
    const s = hub.settings!;
    const bar = barId ? s.saved.find((b) => b.id === barId) : null;
    const look = JSON.parse(JSON.stringify(bar?.look ?? lookOf(s))) as Partial<Settings>;
    // The favorites and the saved list are not part of a look; the dock preview takes sample apps.
    const draft = { ...$state.snapshot(s), ...look, pins: [], saved: [], activeGroup: 0 } as Settings;
    // A new bar starts from the favorites and what is open, on an edge-to-edge bar, whatever the dock
    // has now. A kept bar from before MyStyle gets the same elements to edit, and only changes shape
    // when it has to (islands have no elements).
    if (!bar || !draft.elements?.length) {
      draft.elements = defaultElements(draft.iconSize);
      if (!bar) draft.layout = "bar";
      else if (draft.layout === "island") draft.layout = "float";
    }
    this.draft = draft;
    this.name = bar?.name ?? `Barra ${s.saved.length + 1}`;
    this.selected = "bar";
    this.undoStack = [];
    this.redoStack = [];
    this.baseline = this.fingerprint();
  }

  /** After Salvar: the draft is what is kept now. A new bar becomes the saved bar with this id. */
  saved(id: string) {
    this.parked.delete(this.key ?? "");
    this.key = id;
    this.baseline = this.fingerprint();
  }

  /** The draft as a saved bar's look. */
  look(): Partial<Settings> {
    return lookOf(this.draft!);
  }

  // ---- edits ---------------------------------------------------------------------
  /** Runs one change on the draft and remembers how it was before. `key` merges a run of edits. */
  apply(mutator: (draft: Settings) => void, key?: string) {
    if (!this.draft) return;
    const before = this.snap();
    mutator(this.draft);
    if (JSON.stringify($state.snapshot(this.draft)) === before.json) return;
    const now = Date.now();
    const merge = !!key && key === this.lastKey && (key.startsWith("drag:") || now - this.lastAt < MERGE_MS);
    if (!merge) {
      this.undoStack = [...this.undoStack.slice(-(HISTORY - 1)), before];
      this.redoStack = [];
    }
    this.lastKey = key ?? "";
    this.lastAt = now;
  }
  /** Ends a run of merged edits (a drag): the next edit is a new undo step. */
  settle() {
    this.lastKey = "";
  }
  set(changes: Partial<Settings>, key?: string) {
    this.apply((d) => Object.assign(d, changes), key);
  }

  undo() {
    const back = this.undoStack.at(-1);
    if (!back) return;
    this.redoStack = [...this.redoStack, this.snap()];
    this.undoStack = this.undoStack.slice(0, -1);
    this.restore(back);
    this.lastKey = "";
  }
  redo() {
    const next = this.redoStack.at(-1);
    if (!next) return;
    this.undoStack = [...this.undoStack, this.snap()];
    this.redoStack = this.redoStack.slice(0, -1);
    this.restore(next);
    this.lastKey = "";
  }

  select(id: string) {
    this.selected = id === "bar" || this.elements.some((e) => e.id === id) ? id : "bar";
  }

  // ---- elements ------------------------------------------------------------------
  /** Where something added by a click goes: after the selected element, or at the end of the middle. */
  private afterSelected(): Drop {
    const sel = this.current;
    if (!sel) return { zone: "center", before: null };
    const peers = this.elements.filter((e) => e.zone === sel.zone);
    return { zone: sel.zone, before: peers[peers.indexOf(sel) + 1]?.id ?? null };
  }

  /** Adds a field of icons: where it was dropped, or after the selected element. */
  addIcons(drop?: Drop) {
    if (this.elements.length >= MAX_ELEMENTS) return showToast(`No máximo ${MAX_ELEMENTS} elementos na barra`);
    const where = drop ?? this.afterSelected();
    const el = newIcons({ zone: where.zone, iconSize: this.current?.iconSize ?? this.draft!.iconSize });
    this.apply((d) => (d.elements = insertElement(d.elements ?? [], el, where)));
    this.selected = el.id;
  }

  remove(id = this.selected) {
    if (!this.elements.some((e) => e.id === id)) return;
    this.apply((d) => (d.elements = (d.elements ?? []).filter((e) => e.id !== id)));
    this.selected = "bar";
  }

  duplicate(id = this.selected) {
    const src = this.elements.find((e) => e.id === id);
    if (!src) return;
    if (this.elements.length >= MAX_ELEMENTS) return showToast(`No máximo ${MAX_ELEMENTS} elementos na barra`);
    const peers = this.elements.filter((e) => e.zone === src.zone);
    const copy: BarElement = { ...src, id: newId(), offset: src.free ? src.offset + 24 : src.offset };
    this.apply((d) => (d.elements = insertElement(d.elements ?? [], copy, { zone: src.zone, before: peers[peers.indexOf(src) + 1]?.id ?? null })));
    this.selected = copy.id;
  }

  move(id: string, drop: Drop) {
    this.apply((d) => (d.elements = moveElement(d.elements ?? [], id, drop)));
  }

  /** Earlier (-1) or later (1) in its zone. */
  nudge(dir: -1 | 1, id = this.selected) {
    this.apply((d) => (d.elements = nudgeElement(d.elements ?? [], id, dir)));
  }

  /** Changes one element. `key` merges a run of edits (a slider) into one undo step. */
  update(id: string, changes: Partial<IconsElement>, key?: string) {
    this.apply((d) => {
      const el = d.elements?.find((e) => e.id === id);
      if (el) Object.assign(el, changes);
    }, key ?? undefined);
  }

  setOffset(id: string, offset: number, key: string) {
    this.update(id, { offset: Math.round(clamp(offset, 0, 4000)) }, key);
  }
}

export const editor = new Editor();
