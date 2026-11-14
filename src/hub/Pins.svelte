<script lang="ts">
  import { hub, importFavorites, patch } from "./store.svelte.js";
  import { hue, icons, initial, requestIcon } from "../lib/icons.svelte.js";
  import type { Pin } from "../lib/types";

  const s = $derived(hub.settings!);
  let editing = $state(0);
  const group = $derived(s.groupsEnabled ? editing : 0);

  /** Apps of the group being edited, with their place in the full list. */
  const rows = $derived(s.pins.map((pin, index) => ({ pin, index })).filter(({ pin }) => pin.group === group));

  let adding = $state(false);
  const candidates = $derived(
    hub.apps.filter((a) => a.exe && !rows.some(({ pin }) => pin.path.toLowerCase() === a.exe!.toLowerCase())),
  );

  $effect(() => {
    for (const { pin } of rows) requestIcon(pin.path);
    for (const a of candidates) requestIcon(a.exe);
  });

  function save(pins: Pin[]) {
    patch({ pins });
  }

  function move(row: number, delta: number) {
    const other = rows[row + delta];
    if (!other) return;
    const pins = [...s.pins];
    const a = rows[row].index;
    [pins[a], pins[other.index]] = [pins[other.index], pins[a]];
    save(pins);
  }

  function remove(index: number) {
    save(s.pins.filter((_, i) => i !== index));
  }

  /** An app has one place only, so adding it here takes it from wherever it was. */
  function add(name: string, path: string) {
    const rest = s.pins.filter((p) => p.path.toLowerCase() !== path.toLowerCase());
    save([...rest, { name, path, group }]);
    adding = false;
  }

  function sendTo(index: number, target: number) {
    save(s.pins.map((p, i) => (i === index ? { ...p, group: target } : p)));
  }

  // ---- import from the Windows taskbar ------------------------------------
  let importing = $state(false);
  let imported = $state<number | null>(null);
  async function runImport() {
    importing = true;
    imported = await importFavorites();
    importing = false;
  }
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Favoritos e grupos</h1>
      <p class="lead">
        Os favoritos ficam na dock mesmo fechados, sempre no grupo 1. Os outros grupos começam vazios e recebem os
        apps que você mandar para eles.
      </p>
    </div>
    <button class="btn pri" aria-expanded={adding} onclick={() => (adding = !adding)}>Adicionar app</button>
  </div>

  <div class="group-tabs" role="group" aria-label="Grupo">
    <button class="flt" class:on={group === 0} aria-pressed={group === 0} onclick={() => (editing = 0)}>
      Favoritos {#if s.groupsEnabled}<span class="kbd">Alt+1</span>{/if}
    </button>
    {#if s.groupsEnabled}
      {#each s.groupNames.slice(1) as name, i (i)}
        <button class="flt" class:on={editing === i + 1} aria-pressed={editing === i + 1} onclick={() => (editing = i + 1)}>
          {name} <span class="kbd">Alt+{i + 2}</span>
        </button>
      {/each}
    {/if}
  </div>

  {#if group === 0}
    <div class="import">
      <div>
        <div class="lbl">Trazer da barra de tarefas do Windows</div>
        <p class="sub">
          {#if imported == null}
            Copia para cá os programas fixados na barra de tarefas. Nada é removido de lá.
          {:else if imported === 0}
            Nenhum programa novo. Apps da Microsoft Store e o Explorador de Arquivos não têm como ser importados.
          {:else}
            {imported} {imported === 1 ? "programa adicionado" : "programas adicionados"} aos favoritos.
          {/if}
        </p>
      </div>
      <button class="btn" disabled={importing} onclick={runImport}>{importing ? "Importando…" : "Importar favoritos"}</button>
    </div>
    {#if !s.favoritesEnabled}
      <p class="notice">
        Os favoritos estão desligados, então a dock mostra só o que está aberto.
        <button class="link" onclick={() => patch({ favoritesEnabled: true })}>Ligar favoritos</button>
      </p>
    {/if}
  {/if}

  {#if adding}
    <div class="picker">
      <p class="sub">Escolha entre os programas abertos agora:</p>
      {#each candidates as app (app.key)}
        {@const icon = app.exe ? icons[app.exe] : null}
        <div class="pin-row">
          {#if icon}<img class="pin-icon" src={icon} alt="" />{:else}<span class="pin-icon fallback" style:--h={hue(app.name)}>{initial(app.name)}</span>{/if}
          <div class="pin-text"><div class="pin-name">{app.name}</div><div class="sub ellipsis">{app.exe}</div></div>
          <button class="link" onclick={() => add(app.name, app.exe!)}>Adicionar</button>
        </div>
      {:else}
        <p class="empty">Nenhum programa aberto que ainda não esteja aqui. Abra o programa e volte.</p>
      {/each}
    </div>
  {/if}

  <div class="pin-list">
    {#each rows as { pin, index }, row (pin.path)}
      {@const icon = icons[pin.path]}
      <div class="pin-row">
        {#if icon}<img class="pin-icon" src={icon} alt="" />{:else}<span class="pin-icon fallback" style:--h={hue(pin.name)}>{initial(pin.name)}</span>{/if}
        <div class="pin-text"><div class="pin-name">{pin.name}</div><div class="sub ellipsis">{pin.path}</div></div>
        {#if s.groupsEnabled}
          <select aria-label="Grupo de {pin.name}" value={pin.group} onchange={(e) => sendTo(index, Number(e.currentTarget.value))}>
            <option value={0}>Favoritos</option>
            {#each s.groupNames.slice(1) as name, i (i)}
              <option value={i + 1}>{name}</option>
            {/each}
          </select>
        {/if}
        <button class="ico" aria-label="Mover {pin.name} para cima" disabled={row === 0} onclick={() => move(row, -1)}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M6 15l6-6 6 6"></path></svg>
        </button>
        <button class="ico" aria-label="Mover {pin.name} para baixo" disabled={row === rows.length - 1} onclick={() => move(row, 1)}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M6 9l6 6 6-6"></path></svg>
        </button>
        <button class="link muted" onclick={() => remove(index)}>Tirar</button>
      </div>
    {:else}
      <p class="empty">
        {#if group === 0}
          Nenhum favorito ainda. Importe da barra de tarefas ou clique em Adicionar app.
        {:else}
          Grupo vazio. Com o app em foco, aperte Alt+Shift+{group + 1}, ou arraste o ícone dele na dock até a bolinha do grupo.
        {/if}
      </p>
    {/each}
  </div>
</div>
