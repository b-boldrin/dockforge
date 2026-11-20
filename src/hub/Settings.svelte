<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { getVersion } from "@tauri-apps/api/app";
  import { hub, patch } from "./store.svelte.js";
  import type { Settings } from "../lib/types";

  const s = $derived(hub.settings!);
  let version = $state("");
  let resetDone = $state(false);

  onMount(() => {
    getVersion().then((v) => (version = v)).catch(() => {});
  });

  const open = (url: string) => invoke("open_link", { url }).catch((err) => (hub.error = String(err)));

  async function reset() {
    hub.settings = await invoke<Settings>("reset_settings");
    resetDone = true;
  }
</script>

<div class="page">
  <h1>Configurações</h1>
  <div class="box">
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
    <div><label class="lbl" for="hubtheme">Aparência desta janela</label><div class="sub">Claro natalino é o padrão. O escuro deixa a janela grafite.</div></div>
    <select id="hubtheme" value={s.hubTheme} onchange={(e) => patch({ hubTheme: (e.currentTarget as HTMLSelectElement).value as Settings["hubTheme"] })}>
      <option value="light">Claro</option>
      <option value="dark">Escuro</option>
    </select>
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
  </div>

  <h2>Sobre</h2>
  <div class="box">
  <div class="srow">
    <div><div class="lbl">DockForge {version}</div><div class="sub">Código aberto, licença MIT.</div></div>
  </div>
  <div class="srow"><button class="link" onclick={() => open("https://github.com/b-boldrin/dockforge")}>Código no GitHub</button></div>
  <div class="srow"><button class="link" onclick={() => open("https://github.com/b-boldrin/dockforge-themes")}>Como criar e publicar um tema</button></div>
  <div class="srow"><button class="link" onclick={() => open("https://github.com/b-boldrin/dockforge/issues/new")}>Relatar um problema</button></div>
  </div>
  <div class="row-actions">
    <button class="btn" onclick={() => invoke("open_themes_folder")}>Abrir pasta de temas</button>
    <button class="btn danger" onclick={reset}>Restaurar configurações padrão</button>
  </div>
  {#if resetDone}
    <p class="sub" role="status">Configurações restauradas. Seus apps fixados foram mantidos.</p>
  {/if}
</div>
