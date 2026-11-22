<script lang="ts">
  import Ic, { type Part } from "./Ic.svelte";
  import { BLUETOOTH, POWER, SPEAKER, START, WIFI } from "../../lib/glyphs";
  import { editor } from "./editor.svelte";
  import { drag, startDrag } from "./dnd.svelte";
  import { KINDS, ZONE_LABEL, visualOrder, MAX_ELEMENTS, type Kind } from "./model";
  import type { Zone } from "../../lib/types";

  // The left panel: what can be put on the bar, as blocks of icons (the name is the tooltip),
  // and the layers (the bar and its elements).

  /** One drawing per kind: the tile, and the row in the layers. Settings shows the three
   *  status icons it holds (Wi-Fi, sound, Bluetooth) side by side, as on the bar. */
  const GLYPH: Record<Kind, Part[]> = {
    start: [{ d: START, fill: true }],
    icons: [{ d: "M4 7h4v4H4zM10 7h4v4h-4zM16 7h4v4h-4zM3 16h18" }],
    clock: [{ d: "M12 4a8 8 0 1 0 0 16 8 8 0 0 0 0-16zM12 8v4l3 2" }],
    audio: [{ d: SPEAKER }],
    power: [{ d: POWER }],
    system: [{ d: WIFI }, { d: SPEAKER }, { d: BLUETOOTH }],
  };
  const BLOCKS: Array<{ name: string; kinds: Kind[] }> = [
    { name: "Apps", kinds: ["start", "icons"] },
    { name: "Sistema", kinds: ["clock", "audio", "power"] },
    { name: "Geral", kinds: ["system"] },
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

<!-- A kind with several drawings shows them side by side, smaller. -->
{#snippet glyph(kind: Kind, size: number, small: number)}
  {#if GLYPH[kind].length > 1}
    <span class="ms-glyph-row">{#each GLYPH[kind] as part, i (i)}<Ic d={part.d} size={small} />{/each}</span>
  {:else}
    <Ic parts={GLYPH[kind]} {size} />
  {/if}
{/snippet}

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
              <span class="ms-tile-ic">{@render glyph(kind, 20, 13)}</span>
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
            {@render glyph(el.kind, 14, 10)}
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
