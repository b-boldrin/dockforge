<script lang="ts">
  import Ic from "./Ic.svelte";
  import { editor } from "./editor.svelte";
  import { drag, startDrag } from "./dnd.svelte";
  import { KINDS, ZONE_LABEL, visualOrder, MAX_ELEMENTS } from "./model";
  import type { Zone } from "../../lib/types";

  // The left panel: what can be put on the bar, and the layers (the bar and its elements).

  interface Tile {
    id: string;
    label: string;
    hint: string;
    icon: string;
    ready: boolean;
  }
  const TILES: Tile[] = [
    { id: "icons", label: KINDS.icons.label, hint: "Favoritos e apps abertos", icon: "M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z", ready: true },
    { id: "clock", label: "Data e hora", hint: "Relógio e calendário", icon: "M12 4a8 8 0 1 0 0 16 8 8 0 0 0 0-16zM12 8v4l3 2", ready: false },
    { id: "audio", label: "Áudio", hint: "Volume e saída de som", icon: "M4 10v4h4l5 4V6L8 10zM16 9a4 4 0 0 1 0 6", ready: false },
    { id: "power", label: "Energia", hint: "Desligar, reiniciar, suspender", icon: "M12 3v8M7 6.5a7 7 0 1 0 10 0", ready: false },
  ];

  let query = $state("");
  let open = $state(true);
  const shown = $derived(
    TILES.filter((t) => {
      const q = query.trim().toLowerCase();
      return !q || `${t.label} ${t.hint}`.toLowerCase().includes(q);
    }),
  );

  const rows = $derived(visualOrder(editor.elements));
  const full = $derived(editor.elements.length >= MAX_ELEMENTS);

  function press(e: PointerEvent, tile: Tile) {
    if (!tile.ready || e.button !== 0) return;
    startDrag(e, { label: tile.label, onDrop: (drop) => drop && editor.addIcons(drop), onClick: () => editor.addIcons() });
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
    <section>
      <button class="ms-cat" aria-expanded={open} onclick={() => (open = !open)}>
        <span class="ms-chev" class:open><Ic d="M9 6l6 6-6 6" size={13} /></span>
        Barra de tarefas
      </button>
      {#if open}
        <div class="ms-tiles">
          {#each shown as tile (tile.id)}
            <button
              class="ms-tile"
              class:soon={!tile.ready}
              disabled={!tile.ready || full}
              title={tile.ready ? (full ? `No máximo ${MAX_ELEMENTS} elementos` : "Clique ou arraste para a barra") : "Em breve"}
              onpointerdown={(e) => press(e, tile)}
              onkeydown={(e) => (e.key === "Enter" || e.key === " ") && tile.ready && (e.preventDefault(), editor.addIcons())}
            >
              <span class="ms-tile-ic"><Ic d={tile.icon} size={20} /></span>
              <span class="ms-tile-t">{tile.label}</span>
              <span class="ms-tile-s">{tile.hint}</span>
              {#if !tile.ready}<span class="ms-badge">Em breve</span>{/if}
            </button>
          {/each}
        </div>
        {#if shown.length === 0}<p class="ms-hint">Nada com esse nome.</p>{/if}
      {/if}
    </section>

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
            <Ic d="M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z" size={14} />
            <span class="ms-layer-n">{KINDS[el.kind].name(el)}</span>
            <span class="ms-layer-z">{el.free ? "Livre · " : ""}{ZONE_LABEL[el.zone]}</span>
          </button>
        {/each}
        {#if rows.length === 0}
          <p class="ms-hint">Sem elementos: a barra usa o modelo clássico. Adicione Ícones para montar a sua.</p>
        {/if}
      </div>
    </section>
  </div>
</aside>
