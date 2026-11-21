<script lang="ts">
  import { onMount } from "svelte";
  import ThemePreview from "./ThemePreview.svelte";
  import { hub, nav, patch, lookOf, addBar, saveBarEdit, removeBar, showToast, previewApps } from "./store.svelte.js";
  import type { Settings } from "../lib/types";

  // The bar creator: a canvas with a dotted grid, the dock in the middle, tools on the left,
  // layers beside them and the properties on the right. Nothing reaches the real dock until
  // "Usar na minha dock", and nothing is kept until "Salvar".

  const s = $derived(hub.settings!);
  const editing = $derived(nav.editBar ? (s.saved.find((b) => b.id === nav.editBar) ?? null) : null);

  let name = $state("");
  let draft = $state<Partial<Settings>>({});
  onMount(() => {
    const bar = nav.editBar ? hub.settings?.saved.find((b) => b.id === nav.editBar) : null;
    name = bar?.name ?? `Barra ${(hub.settings?.saved.length ?? 0) + 1}`;
    draft = JSON.parse(JSON.stringify(bar?.look ?? lookOf(hub.settings!)));
  });
  const set = (changes: Partial<Settings>) => (draft = { ...draft, ...changes });
  const view = $derived({ ...s, ...draft, activeGroup: 0 } as Settings);

  // ---- canvas: tool, zoom and panning -------------------------------------------
  type Tool = "select" | "hand";
  let tool = $state<Tool>("select");
  let zoom = $state(1);
  let pan = $state({ x: 0, y: 0 });
  let dragging = $state<{ x: number; y: number; px: number; py: number } | null>(null);
  const zoomBy = (step: number) => (zoom = Math.min(2, Math.max(0.4, Math.round((zoom + step) * 10) / 10)));
  const fitView = () => {
    zoom = 1;
    pan = { x: 0, y: 0 };
  };
  function down(e: PointerEvent) {
    if (tool !== "hand" && e.button !== 1) return;
    dragging = { x: e.clientX, y: e.clientY, px: pan.x, py: pan.y };
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
  }
  function move(e: PointerEvent) {
    if (!dragging) return;
    pan = { x: dragging.px + e.clientX - dragging.x, y: dragging.py + e.clientY - dragging.y };
  }
  const up = () => (dragging = null);
  function wheel(e: WheelEvent) {
    if (!e.ctrlKey) return;
    e.preventDefault();
    zoomBy(e.deltaY < 0 ? 0.1 : -0.1);
  }
  function key(e: KeyboardEvent) {
    const t = e.target as HTMLElement;
    if (t.closest("input, select, textarea")) return;
    if (e.key === "v" || e.key === "V") tool = "select";
    else if (e.key === "h" || e.key === "H") tool = "hand";
    else if (e.key === "+" || e.key === "=") zoomBy(0.1);
    else if (e.key === "-") zoomBy(-0.1);
    else if (e.key === "0") fitView();
  }

  // ---- layers ---------------------------------------------------------------------
  type Layer = "bar" | "apps" | "marks";
  let layer = $state<Layer>("bar");
  // ---- inspector ------------------------------------------------------------------
  let tab = $state<"design" | "anim" | "behav">("design");
  const fills: Array<[string | null, string]> = [
    [null, "Do tema"],
    ["#ffffff", "Branco neve"],
    ["#c8102e", "Vermelho natal"],
    ["#26262b", "Grafite"],
    ["#f2e6d3", "Champanhe"],
    ["#1f3b5c", "Azul noite"],
  ];
  const accents: Array<[string | null, string]> = [
    [null, "Do tema"],
    ["#c8102e", "Vermelho"],
    ["#e0a23a", "Dourado"],
    ["#2f6bff", "Azul"],
    ["#ffffff", "Branco"],
  ];
  const radii: Array<[number | null, string]> = [[null, "Tema"], [0, "0"], [8, "8"], [16, "16"], [24, "24"], [32, "32"]];
  const shapes: Array<[Settings["layout"], string, string]> = [
    ["float", "Flutuante", "M8 9h18v6H8z"],
    ["bar", "Inteira", "M1 9h32v6H1z"],
    ["island", "Ilha", "M11 9h12v6H11z"],
  ];
  const positions: Array<[Settings["position"], string]> = [["bottom", "Embaixo"], ["top", "Em cima"], ["left", "Esquerda"], ["right", "Direita"]];
  const marks: Array<[Settings["indicator"], string]> = [["theme", "Do tema"], ["none", "Nenhuma"], ["dot", "Ponto"], ["dash", "Traço"], ["bar", "Barra"], ["glow", "Brilho"]];
  const hovers: Array<[number, string]> = [[1, "Nenhum"], [1.15, "Leve"], [1.3, "Ampliar"], [1.5, "Forte"]];
  const num = (e: Event) => Number((e.currentTarget as HTMLInputElement).value);

  // ---- save -----------------------------------------------------------------------
  function back() {
    nav.tab = nav.from;
    nav.editBar = null;
  }
  async function save() {
    const look = $state.snapshot(draft) as Partial<Settings>;
    if (editing) {
      await saveBarEdit(editing.id, name, look);
      showToast("Barra salva");
    } else {
      const id = await addBar(name, look);
      if (id) nav.editBar = id;
    }
  }
  async function use() {
    await patch($state.snapshot(draft) as Partial<Settings>);
    showToast("Aplicada na sua dock");
  }
  async function remove() {
    if (!editing) return;
    await removeBar(editing.id);
    back();
  }

  const apps = previewApps(7);
