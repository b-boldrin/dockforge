<script lang="ts">
  import { onMount } from "svelte";
  import ThemePreview from "../ThemePreview.svelte";
  import Ic from "./Ic.svelte";
  import { previewApps } from "../store.svelte.js";
  import { editor } from "./editor.svelte";
  import { drag, dropLine, startDrag } from "./dnd.svelte";
  import { KINDS, clamp, sizeOf } from "./model";

  // The middle: a pan and zoom canvas with the bar on a wallpapered desktop. The bar is the real
  // Dock component rendered from the draft (not interactive), so what is seen here is what the
  // dock will be. Selecting and dragging read the positions straight from the rendered fields.

  const DESK_W = 960;
  const DESK_H = 540;
  const LABEL_H = 28;

  const view = $derived({ ...editor.draft!, activeGroup: 0 });
  const apps = $derived(previewApps(7));
  const side = $derived(view.position === "left" || view.position === "right");
  const SHAPE = { float: "Flutuante", bar: "Inteira", island: "Ilha" } as const;
  const EDGE = { bottom: "embaixo", top: "em cima", left: "à esquerda", right: "à direita" } as const;

  let host = $state<HTMLDivElement>();
  let stageEl = $state<HTMLDivElement>();
  let deskEl = $state<HTMLDivElement>();
  let panFrom = $state<{ x: number; y: number; px: number; py: number } | null>(null);
  let space = $state(false);

  // ---- zoom and pan ---------------------------------------------------------------
  function setZoom(next: number, cx?: number, cy?: number) {
    if (!host) return;
    const z = clamp(Math.round(next * 100) / 100, 0.25, 3);
    const r = host.getBoundingClientRect();
    const px = cx ?? r.width / 2;
    const py = cy ?? r.height / 2;
    const k = z / editor.zoom;
    editor.pan = { x: px - (px - editor.pan.x) * k, y: py - (py - editor.pan.y) * k };
    editor.zoom = z;
  }
  function fit() {
    if (!host) return;
    const r = host.getBoundingClientRect();
    const z = clamp(Math.min((r.width - 96) / DESK_W, (r.height - 150) / (DESK_H + LABEL_H)), 0.25, 1.5);
    const zoom = Math.round(z * 100) / 100;
    editor.zoom = zoom;
    editor.pan = { x: (r.width - DESK_W * zoom) / 2, y: (r.height - (DESK_H + LABEL_H) * zoom) / 2 - 14 };
  }
  function wheel(e: WheelEvent) {
    e.preventDefault();
    const r = host!.getBoundingClientRect();
    if (e.ctrlKey || e.metaKey) setZoom(editor.zoom * Math.exp(-e.deltaY * 0.0025), e.clientX - r.left, e.clientY - r.top);
    else editor.pan = { x: editor.pan.x - e.deltaX, y: editor.pan.y - e.deltaY };
  }

  // ---- pointer ----------------------------------------------------------------------
  /** How many screen pixels one pixel of the bar takes: the canvas zoom, and the preview's own shrink. */
  const scale = () => editor.zoom * (parseFloat(deskEl?.querySelector<HTMLElement>(".df-fit")?.style.zoom ?? "") || 1);

  function down(e: PointerEvent) {
    if ((e.target as HTMLElement).closest(".ms-dock-tools")) return;
    if (e.button === 1 || (e.button === 0 && (editor.tool === "hand" || space))) {
      e.preventDefault();
      panFrom = { x: e.clientX, y: e.clientY, px: editor.pan.x, py: editor.pan.y };
      host?.setPointerCapture(e.pointerId);
      return;
    }
    if (e.button !== 0) return;
    // A field selects itself and can be dragged; the bar and the empty canvas select the bar.
    const field = (e.target as HTMLElement).closest<HTMLElement>("[data-el]");
    const el = field && deskEl?.contains(field) ? editor.elements.find((x) => x.id === field.dataset.el) : null;
    if (!field || !el) return editor.select("bar");
    editor.select(el.id);
    const label = KINDS[el.kind].name(el);
    if (el.free) {
      const from = el.offset;
      const k = scale();
      startDrag(e, { label, onMove: (dx, dy) => editor.setOffset(el.id, from + (side ? dy : dx) / k, `drag:${el.id}`) });
    } else {
      startDrag(e, { label, dim: field, onDrop: (drop) => drop && editor.move(el.id, drop) });
    }
  }
  // The corner squares: drag to make the selected element (or, on the bar, every element)
  // bigger or smaller. Outward grows, inward shrinks; the size is the icon size or the module's height.
  function resizeStart(e: PointerEvent, sx: number, sy: number) {
    if (e.button !== 0) return;
    e.stopPropagation();
    e.preventDefault();
    const id = editor.selected;
    const from = Object.fromEntries(editor.elements.map((el) => [el.id, sizeOf(el)]));
    const k = scale();
    startDrag(e, { label: "Tamanho", onMove: (dx, dy) => editor.resize(id, (sx * dx + sy * dy) / 2 / k, from, `drag:size:${id}`) });
  }
  function move(e: PointerEvent) {
    if (panFrom) editor.pan = { x: panFrom.px + e.clientX - panFrom.x, y: panFrom.py + e.clientY - panFrom.y };
  }
  const release = () => (panFrom = null);

  const typing = (t: EventTarget | null) => t instanceof HTMLElement && !!t.closest("input, select, textarea, [contenteditable]");
  function keydown(e: KeyboardEvent) {
    if (e.code === "Space" && !typing(e.target) && !(e.target as HTMLElement).closest?.("button")) {
      e.preventDefault();
      space = true;
    }
  }
  const keyup = (e: KeyboardEvent) => e.code === "Space" && (space = false);

  // ---- selection box and drop line ----------------------------------------------
  // Measured every frame from the rendered bar, so it follows whatever moves it: an edit, the
  // preview shrinking a big bar to fit, a theme finishing loading.
  interface Box {
    x: number;
    y: number;
    w: number;
    h: number;
  }
  let box = $state<Box | null>(null);
  let line = $state<Box | null>(null);
  const same = (a: Box | null, b: Box | null) => (a && b ? Math.abs(a.x - b.x) + Math.abs(a.y - b.y) + Math.abs(a.w - b.w) + Math.abs(a.h - b.h) < 0.3 : a === b);

  onMount(() => {
    editor.canvas = { zoomBy: (step) => setZoom(editor.zoom + step), fit };
    host!.addEventListener("wheel", wheel, { passive: false });
    let frame = requestAnimationFrame(function tick() {
      frame = requestAnimationFrame(tick);
      if (!stageEl || !deskEl) return;
      const origin = stageEl.getBoundingClientRect();
      const z = editor.zoom;
      const local = (r: { left: number; top: number; width: number; height: number }): Box => ({ x: (r.left - origin.left) / z, y: (r.top - origin.top) / z, w: r.width / z, h: r.height / z });
      const target = editor.selected === "bar" ? deskEl.querySelector(".df-dock") : deskEl.querySelector(`[data-el="${CSS.escape(editor.selected)}"]`);
      const next = target ? local(target.getBoundingClientRect()) : null;
      if (!same(box, next)) box = next;
      const lineRect = drag.on && drag.drop ? dropLine(drag.drop) : null;
      const nextLine = lineRect ? local(lineRect) : null;
      if (!same(line, nextLine)) line = nextLine;
    });
    // The first fit waits for the layout to settle.
    const first = requestAnimationFrame(fit);
    return () => {
      cancelAnimationFrame(frame);
      cancelAnimationFrame(first);
      host?.removeEventListener("wheel", wheel);
      editor.canvas = null;
    };
  });

  const selectedName = $derived(editor.current ? KINDS[editor.current.kind].name(editor.current) : "Barra");
