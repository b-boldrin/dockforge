<script lang="ts">
  import Ic from "./Ic.svelte";
  import { editor } from "./editor.svelte";
  import { drag, startDrag } from "./dnd.svelte";
  import { KINDS, ZONE_LABEL, visualOrder, MAX_ELEMENTS, type Kind } from "./model";
  import type { Zone } from "../../lib/types";

  // The left panel: what can be put on the bar, as blocks of icons (the name is the tooltip),
  // and the layers (the bar and its elements).

  /** One drawing per kind: the tile, and the row in the layers. */
  const GLYPH: Record<Kind, string> = {
    icons: "M4 7h4v4H4zM10 7h4v4h-4zM16 7h4v4h-4zM3 16h18",
    clock: "M12 4a8 8 0 1 0 0 16 8 8 0 0 0 0-16zM12 8v4l3 2",
    audio: "M4 10v4h4l5 4V6L8 10zM16 9a4 4 0 0 1 0 6",
    power: "M12 3v8M7 6.5a7 7 0 1 0 10 0",
    system: "M5 12.5a10 10 0 0 1 14 0M8 15.5a6 6 0 0 1 8 0M12 19h.01",
  };
  const BLOCKS: Array<{ name: string; kinds: Kind[] }> = [
    { name: "Apps", kinds: ["icons"] },
    { name: "Sistema", kinds: ["clock", "audio", "power", "system"] },
  ];

  let query = $state("");
  const matches = (kind: Kind) => {
    const q = query.trim().toLowerCase();
    return !q || KINDS[kind].label.toLowerCase().includes(q);
  };
  const blocks = $derived(BLOCKS.map((b) => ({ ...b, kinds: b.kinds.filter(matches) })).filter((b) => b.kinds.length));

  const rows = $derived(visualOrder(editor.elements));
  const full = $derived(editor.elements.length >= MAX_ELEMENTS);

  function press(e: PointerEvent, kind: Kind) {
    if (e.button !== 0) return;
    startDrag(e, { label: KINDS[kind].label, onDrop: (drop) => drop && editor.add(kind, drop), onClick: () => editor.add(kind) });
  }
  function layerPress(e: PointerEvent, id: string, label: string) {
    if (e.button !== 0) return;
    editor.select(id);
    startDrag(e, { label, onDrop: (drop) => drop && editor.move(id, drop) });
  }
  // The row where the dragged thing would land: a line above it, or under the last row of a zone.
  const lineAbove = (id: string) => drag.on && drag.drop?.before === id;
  const lineBelow = (id: string, zone: string, i: number) =>
    drag.on && drag.drop?.before === null && drag.drop.zone === zone && rows.map((r) => r.zone).lastIndexOf(zone as Zone) === i;
</script>

<aside class="ms-left" aria-label="Elementos e camadas">
  <div class="ms-search">
    <Ic d="M11 4a7 7 0 1 0 0 14 7 7 0 0 0 0-14zM20 20l-4-4" size={14} />
    <input type="search" placeholder="Buscar elementos" aria-label="Buscar elementos" bind:value={query} />
  </div>

  <div class="ms-scroll">
    {#each blocks as block (block.name)}
      <section>
        <h2 class="ms-h">{block.name}</h2>
        <div class="ms-tiles">
          {#each block.kinds as kind (kind)}
            <button
              class="ms-tile"
              disabled={full}
              title={full ? `No máximo ${MAX_ELEMENTS} elementos` : `${KINDS[kind].label}: clique ou arraste para a barra`}
              aria-label={KINDS[kind].label}
              onpointerdown={(e) => press(e, kind)}
              onkeydown={(e) => (e.key === "Enter" || e.key === " ") && (e.preventDefault(), editor.add(kind))}
            >
              <span class="ms-tile-ic"><Ic d={GLYPH[kind]} size={20} /></span>
            </button>
          {/each}
        </div>
      </section>
    {/each}
    {#if blocks.length === 0}<p class="ms-hint">Nada com esse nome.</p>{/if}

    <section>
      <h2 class="ms-h">Camadas</h2>
      <div class="ms-layers" role="tree" aria-label="Camadas">
        <button class="ms-layer" class:on={editor.selected === "bar"} data-layer="bar" role="treeitem" aria-selected={editor.selected === "bar"} onclick={() => editor.select("bar")}>
          <Ic d="M3 14h18v6H3z" size={14} />
          <span class="ms-layer-n">Barra</span>
        </button>
        {#each rows as el, i (el.id)}
          <button
            class="ms-layer child"
            class:on={editor.selected === el.id}
            class:above={lineAbove(el.id)}
            class:below={lineBelow(el.id, el.zone, i)}
            data-layer={el.id}
            data-zone={el.zone}
            role="treeitem"
            aria-selected={editor.selected === el.id}
            onpointerdown={(e) => layerPress(e, el.id, KINDS[el.kind].name(el))}
            onkeydown={(e) => e.key === "Enter" && editor.select(el.id)}
          >
            <Ic d={GLYPH[el.kind]} size={14} />
            <span class="ms-layer-n">{KINDS[el.kind].name(el)}</span>
            <span class="ms-layer-z">{el.free ? "Livre · " : ""}{ZONE_LABEL[el.zone]}</span>
          </button>
        {/each}
        {#if rows.length === 0}
          <p class="ms-hint">Sem elementos: a barra usa o modelo clássico. Adicione elementos para montar a sua.</p>
        {/if}
      </div>
    </section>
  </div>
</aside>
