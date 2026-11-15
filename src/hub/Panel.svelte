<script lang="ts">
  import ThemePreview from "./ThemePreview.svelte";
  import { slide } from "svelte/transition";
  import { hub, patch, previewApps } from "./store.svelte.js";
  import type { Settings } from "../lib/types";

  const s = $derived(hub.settings!);
  const themeName = $derived(hub.themes.find((t) => t.id === s.theme)?.name ?? s.theme);
  // The preview is small: shrink the icons but keep every other choice.
  const preview = $derived<Settings>({ ...s, iconSize: Math.round(Math.min(s.iconSize, 48) * 0.62) });

  const positions: Array<[Settings["position"], string]> = [
    ["bottom", "Embaixo"],
    ["top", "Em cima"],
    ["left", "Esquerda"],
    ["right", "Direita"],
  ];
  const layouts: Array<[Settings["layout"], string]> = [
    ["float", "Flutuante"],
    ["bar", "Barra inteira"],
    ["island", "Ilha"],
  ];
  const aligns: Array<[Settings["align"], string, string]> = [
    ["start", "Início", "Topo"],
    ["center", "Centro", "Centro"],
    ["end", "Fim", "Base"],
  ];
  const vertical = $derived(s.position === "left" || s.position === "right");
  const accents: Array<[string, string]> = [
    ["#6fe3dc", "Ciano"],
    ["#e2524f", "Vermelho"],
    ["#4fbf7d", "Verde"],
    ["#f0c45c", "Dourado"],
    ["#e8f3f0", "Branco"],
  ];

  const num = (e: Event) => Number((e.currentTarget as HTMLInputElement).value);
  const val = (e: Event) => (e.currentTarget as HTMLInputElement | HTMLSelectElement).value;

  function renameGroup(i: number, name: string) {
    const names = [...s.groupNames];
    names[i] = name;
    patch({ groupNames: names });
  }

  const MIN_GROUPS = 4;
  const MAX_GROUPS = 9;
  function addGroup() {
    if (s.groupNames.length < MAX_GROUPS) patch({ groupNames: [...s.groupNames, `Grupo ${s.groupNames.length + 1}`] });
  }
  /** Removes the last group. Its apps go back to group 1 as ordinary open apps. */
  function removeGroup() {
    if (s.groupNames.length <= MIN_GROUPS) return;
    const last = s.groupNames.length - 1;
    patch({
      groupNames: s.groupNames.slice(0, last),
      pins: s.pins.filter((p) => p.group !== last),
      activeGroup: Math.min(s.activeGroup, last - 1),
    });
  }
  const slideOpts = { duration: 220 };
</script>

