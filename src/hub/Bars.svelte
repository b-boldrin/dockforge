<script lang="ts">
  import ThemePreview from "./ThemePreview.svelte";
  import { hub, applyBar, keepBar, lookOf, removeBar, renameBar, updateBar, previewApps } from "./store.svelte.js";
  import type { SavedBar, Settings } from "../lib/types";

  const s = $derived(hub.settings!);
  const bars = $derived(s.saved);
  const apps = previewApps(6);

  /** The dock a card shows: the bar's own look over the current dock, at the size of a card. */
  function preview(bar: SavedBar): Settings {
    const merged = { ...s, ...bar.look } as Settings;
    return { ...merged, iconSize: Math.round((merged.iconSize ?? 44) * 0.5) };
  }

  function nameFor(): string {
    return `Barra ${bars.length + 1}`;
  }

  /** Names of the bars that match the dock as it is, so a card can say it is in use. */
  const inUse = (bar: SavedBar) => JSON.stringify(bar.look) === JSON.stringify(lookOf(s));
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Minhas barras</h1>
      <p class="lead">As docks que você usou na Loja ficam aqui sozinhas. Você também pode salvar a dock atual.</p>
    </div>
    <button class="btn" onclick={() => keepBar(nameFor())}>Salvar a dock atual</button>
  </div>

  {#if bars.length === 0}
    <p class="empty">Nenhuma barra salva ainda. Escolha uma na Loja ou salve a dock atual.</p>
  {:else}
    <div class="bars">
      {#each bars as bar (bar.id)}
        <article class="bar-card" class:on={inUse(bar)}>
          <div class="bar-pic">
            <ThemePreview theme={bar.look.theme ?? s.theme} apps={apps} settings={preview(bar)} />
          </div>
          <div class="bar-body">
            <input
              class="bar-name"
              aria-label="Nome da barra"
              value={bar.name}
              maxlength="40"
              onchange={(e) => renameBar(bar.id, (e.currentTarget as HTMLInputElement).value)}
            />
            <div class="bar-actions">
              {#if inUse(bar)}
                <span class="in-use">Em uso</span>
              {:else}
                <button class="link" onclick={() => applyBar(bar)}>Usar</button>
              {/if}
              <button class="link muted" onclick={() => updateBar(bar.id)}>Atualizar com a dock atual</button>
              <button class="link muted" onclick={() => removeBar(bar.id)}>Remover</button>
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
    height: 132px; border-radius: 12px; overflow: hidden;
    outline: 1px solid rgba(255, 255, 255, 0.08); outline-offset: -1px;
  }
  .bar-card.on .bar-pic { outline: 2px solid var(--accent); outline-offset: -2px; }
  .bar-name {
    background: transparent; border: 1px solid transparent; border-radius: 6px;
    padding: 4px 6px; margin-left: -6px; font-size: 14px; font-weight: 600; width: calc(100% + 12px);
  }
  .bar-name:hover, .bar-name:focus { border-color: rgba(255, 255, 255, 0.18); }
  .bar-actions { display: flex; align-items: center; gap: 14px; flex-wrap: wrap; font-size: 13px; }
  .bar-actions .in-use { font-size: 13px; }
  .empty { color: var(--text-3); font-size: 14px; margin: 24px 0; }
  .page-head .btn { flex: none; }
</style>
