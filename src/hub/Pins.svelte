<script lang="ts">
  import { hub, patch } from "./store.svelte.js";
  import { hue, icons, initial, requestIcon } from "../lib/icons.svelte.js";
  import type { Pin } from "../lib/types";

  const s = $derived(hub.settings!);
  let editing = $state(0);
  const group = $derived(s.groupsEnabled ? editing : 0);

  /** Pins of the group being edited, with their place in the full list. */
  const rows = $derived(
    s.pins
      .map((pin, index) => ({ pin, index }))
      .filter(({ pin }) => !s.groupsEnabled || pin.group === group),
  );

  let adding = $state(false);
  const candidates = $derived(
    hub.apps.filter(
      (a) => a.exe && !rows.some(({ pin }) => pin.path.toLowerCase() === a.exe!.toLowerCase()),
    ),
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

  function add(name: string, path: string) {
    save([...s.pins, { name, path, group }]);
    adding = false;
  }
</script>

<div class="page">
  <div class="page-head">
    <div>
      <h1>Apps fixados</h1>
      <p class="lead">Ficam na dock mesmo fechados. Também dá para fixar com o botão direito num app aberto na dock.</p>
    </div>
    <button class="btn pri" aria-expanded={adding} onclick={() => (adding = !adding)}>Adicionar app</button>
  </div>

  {#if s.groupsEnabled}
    <div class="group-tabs" role="group" aria-label="Grupo">
      {#each s.groupNames as name, i (i)}
        <button class="flt" class:on={editing === i} aria-pressed={editing === i} onclick={() => (editing = i)}>
          {name} <span class="kbd">Alt+{i + 1}</span>
        </button>
      {/each}
    </div>
  {/if}

  {#if adding}
    <div class="picker">
      <p class="sub">Escolha entre os programas abertos agora:</p>
      {#each candidates as app (app.key)}
        {@const icon = app.exe ? icons[app.exe] : null}
        <div class="pin-row">
          {#if icon}<img class="pin-icon" src={icon} alt="" />{:else}<span class="pin-icon fallback" style:--h={hue(app.name)}>{initial(app.name)}</span>{/if}
          <div class="pin-text"><div class="pin-name">{app.name}</div><div class="sub ellipsis">{app.exe}</div></div>
          <button class="link" onclick={() => add(app.name, app.exe!)}>Fixar</button>
        </div>
      {:else}
        <p class="empty">Nenhum programa aberto que ainda não esteja fixado. Abra o programa e volte aqui.</p>
      {/each}
    </div>
  {/if}

  <div class="pin-list">
    {#each rows as { pin, index }, row (index)}
      {@const icon = icons[pin.path]}
      <div class="pin-row">
        {#if icon}<img class="pin-icon" src={icon} alt="" />{:else}<span class="pin-icon fallback" style:--h={hue(pin.name)}>{initial(pin.name)}</span>{/if}
        <div class="pin-text"><div class="pin-name">{pin.name}</div><div class="sub ellipsis">{pin.path}</div></div>
        <button class="ico" aria-label="Mover {pin.name} para cima" disabled={row === 0} onclick={() => move(row, -1)}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M6 15l6-6 6 6"></path></svg>
        </button>
        <button class="ico" aria-label="Mover {pin.name} para baixo" disabled={row === rows.length - 1} onclick={() => move(row, 1)}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path d="M6 9l6 6 6-6"></path></svg>
        </button>
        <button class="link muted" onclick={() => remove(index)}>Desafixar</button>
      </div>
    {:else}
      <p class="empty">Nada fixado {s.groupsEnabled ? "neste grupo" : ""} ainda. Clique em Adicionar app para escolher um programa.</p>
    {/each}
  </div>
</div>