<aside class="panel" aria-label="Sua dock">
  <h2 class="panel-title">Sua dock</h2>
  <p class="sub">Tema: {themeName}</p>
  <div class="panel-preview">
    <ThemePreview theme={s.theme} apps={previewApps(6)} settings={preview} />
  </div>

  <h3 class="ph">Estilo</h3>
  <div class="seg-label" id="seg-layout">Formato</div>
  <div class="seg-group" role="group" aria-labelledby="seg-layout">
    {#each layouts as [id, label] (id)}
      <button class="seg" class:on={s.layout === id} aria-pressed={s.layout === id} onclick={() => patch({ layout: id })}>{label}</button>
    {/each}
  </div>
  <div class="seg-label" id="seg-edge">Borda da tela</div>
  <div class="seg-group" role="group" aria-labelledby="seg-edge">
    {#each positions as [id, label] (id)}
      <button
        class="seg"
        class:on={s.position === id}
        aria-pressed={s.position === id}
        disabled={s.layout === "island" && (id === "left" || id === "right")}
        onclick={() => patch({ position: id })}>{label}</button>
    {/each}
  </div>
  <div class="seg-label" id="seg-align">{s.layout === "bar" ? "Apps na barra" : "Posição na borda"}</div>
  <div class="seg-group" role="group" aria-labelledby="seg-align">
    {#each aligns as [id, horizontal, upright] (id)}
      <button class="seg" class:on={s.align === id} aria-pressed={s.align === id} onclick={() => patch({ align: id })}>
        {vertical ? upright : horizontal}
      </button>
    {/each}
  </div>
  {#if s.layout === "island"}
    <p class="hint">A ilha mostra o app em foco e os widgets. Aponte para ela e os outros apps aparecem.</p>
  {/if}
  <div class="prow">
    <label for="isz">Tamanho</label>
    <span class="range"><input id="isz" type="range" min="32" max="72" step="4" value={s.iconSize} onchange={(e) => patch({ iconSize: num(e) })} /><span class="rv">{s.iconSize} px</span></span>
  </div>
  <div class="prow">
    <label for="mag">Ampliação</label>
    <span class="range"><input id="mag" type="range" min="1" max="1.6" step="0.05" value={s.magnify} onchange={(e) => patch({ magnify: num(e) })} /><span class="rv">{Math.round(s.magnify * 100)}%</span></span>
  </div>

  <h3 class="ph">Comportamento</h3>
  <div class="prow">
    <span>Ocultar sozinha</span>
    <button class="tg" aria-label="Ocultar sozinha" aria-pressed={s.autoHide} onclick={() => patch({ autoHide: !s.autoHide })}></button>
  </div>
  {#if !s.autoHide && s.layout !== "island"}
    <div class="prow" transition:slide={slideOpts}>
      <span class="stack"><span>Empurrar as janelas</span><span class="sub">Janelas maximizadas terminam onde a dock começa</span></span>
      <button class="tg" aria-label="Empurrar as janelas" aria-pressed={s.reserveSpace} onclick={() => patch({ reserveSpace: !s.reserveSpace })}></button>
    </div>
  {/if}
  {#if s.autoHide}
    <div class="prow" transition:slide={slideOpts}>
      <label for="dly" class="muted">Esperar</label>
      <span class="range"><input id="dly" type="range" min="0" max="2000" step="100" value={s.hideDelay} onchange={(e) => patch({ hideDelay: num(e) })} /><span class="rv">{(s.hideDelay / 1000).toFixed(1)} s</span></span>
    </div>
  {/if}
  <div class="prow">
    <label for="shw">Mostrar</label>
    <select id="shw" value={s.showMode} onchange={(e) => patch({ showMode: val(e) as Settings["showMode"] })}>
      <option value="both">Favoritos e abertos</option>
      <option value="pinned">Só favoritos</option>
      <option value="open">Só abertos</option>
    </select>
  </div>
  <div class="prow">
    <label for="clk">Ao clicar</label>
    <select id="clk" value={s.clickAction} onchange={(e) => patch({ clickAction: val(e) as Settings["clickAction"] })}>
      <option value="focus-min">Abrir ou minimizar</option>
      <option value="cycle">Alternar janelas</option>
      <option value="focus">Só abrir</option>
    </select>
  </div>
  <div class="prow">
    <span>Separar favoritos e abertos</span>
    <button class="tg" aria-label="Separar favoritos e abertos" aria-pressed={s.separator} onclick={() => patch({ separator: !s.separator })}></button>
  </div>

  {#if !vertical && s.layout !== "island"}
    <div class="prow" transition:slide={slideOpts}>
      <span class="stack"><span>Nome ao lado do ícone</span><span class="sub">Cada app aberto vira um botão com o nome</span></span>
      <button class="tg" aria-label="Nome ao lado do ícone" aria-pressed={s.labels} onclick={() => patch({ labels: !s.labels })}></button>
    </div>
  {/if}

  <h3 class="ph">Favoritos e grupos</h3>
  <div class="prow">
    <span class="stack"><span>Mostrar favoritos</span><span class="sub">Apps que ficam na dock mesmo fechados</span></span>
    <button class="tg" aria-label="Mostrar favoritos" aria-pressed={s.favoritesEnabled} onclick={() => patch({ favoritesEnabled: !s.favoritesEnabled })}></button>
  </div>
  <div class="prow">
    <span class="stack"><span>Grupos de apps</span><span class="sub">De 4 a 9, um atalho Alt+número para cada</span></span>
    <button class="tg" aria-label="Usar grupos" aria-pressed={s.groupsEnabled} onclick={() => patch({ groupsEnabled: !s.groupsEnabled })}></button>
  </div>
  {#if s.groupsEnabled}
  <div transition:slide={slideOpts}>
    <p class="hint">
      O grupo 1 tem os favoritos e tudo que está aberto. Os outros começam vazios: mande o app em foco para um
      deles com Alt+Shift+número, ou arraste o ícone até a bolinha do grupo.
    </p>
    <ol class="groups">
      {#each s.groupNames as name, i (i)}
        <li class:current={i === s.activeGroup} transition:slide={slideOpts}>
          <button class="key" aria-label="Mostrar o grupo {name}" onclick={() => patch({ activeGroup: i })}>Alt+{i + 1}</button>
          <input aria-label="Nome do grupo {i + 1}" value={name} maxlength="24" onchange={(e) => renameGroup(i, val(e))} />
          <span class="count" title="Apps neste grupo">{i === 0 ? s.pins.filter((p) => p.group === 0).length : s.pins.filter((p) => p.group === i).length}</span>
        </li>
      {/each}
    </ol>
    <div class="group-actions">
      <button class="link" disabled={s.groupNames.length >= MAX_GROUPS} onclick={addGroup}>
        {s.groupNames.length >= MAX_GROUPS ? "Limite de 9 grupos" : `Adicionar grupo (Alt+${s.groupNames.length + 1})`}
      </button>
      {#if s.groupNames.length > MIN_GROUPS}
        <button class="link muted" onclick={removeGroup}>Remover o último</button>
      {/if}
    </div>
  </div>
  {/if}

  <h3 class="ph">Widgets</h3>
  <div class="prow">
    <span class="stack"><span>Tocando agora</span><span class="sub">Escolha os players em Configurações</span></span>
    <button class="tg" aria-label="Tocando agora" aria-pressed={s.widgetMedia} onclick={() => patch({ widgetMedia: !s.widgetMedia })}></button>
  </div>
  <div class="prow">
    <span>Relógio</span>
    <button class="tg" aria-label="Relógio" aria-pressed={s.widgetClock} onclick={() => patch({ widgetClock: !s.widgetClock })}></button>
  </div>

  <div class="prow">
    <span class="stack"><span>Sistema</span><span class="sub">Processador, memória e bateria</span></span>
    <button class="tg" aria-label="Sistema" aria-pressed={s.widgetSystem} onclick={() => patch({ widgetSystem: !s.widgetSystem })}></button>
  </div>

  <h3 class="ph">Ajustes do tema</h3>
  <div class="prow">
    <span>Destaque</span>
    <span class="swatches">
      {#each accents as [color, label] (color)}
        <button class="sw" class:on={s.accent === color} aria-label="Destaque {label}" aria-pressed={s.accent === color} style:background={color} onclick={() => patch({ accent: color })}></button>
      {/each}
    </span>
  </div>
  <div class="prow">
    <label for="rad">Cantos</label>
    <span class="range"><input id="rad" type="range" min="0" max="32" step="2" value={s.radius ?? 20} onchange={(e) => patch({ radius: num(e) })} /><span class="rv">{s.radius == null ? "tema" : `${s.radius} px`}</span></span>
  </div>
  <div class="prow">
    <label for="opa">Opacidade</label>
    <span class="range"><input id="opa" type="range" min="30" max="100" step="5" value={s.opacity ?? 100} onchange={(e) => patch({ opacity: num(e) })} /><span class="rv">{s.opacity == null ? "tema" : `${s.opacity}%`}</span></span>
  </div>
  <div class="panel-actions">
    <button class="link muted" onclick={() => patch({ accent: null, radius: null, opacity: null })}>Desfazer ajustes</button>
  </div>
</aside>
