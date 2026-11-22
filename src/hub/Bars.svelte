<script lang="ts">
  import ThemePreview from "./ThemePreview.svelte";
  import { hub, applyBar, lookOf, previewApps, openMyStyle, sameLook } from "./store.svelte.js";
  import type { SavedBar, Settings } from "../lib/types";

  const s = $derived(hub.settings!);
  const bars = $derived(s.saved);
  const apps = previewApps(6);


  /** The card: the bar's look over the current dock, at the size of a card. */
  function card(bar: SavedBar): Settings {
    const merged = { ...s, elements: null, ...bar.look } as Settings;
    // Half size, elements included: each one carries its own icon size.
    const elements = merged.elements?.map((el) => (el.kind === "icons" ? { ...el, iconSize: Math.round(el.iconSize * 0.5), gap: Math.round(el.gap * 0.5) } : { ...el, size: Math.round(el.size * 0.5) })) ?? null;
    return { ...merged, elements, iconSize: Math.round((merged.iconSize ?? 44) * 0.5) };
  }
  const inUse = (bar: SavedBar) => sameLook(bar.look, lookOf(s));

</script>

<div class="page">
    <div class="page-head">
      <div>
        <h1>Minhas barras</h1>
        <p class="lead">As docks que você usou na Loja ficam aqui. Clique numa imagem para usar.</p>
      </div>
      <button class="btn pri" onclick={() => openMyStyle(null)}>Criar barra</button>
    </div>

    {#if bars.length === 0}
      <p class="empty">Nenhuma barra salva ainda. Escolha uma na Loja ou salve a dock atual.</p>
    {:else}
      <div class="bars">
        {#each bars as bar (bar.id)}
          <article class="bar-card" class:on={inUse(bar)}>
            <button class="bar-pic" aria-label="Usar a barra {bar.name}" onclick={() => applyBar(bar)}>
              <ThemePreview theme={bar.look.theme ?? s.theme} apps={apps} settings={card(bar)} />
            </button>
            <div class="bar-body">
              <div class="bar-name">{bar.name}</div>
              <div class="bar-actions">
                {#if inUse(bar)}<span class="in-use">Em uso</span>{/if}
                <button class="link muted" onclick={() => openMyStyle(bar.id)}>Configurar</button>
              </div>
            </div>
          </article>
        {/each}
      </div>
    {/if}
</div>

<style>
  .bars { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 20px 16px; margin-top: 20px; }
  .bar-card { display: flex; flex-direction: column; gap: 10px; }
  .bar-pic {
    height: 132px; padding: 0; border: 0; border-radius: 12px; overflow: hidden; cursor: pointer; display: block;
    outline: 1px solid var(--hover-2); outline-offset: -1px; background: var(--panel);
    transition: translate 240ms var(--ease), box-shadow 240ms var(--ease);
  }
  .bar-pic:hover { translate: 0 -2px; box-shadow: 0 12px 24px rgba(0, 0, 0, 0.45); }
  .bar-card.on .bar-pic { outline: 2px solid var(--accent); outline-offset: -2px; }
  .bar-name { font-size: 14px; font-weight: 600; }
  .bar-actions { display: flex; align-items: center; gap: 14px; font-size: 13px; }
  .empty { color: var(--text-3); font-size: 14px; margin: 24px 0; }
  .page-head .btn { flex: none; }
</style>