</script>

<svelte:window onkeydown={key} />

<div class="creator">
  <header class="c-top">
    <button class="c-back" onclick={back} aria-label="Voltar">
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 6l-6 6 6 6" /></svg>
    </button>
    <nav class="c-crumbs" aria-label="Caminho">
      <span class="muted">{nav.from === "bars" ? "Minhas barras" : "Loja"}</span>
      <span class="muted" aria-hidden="true">/</span>
      <input class="c-name" aria-label="Nome da barra" maxlength="40" bind:value={name} />
    </nav>
    <div class="c-grow"></div>
    <div class="c-zoom" role="group" aria-label="Zoom">
      <button aria-label="Afastar" onclick={() => zoomBy(-0.1)}>–</button>
      <button class="c-zoom-val" title="Ajustar (0)" onclick={fitView}>{Math.round(zoom * 100)}%</button>
      <button aria-label="Aproximar" onclick={() => zoomBy(0.1)}>+</button>
    </div>
    {#if editing}
      <button class="btn danger" onclick={remove}>Deletar</button>
    {/if}
    <button class="btn" onclick={use}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 5l12 7-12 7z" /></svg>
      Usar na minha dock
    </button>
    <button class="btn pri" onclick={save}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 4h11l3 3v13H5z" /><path d="M8 4v5h7V4" /><path d="M8 20v-6h8v6" /></svg>
      Salvar
    </button>
  </header>

  <div class="c-body">
    <aside class="c-layers" aria-label="Camadas">
      <h2>Camadas</h2>
      <button class="c-layer" class:on={layer === "bar"} onclick={() => (layer = "bar")}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 14h18v6H3z" /></svg>Barra
      </button>
      <button class="c-layer d1" class:on={layer === "apps"} onclick={() => (layer = "apps")}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 4h6v6H4zM14 4h6v6h-6zM4 14h6v6H4zM14 14h6v6h-6z" /></svg>Apps fixados
      </button>
      <button class="c-layer d2" class:on={layer === "marks"} onclick={() => (layer = "marks")}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 18h8" /></svg>Marca de app aberto
      </button>
    </aside>

    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <main
      class="c-canvas"
      class:hand={tool === "hand"}
      class:grabbing={dragging}
      onpointerdown={down}
      onpointermove={move}
      onpointerup={up}
      onpointercancel={up}
      onwheel={wheel}
    >
      <div class="c-rail" role="toolbar" aria-label="Ferramentas">
        <button aria-label="Selecionar" title="Selecionar (V)" aria-pressed={tool === "select"} class:on={tool === "select"} onclick={() => (tool = "select")}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 3l14 8-6 1.5L10 19z" /></svg>
        </button>
        <button aria-label="Mover a tela" title="Mover a tela (H)" aria-pressed={tool === "hand"} class:on={tool === "hand"} onclick={() => (tool = "hand")}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 13V5.5a1.5 1.5 0 0 1 3 0V12M11 11V4a1.5 1.5 0 0 1 3 0v7M14 11V5.5a1.5 1.5 0 0 1 3 0V14a6 6 0 0 1-6 6h-1a6 6 0 0 1-5-2.7L3 14a1.6 1.6 0 0 1 2.6-1.8L8 15" /></svg>
        </button>
        <span class="c-rail-sep" aria-hidden="true"></span>
        <button aria-label="Aproximar" title="Aproximar (+)" onclick={() => zoomBy(0.1)}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="M20 20l-3.5-3.5M8 11h6M11 8v6" /></svg>
        </button>
        <button aria-label="Afastar" title="Afastar (-)" onclick={() => zoomBy(-0.1)}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="M20 20l-3.5-3.5M8 11h6" /></svg>
        </button>
        <button aria-label="Ajustar à tela" title="Ajustar à tela (0)" onclick={fitView}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" /></svg>
        </button>
      </div>

      <div class="c-stage" style:transform="translate({pan.x}px, {pan.y}px) scale({zoom})">
        <div class="c-stage-label">
          <span>Área de trabalho</span>
          <span>{shapes.find((x) => x[0] === view.layout)?.[1]} · {positions.find((x) => x[0] === view.position)?.[1]}</span>
        </div>
        <div class="c-desk" data-sel={layer}>
          <ThemePreview theme={view.theme} {apps} settings={view} />
        </div>
      </div>

    </main>

    <aside class="c-inspector" aria-label="Propriedades">
      <div class="c-tabs" role="tablist" aria-label="Seções">
        <button role="tab" aria-selected={tab === "design"} class:on={tab === "design"} onclick={() => (tab = "design")}>Design</button>
        <button role="tab" aria-selected={tab === "anim"} class:on={tab === "anim"} onclick={() => (tab = "anim")}>Animação</button>
        <button role="tab" aria-selected={tab === "behav"} class:on={tab === "behav"} onclick={() => (tab = "behav")}>Comportamento</button>
      </div>

      <div class="c-props">
        {#if tab === "design"}
          <section>
            <h3>Tema base</h3>
            <select aria-label="Tema base" value={view.theme} onchange={(e) => set({ theme: (e.currentTarget as HTMLSelectElement).value })}>
              {#each hub.themes as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
            </select>
          </section>

          <section class:hl={layer === "bar"}>
            <h3>Formato</h3>
            <div class="c-seg c3">
              {#each shapes as [id, label, path] (id)}
                <button class:on={view.layout === id} aria-pressed={view.layout === id} onclick={() => set(id === "bar" ? { layout: id, autoHide: false, reserveSpace: true } : { layout: id })}>
                  <svg viewBox="0 0 34 24" class="shape" aria-hidden="true"><path d={path} /></svg>{label}
                </button>
              {/each}
            </div>
            <div class="c-seg c4">
              {#each positions as [id, label] (id)}
                <button class:on={view.position === id} aria-pressed={view.position === id} onclick={() => set({ position: id })}>{label}</button>
              {/each}
            </div>
          </section>

          <section class:hl={layer === "bar"}>
            <h3>Preenchimento</h3>
            <div class="c-swatches">
              {#each fills as [hex, label] (label)}
                <button class="c-swatch" class:theme={hex === null} style:background={hex ?? undefined} aria-label={label} title={label} aria-pressed={view.fill === hex} class:on={view.fill === hex} onclick={() => set({ fill: hex })}></button>
              {/each}
              <label class="c-swatch c-custom" title="Outra cor">
                <input type="color" aria-label="Outra cor" value={view.fill ?? "#ffffff"} oninput={(e) => set({ fill: (e.currentTarget as HTMLInputElement).value })} />
              </label>
            </div>
            <label class="c-row"><span>Opacidade</span>
              <input type="range" min="30" max="100" value={view.opacity ?? 100} oninput={(e) => set({ opacity: num(e) })} />
              <span class="c-val">{view.opacity ?? 100}%</span>
            </label>
          </section>

          <section class:hl={layer === "bar"}>
            <h3>Cantos</h3>
            <div class="c-seg c6">
              {#each radii as [r, label] (label)}
                <button class:on={view.radius === r} aria-pressed={view.radius === r} onclick={() => set({ radius: r })}>{label}</button>
              {/each}
            </div>
          </section>

          <section class:hl={layer === "apps"}>
            <h3>Apps</h3>
            <label class="c-row"><span>Espaço de cada app</span>
              <input type="range" min="32" max="72" step="4" value={view.iconSize} oninput={(e) => set({ iconSize: num(e) })} />
              <span class="c-val">{view.iconSize}</span>
            </label>
            <label class="c-row"><span>Tamanho do ícone</span>
              <input type="range" min="0.5" max="1.2" step="0.05" value={view.appScale} oninput={(e) => set({ appScale: num(e) })} />
              <span class="c-val">{Math.round(view.appScale * 100)}%</span>
            </label>
            <label class="c-row"><span>Espaço entre apps</span>
              <input type="range" min="0" max="24" value={view.gap ?? 8} oninput={(e) => set({ gap: num(e) })} />
              <span class="c-val">{view.gap ?? "Tema"}</span>
            </label>
          </section>

          <section class:hl={layer === "marks"}>
            <h3>Marca de app aberto</h3>
            <div class="c-seg c3">
              {#each marks as [id, label] (id)}
                <button class:on={view.indicator === id} aria-pressed={view.indicator === id} onclick={() => set({ indicator: id })}>{label}</button>
              {/each}
            </div>
            <div class="c-swatches">
              {#each accents as [hex, label] (label)}
                <button class="c-swatch small" class:theme={hex === null} style:background={hex ?? undefined} aria-label="Cor da marca: {label}" title={label} aria-pressed={view.accent === hex} class:on={view.accent === hex} onclick={() => set({ accent: hex })}></button>
              {/each}
            </div>
          </section>
        {:else if tab === "anim"}
          <section>
            <h3>Ao passar o mouse no app</h3>
            <div class="c-seg c4">
              {#each hovers as [v, label] (label)}
                <button class:on={Math.abs(view.magnify - v) < 0.01} aria-pressed={Math.abs(view.magnify - v) < 0.01} onclick={() => set({ magnify: v })}>{label}</button>
              {/each}
            </div>
            <p class="c-hint">Passe o mouse nos apps da barra ao lado para ver.</p>
          </section>
          <section>
            <h3>Animações da dock</h3>
            <div class="c-seg c3">
              <button class:on={view.motion === "on"} onclick={() => set({ motion: "on" })}>Ligadas</button>
              <button class:on={view.motion === "system"} onclick={() => set({ motion: "system" })}>Do Windows</button>
              <button class:on={view.motion === "off"} onclick={() => set({ motion: "off" })}>Desligadas</button>
            </div>
            <p class="c-hint">A marca dos apps abertos.</p>
          </section>
        {:else}
          <section>
            <h3>Comportamento</h3>
            <div class="c-toggle">
              <span><span>Ocultar sozinha</span><span class="muted">Some e volta ao encostar o mouse na borda.</span></span>
              <button class="tg" aria-label="Ocultar sozinha" aria-pressed={view.autoHide} onclick={() => set({ autoHide: !view.autoHide })}></button>
            </div>
            {#if view.autoHide}
              <div class="c-toggle">
                <span><span>Só com janela maximizada</span><span class="muted">Fica à vista sobre janelas soltas.</span></span>
                <button class="tg" aria-label="Só com janela maximizada" aria-pressed={view.hideOnlyMaximized} onclick={() => set({ hideOnlyMaximized: !view.hideOnlyMaximized })}></button>
              </div>
            {/if}
            <div class="c-toggle">
              <span><span>Empurrar janelas</span><span class="muted">Janelas maximizadas terminam na barra.</span></span>
              <button class="tg" aria-label="Empurrar janelas" aria-pressed={view.reserveSpace} onclick={() => set({ reserveSpace: !view.reserveSpace })}></button>
            </div>
          </section>
        {/if}
      </div>
    </aside>
  </div>
</div>

<style>
  .creator { height: 100vh; display: flex; flex-direction: column; background: var(--bg); }
  svg { width: 18px; height: 18px; fill: none; stroke: currentColor; stroke-width: 1.7; stroke-linecap: round; stroke-linejoin: round; flex: none; }
  button:not(.btn) { border: 0; background: none; padding: 0; cursor: pointer; }
  .muted { color: var(--text-3); }

  .c-top { height: 52px; flex: none; display: flex; align-items: center; gap: 10px; padding: 0 16px; border-bottom: 1px solid var(--line); }
  .c-top > * { flex: none; }
  .c-top > .c-grow { flex: 1; min-width: 0; }
  .c-top .btn { white-space: nowrap; margin: 0; }
  .c-back { width: 34px; height: 34px; border-radius: 8px; display: grid; place-items: center; color: var(--text-2); }
  .c-back:hover { background: var(--hover-2); color: var(--text); }
  .c-crumbs { display: flex; align-items: center; gap: 8px; font-size: 13px; }
  .c-name { width: 200px; height: 30px; padding: 0 8px; border-radius: 6px; border: 1px solid transparent; background: transparent; font-weight: 600; }
  .c-name:hover, .c-name:focus { border-color: var(--line-2); outline: none; background: var(--panel); }
  .c-grow { flex: 1; }
  .c-zoom { display: flex; align-items: center; padding: 3px; border-radius: 9px; background: var(--raised); }
  .c-zoom button { height: 28px; min-width: 28px; border-radius: 6px; font-size: 15px; }
  .c-zoom button:hover { background: var(--hover-2); }
  .c-zoom .c-zoom-val { font-size: 12px; min-width: 46px; font-variant-numeric: tabular-nums; }
  .c-top .btn { display: inline-flex; align-items: center; gap: 7px; }
  .c-top .btn svg { width: 15px; height: 15px; }
  .btn.danger { color: var(--coral); }

  .c-body { flex: 1; min-height: 0; display: flex; }

  .c-layers { width: 236px; flex: none; padding: 14px 10px; border-right: 1px solid var(--line); display: flex; flex-direction: column; gap: 2px; overflow: auto; }
  .c-layers h2 { font-size: 13px; font-weight: 600; padding: 0 8px 8px; }
  .c-layer { display: flex; align-items: center; gap: 8px; height: 32px; padding: 0 8px; border-radius: 7px; font-size: 13px; text-align: left; width: 100%; color: var(--text); }
  .c-layer svg { width: 15px; height: 15px; color: var(--text-3); }
  .c-layer:hover:not(:disabled) { background: var(--hover); }
  .c-layer.on { background: color-mix(in srgb, var(--accent) 12%, transparent); color: var(--accent); }
  .c-layer.on svg { color: var(--accent); }
  .c-layer:disabled { cursor: default; }
  .d1 { padding-left: 16px; }
  .c-layer.d1 { padding-left: 24px; }
  .c-layer.d2 { padding-left: 40px; }

  .c-canvas {
    flex: 1; min-width: 0; position: relative; overflow: hidden;
    display: flex; align-items: center; justify-content: center;
    background-color: var(--panel);
    background-image: radial-gradient(var(--grid-dot) 1.2px, transparent 1.2px);
    background-size: 22px 22px;
  }
  .c-canvas.hand { cursor: grab; }
  .c-canvas.grabbing { cursor: grabbing; }
  .c-rail {
    position: absolute; left: 14px; top: 50%; translate: 0 -50%; z-index: 2;
    display: flex; flex-direction: column; gap: 2px; padding: 6px; border-radius: 14px;
    background: var(--bg); box-shadow: 0 0 0 1px var(--line-2), 0 8px 24px rgba(0, 0, 0, 0.12);
  }
  .c-rail button { width: 38px; height: 38px; border-radius: 10px; display: grid; place-items: center; color: var(--text); }
  .c-rail button:hover { background: var(--hover-2); }
  .c-rail button.on { background: var(--accent); color: var(--on-accent); }
  .c-rail-sep { height: 1px; margin: 4px 6px; background: var(--line-2); }

  .c-stage { width: min(860px, calc(100% - 140px)); display: flex; flex-direction: column; gap: 8px; transform-origin: center; transition: transform 160ms var(--ease); }
  .c-canvas.grabbing .c-stage { transition: none; }
  .c-stage-label { display: flex; justify-content: space-between; font-size: 12px; color: var(--text-3); }
  .c-desk { height: 460px; border-radius: 14px; overflow: hidden; box-shadow: 0 0 0 1px var(--line-2), 0 16px 40px rgba(0, 0, 0, 0.12); }
  .c-desk :global(.df-preview) { height: 100%; }
  /* The selected layer is outlined, as in a drawing program. */
  .c-desk[data-sel="bar"] :global(.df-dock) { outline: 1.5px solid #2f6bff; outline-offset: 5px; }
  .c-desk[data-sel="apps"] :global(.df-apps) { outline: 1.5px solid #2f6bff; outline-offset: 3px; border-radius: 8px; }
  .c-desk[data-sel="marks"] :global(.df-indicator) { outline: 1.5px solid #2f6bff; outline-offset: 2px; }

  .c-inspector { width: 320px; flex: none; border-left: 1px solid var(--line); display: flex; flex-direction: column; min-height: 0; }
  .c-tabs { display: flex; gap: 4px; padding: 10px 12px; border-bottom: 1px solid var(--line); }
  .c-tabs button { flex: 1; height: 32px; border-radius: 8px; color: var(--text-2); font-size: 13px; font-weight: 500; }
  .c-tabs button.on { background: var(--raised); color: var(--text); }
  .c-props { padding: 16px; display: flex; flex-direction: column; gap: 22px; overflow: auto; }
  .c-props section { display: flex; flex-direction: column; gap: 10px; border-radius: 10px; transition: background 200ms var(--ease); }
  .c-props section.hl { background: color-mix(in srgb, #2f6bff 6%, transparent); box-shadow: 0 0 0 8px color-mix(in srgb, #2f6bff 6%, transparent); }
  .c-props h3 { font-size: 12px; font-weight: 600; color: var(--text-2); }
  .c-props select { height: 34px; border-radius: 8px; border: 1px solid var(--line-2); background: var(--bg); padding: 0 8px; }
  .c-seg { display: grid; gap: 4px; padding: 3px; border-radius: 10px; background: var(--raised); }
  .c-seg.c3 { grid-template-columns: repeat(3, minmax(0, 1fr)); }
  .c-seg.c4 { grid-template-columns: repeat(4, minmax(0, 1fr)); }
  .c-seg.c6 { grid-template-columns: repeat(6, minmax(0, 1fr)); }
  .c-seg button { min-height: 30px; border-radius: 7px; font-size: 12px; color: var(--text-2); display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 4px; padding: 4px 2px; }
  .c-seg button.on { background: var(--bg); color: var(--text); box-shadow: 0 1px 3px rgba(0, 0, 0, 0.14); }
  .c-seg .shape { width: 34px; height: 24px; fill: currentColor; stroke: none; opacity: 0.75; }
  .c-swatches { display: flex; flex-wrap: wrap; gap: 8px; }
  .c-swatch { width: 40px; height: 40px; border-radius: 11px; box-shadow: inset 0 0 0 1px var(--line-2); position: relative; overflow: hidden; }
  .c-swatch.small { width: 32px; height: 32px; border-radius: 9px; }
  .c-swatch.theme { background: conic-gradient(var(--raised) 25%, var(--raised-2) 0 50%, var(--raised) 0 75%, var(--raised-2) 0) 0 0 / 12px 12px; }
  .c-swatch.on { box-shadow: 0 0 0 2px var(--bg), 0 0 0 4px var(--accent); }
  .c-custom { display: grid; place-items: center; cursor: pointer; background: conic-gradient(#ff5a5a, #ffd25a, #5aff8c, #5ad2ff, #8c5aff, #ff5ad2, #ff5a5a); }
  .c-custom input { opacity: 0; position: absolute; inset: 0; width: 100%; height: 100%; cursor: pointer; }
  .c-row { display: flex; align-items: center; gap: 10px; font-size: 13px; }
  .c-row > span:first-child { width: 118px; flex: none; }
  .c-row input[type="range"] { flex: 1; min-width: 0; accent-color: var(--accent); }
  .c-val { width: 40px; text-align: right; font-variant-numeric: tabular-nums; color: var(--text-2); }
  .c-hint { margin: 0; font-size: 12px; color: var(--text-3); }
  .c-toggle { display: flex; align-items: center; justify-content: space-between; gap: 12px; font-size: 13px; }
  .c-toggle > span { display: flex; flex-direction: column; gap: 2px; }
  .c-toggle .muted { font-size: 12px; }

  @media (max-width: 1100px) {
    .c-layers { display: none; }
  }
</style>