</script>

<svelte:window onkeydown={keydown} onkeyup={keyup} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="ms-canvas"
  class:hand={editor.tool === "hand" || space}
  class:grabbing={panFrom}
  style:background-size="{22 * editor.zoom}px {22 * editor.zoom}px"
  style:background-position="{editor.pan.x}px {editor.pan.y}px"
  bind:this={host}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={release}
  onpointercancel={release}
>
  <div class="ms-stage" bind:this={stageEl} style:transform="translate({editor.pan.x}px, {editor.pan.y}px) scale({editor.zoom})">
    <div class="ms-stage-label">
      <span>Área de trabalho</span>
      <span>{SHAPE[view.layout]} · {EDGE[view.position]}</span>
    </div>
    <div class="ms-desk" bind:this={deskEl} style:width="{DESK_W}px" style:height="{DESK_H}px">
      <ThemePreview theme={view.theme} {apps} settings={view} />
    </div>

    {#if box}
      <div class="ms-sel" style:left="{box.x}px" style:top="{box.y}px" style:width="{box.w}px" style:height="{box.h}px" style:--z={1 / editor.zoom}>
        {#if editor.elements.length}
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <i class="h tl" title="Arraste para redimensionar" onpointerdown={(e) => resizeStart(e, -1, -1)}></i>
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <i class="h tr" title="Arraste para redimensionar" onpointerdown={(e) => resizeStart(e, 1, -1)}></i>
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <i class="h bl" title="Arraste para redimensionar" onpointerdown={(e) => resizeStart(e, -1, 1)}></i>
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <i class="h br" title="Arraste para redimensionar" onpointerdown={(e) => resizeStart(e, 1, 1)}></i>
        {/if}
        <span class="ms-sel-l">{selectedName}</span>
      </div>
    {/if}
    {#if line}
      <div class="ms-drop" style:left="{line.x}px" style:top="{line.y}px" style:width="{line.w}px" style:height="{line.h}px"></div>
    {/if}
  </div>

  <div class="ms-dock-tools" role="toolbar" aria-label="Ferramentas">
    <button class:on={editor.tool === "select"} aria-pressed={editor.tool === "select"} title="Selecionar (V)" aria-label="Selecionar" onclick={() => (editor.tool = "select")}>
      <Ic d="M5 3l14 8-6 1.5L10 19z" size={18} />
    </button>
    <button class:on={editor.tool === "hand"} aria-pressed={editor.tool === "hand"} title="Mover a tela (H)" aria-label="Mover a tela" onclick={() => (editor.tool = "hand")}>
      <Ic d="M8 13V5.5a1.5 1.5 0 0 1 3 0V12M11 11V4a1.5 1.5 0 0 1 3 0v7M14 11V5.5a1.5 1.5 0 0 1 3 0V14a6 6 0 0 1-6 6h-1a6 6 0 0 1-5-2.7L3 14a1.6 1.6 0 0 1 2.6-1.8L8 15" size={18} />
    </button>
    <span class="sep" aria-hidden="true"></span>
    <button disabled={!editor.canUndo} title="Desfazer (Ctrl+Z)" aria-label="Desfazer" onclick={() => editor.undo()}><Ic d="M9 14 4 9l5-5M4 9h10.5a5.5 5.5 0 0 1 0 11H11" size={18} /></button>
    <button disabled={!editor.canRedo} title="Refazer (Ctrl+Shift+Z)" aria-label="Refazer" onclick={() => editor.redo()}><Ic d="M15 14l5-5-5-5M20 9H9.5a5.5 5.5 0 0 0 0 11H13" size={18} /></button>
    <span class="sep" aria-hidden="true"></span>
    <button disabled={!editor.current} title="Excluir (Delete)" aria-label="Excluir" onclick={() => editor.remove()}><Ic d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" size={18} /></button>
    <button disabled={!editor.current} title="Duplicar (Ctrl+D)" aria-label="Duplicar" onclick={() => editor.duplicate()}><Ic d="M8 8h12v12H8zM4 16V4h12" size={18} /></button>
  </div>
</div>
