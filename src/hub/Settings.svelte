<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getVersion } from "@tauri-apps/api/app";
  import { hub, patch } from "./store.svelte.js";
  import { slide } from "svelte/transition";
  import type { Settings } from "../lib/types";

  const s = $derived(hub.settings!);
  let version = $state("");
  let resetDone = $state(false);

  onMount(() => {
    getVersion().then((v) => (version = v)).catch(() => {});
  });

  const open = (url: string) => invoke("open_link", { url }).catch((err) => (hub.error = String(err)));

  // ---- integrations ---------------------------------------------------------
  type Kind = Settings["mediaBlocked"][number];
  const players: Array<[Kind, string, string]> = [
    ["spotify", "Spotify", "O aplicativo do Spotify para Windows."],
    ["browsers", "Navegadores", "YouTube Music, YouTube e outros sites tocando no Chrome, Edge, Firefox, Brave, Opera ou Vivaldi."],
    ["others", "Outros players", "Qualquer outro programa que apareça nos controles de mídia do Windows."],
  ];
  function togglePlayer(kind: Kind) {
    const blocked = s.mediaBlocked.includes(kind) ? s.mediaBlocked.filter((k) => k !== kind) : [...s.mediaBlocked, kind];
    patch({ mediaBlocked: blocked });
  }
  const agents = ["Claude Code", "OpenCode", "Codex"];

  async function reset() {
    hub.settings = await invoke<Settings>("reset_settings");
    resetDone = true;
  }
</script>

<div class="page">
  <h1>Configurações</h1>
  <div class="srow">
    <div><div class="lbl">Iniciar com o Windows</div><div class="sub">A dock abre sozinha quando você liga o computador.</div></div>
    <button class="tg" aria-label="Iniciar com o Windows" aria-pressed={s.startWithWindows} onclick={() => patch({ startWithWindows: !s.startWithWindows })}></button>
  </div>
  <div class="srow">
    <div>
      <div class="lbl">Esconder a barra de tarefas do Windows</div>
      <div class="sub">Some enquanto o DockForge está aberto e volta quando ele fecha. A tecla Windows continua abrindo o menu Iniciar.</div>
    </div>
    <button class="tg" aria-label="Esconder a barra de tarefas do Windows" aria-pressed={s.hideTaskbar} onclick={() => patch({ hideTaskbar: !s.hideTaskbar })}></button>
  </div>
  <div class="srow">
    <div><label class="lbl" for="lang">Idioma</label><div class="sub">Vale para a dock e para os menus. Esta janela ainda está só em português.</div></div>
    <select id="lang" value={s.language} onchange={(e) => patch({ language: (e.currentTarget as HTMLSelectElement).value as Settings["language"] })}>
      <option value="pt">Português (Brasil)</option>
      <option value="en">English</option>
    </select>
  </div>

  <div class="srow">
    <div><label class="lbl" for="motion">Animações</label><div class="sub">Na dock e nesta janela. "Seguir o Windows" desliga quando os efeitos de animação do Windows estão desligados.</div></div>
    <select id="motion" value={s.motion} onchange={(e) => patch({ motion: (e.currentTarget as HTMLSelectElement).value as Settings["motion"] })}>
      <option value="on">Ligadas</option>
      <option value="system">Seguir o Windows</option>
      <option value="off">Desligadas</option>
    </select>
  </div>

  <h2>Integrações</h2>
  <p class="lead">O que a dock pode mostrar de outros programas. Cada uma liga e desliga sozinha.</p>
  <div class="srow">
    <div><div class="lbl">Tocando agora</div><div class="sub">Música ou vídeo em reprodução, com capa e botões de pausar e pular.</div></div>
    <button class="tg" aria-label="Tocando agora" aria-pressed={s.widgetMedia} onclick={() => patch({ widgetMedia: !s.widgetMedia })}></button>
  </div>
  {#if s.widgetMedia}
    <div class="nested" transition:slide={{ duration: 220 }}>
      {#each players as [kind, name, about] (kind)}
        <div class="srow">
          <div><div class="lbl">{name}</div><div class="sub">{about}</div></div>
          <button class="tg" aria-label={name} aria-pressed={!s.mediaBlocked.includes(kind)} onclick={() => togglePlayer(kind)}></button>
        </div>
      {/each}
    </div>
  {/if}
  <div class="srow">
    <div><div class="lbl">Sistema</div><div class="sub">Uso do processador, da memória e carga da bateria.</div></div>
    <button class="tg" aria-label="Sistema" aria-pressed={s.widgetSystem} onclick={() => patch({ widgetSystem: !s.widgetSystem })}></button>
  </div>
  <div class="srow">
    <div><div class="lbl">Relógio</div><div class="sub">Hora e data no fim da dock.</div></div>
    <button class="tg" aria-label="Relógio" aria-pressed={s.widgetClock} onclick={() => patch({ widgetClock: !s.widgetClock })}></button>
  </div>
  {#each agents as agent (agent)}
    <div class="srow is-soon">
      <div><div class="lbl">{agent}</div><div class="sub">Quanto ainda resta do seu limite de uso. Ainda não está pronto.</div></div>
      <span class="soon">Em breve</span>
    </div>
  {/each}

  <h2>Sobre</h2>
  <div class="srow">
    <div><div class="lbl">DockForge {version}</div><div class="sub">Código aberto, licença MIT.</div></div>
  </div>
  <div class="srow"><button class="link" onclick={() => open("https://github.com/b-boldrin/dockforge")}>Código no GitHub</button></div>
  <div class="srow"><button class="link" onclick={() => open("https://github.com/b-boldrin/dockforge-themes")}>Como criar e publicar um tema</button></div>
  <div class="srow"><button class="link" onclick={() => open("https://github.com/b-boldrin/dockforge/issues/new")}>Relatar um problema</button></div>
  <div class="row-actions">
    <button class="btn" onclick={() => invoke("open_themes_folder")}>Abrir pasta de temas</button>
    <button class="btn danger" onclick={reset}>Restaurar configurações padrão</button>
  </div>
  {#if resetDone}
    <p class="sub" role="status">Configurações restauradas. Seus apps fixados foram mantidos.</p>
  {/if}
</div>
