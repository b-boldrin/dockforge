<script lang="ts">
  import ThemePreview from "./ThemePreview.svelte";
  import { hub, applyBar, keepBar, lookOf, removeBar, saveBarEdit, previewApps } from "./store.svelte.js";
  import type { SavedBar, Settings } from "../lib/types";

  const s = $derived(hub.settings!);
  const bars = $derived(s.saved);
  const apps = previewApps(6);

  /** Integrations a bar can show. A bar keeps at most MAX_INTEGRATIONS of them on. */
  const MAX_INTEGRATIONS = 4;
  const integrations: Array<[keyof Settings, string, string]> = [
    ["widgetMedia", "Tocando agora", "Música ou vídeo em reprodução."],
    ["widgetSystem", "Sistema", "Processador, memória e bateria."],
    ["widgetClock", "Relógio", "Hora e data."],
    ["widgetPomodoro", "Pomodoro", "25 minutos de foco e 5 de pausa."],
    ["widgetWorldClock", "Hora em outra cidade", "A hora de uma cidade escolhida."],
  ];
  const onCount = (look: Partial<Settings>) => integrations.filter(([key]) => look[key] === true).length;

  /** The card: the bar's look over the current dock, at the size of a card. */
  function card(bar: SavedBar): Settings {
    const merged = { ...s, ...bar.look } as Settings;
    return { ...merged, iconSize: Math.round((merged.iconSize ?? 44) * 0.5) };
  }
  const inUse = (bar: SavedBar) => JSON.stringify(bar.look) === JSON.stringify(lookOf(s));

  // ---- the editor: changes stay here until "Salvar" -----------------------------
  let editingId = $state<string | null>(null);
  let draftName = $state("");
  let draftLook = $state<Partial<Settings>>({});

  function edit(bar: SavedBar) {
    editingId = bar.id;
    draftName = bar.name;
    draftLook = JSON.parse(JSON.stringify(bar.look));
  }
  function closeEditor() {
    editingId = null;
  }
  function toggleIntegration(key: keyof Settings) {
    const on = draftLook[key] === true;
    if (!on && onCount(draftLook) >= MAX_INTEGRATIONS) return;
    draftLook = { ...draftLook, [key]: !on };
  }
  async function save() {
    if (!editingId) return;
    await saveBarEdit(editingId, draftName, draftLook);
    closeEditor();
  }
  async function remove() {
    if (!editingId) return;
    await removeBar(editingId);
    closeEditor();
  }
  const editing = $derived(bars.find((b) => b.id === editingId) ?? null);
  const draftSettings = $derived({ ...s, ...draftLook, iconSize: Math.round(((draftLook.iconSize ?? s.iconSize) as number) * 0.7) } as Settings);
</script>

<div class="page">
  {#if editing}
    <!-- Buttons come first, above the content. Nothing here is saved until "Salvar". -->
    <div class="editor-bar">
      <button class="link muted" onclick={closeEditor}>Voltar</button>
      <div class="editor-actions">
        <button class="btn danger" onclick={remove}>Deletar</button>
        <button class="btn pri" onclick={save}>Salvar</button>
      </div>
    </div>

    <h1>Configurar barra</h1>
    <div class="editor-pic">
      <ThemePreview theme={draftLook.theme ?? s.theme} apps={apps} settings={draftSettings} />
    </div>

    <div class="box">
      <div class="srow">
        <label class="lbl" for="bar-name">Nome</label>
        <input id="bar-name" class="name-input" value={draftName} maxlength="40" oninput={(e) => (draftName = (e.currentTarget as HTMLInputElement).value)} />
      </div>
    </div>

    <h2>Integrações desta barra</h2>
    <p class="lead">Escolha até {MAX_INTEGRATIONS}. Hoje {onCount(draftLook)} de {MAX_INTEGRATIONS}.</p>
    <div class="box">
      {#each integrations as [key, label, about] (key)}
        {@const on = draftLook[key] === true}
        {@const blocked = !on && onCount(draftLook) >= MAX_INTEGRATIONS}
        <div class="srow" class:is-blocked={blocked}>
          <div><div class="lbl">{label}</div><div class="sub">{about}</div></div>
          <button class="tg" aria-label={label} aria-pressed={on} disabled={blocked} onclick={() => toggleIntegration(key)}></button>
        </div>
      {/each}
    </div>
  {:else}
    <div class="page-head">
      <div>
        <h1>Minhas barras</h1>
        <p class="lead">As docks que você usou na Loja ficam aqui. Clique numa imagem para usar.</p>
      </div>
      <button class="btn" onclick={() => keepBar(`Barra ${bars.length + 1}`)}>Salvar a dock atual</button>
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
                <button class="link muted" onclick={() => edit(bar)}>Configurar</button>
              </div>
            </div>
          </article>
        {/each}
      </div>
    {/if}
  {/if}
</div>

<style>
  .editor-bar { display: flex; align-items: center; justify-content: space-between; gap: 16px; margin-bottom: 20px; }
  .editor-actions { display: flex; gap: 8px; }
  .editor-pic { height: 200px; border-radius: 14px; overflow: hidden; margin-top: 16px; outline: 1px solid rgba(255, 255, 255, 0.08); outline-offset: -1px; }
  .name-input {
    background: var(--raised-2); border: 1px solid var(--line-2); border-radius: 6px; padding: 6px 10px; font-size: 13px; width: 240px;
  }
  .box .srow.is-blocked .lbl, .box .srow.is-blocked .sub { color: var(--text-3); }
  .bars { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 20px 16px; margin-top: 20px; }
  .bar-card { display: flex; flex-direction: column; gap: 10px; }
  .bar-pic {
    height: 132px; padding: 0; border: 0; border-radius: 12px; overflow: hidden; cursor: pointer; display: block;
    outline: 1px solid rgba(255, 255, 255, 0.08); outline-offset: -1px; background: var(--panel);
    transition: translate 240ms var(--ease), box-shadow 240ms var(--ease);
  }
  .bar-pic:hover { translate: 0 -2px; box-shadow: 0 12px 24px rgba(0, 0, 0, 0.45); }
  .bar-card.on .bar-pic { outline: 2px solid var(--accent); outline-offset: -2px; }
  .bar-name { font-size: 14px; font-weight: 600; }
  .bar-actions { display: flex; align-items: center; gap: 14px; font-size: 13px; }
  .empty { color: var(--text-3); font-size: 14px; margin: 24px 0; }
  .page-head .btn { flex: none; }
</style>
