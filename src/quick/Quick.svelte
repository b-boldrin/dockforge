<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import type { MediaInfo, Settings, SystemStats } from "../lib/types";

  // The quick panel: Windows' quick settings in their layout (sliders, then tiles), with GNOME's
  // touches (round header buttons, pill-shaped tiles, the date at the top).
  let settings = $state<Settings | null>(null);
  let system = $state<SystemStats | null>(null);
  let media = $state<MediaInfo | null>(null);
  let volume = $state<{ level: number; muted: boolean } | null>(null);
  let now = $state(new Date());

  const readVolume = () =>
    invoke<{ level: number; muted: boolean } | null>("get_volume")
      .then((v) => (volume = v ?? null))
      .catch(() => (volume = null));

  onMount(() => {
    const unlisten: Array<() => void> = [];
    (async () => {
      unlisten.push(await listen<Settings>("settings-changed", (e) => (settings = e.payload)));
      unlisten.push(await listen<SystemStats | null>("system-changed", (e) => (system = e.payload)));
      unlisten.push(await listen<MediaInfo | null>("media-changed", (e) => (media = e.payload)));
      unlisten.push(await listen("quick-opened", () => {
        now = new Date();
        readVolume();
      }));
      settings = await invoke<Settings>("get_settings");
      system = await invoke<SystemStats | null>("get_system");
      media = await invoke<MediaInfo | null>("get_media");
      readVolume();
    })();
    const timer = setInterval(() => {
      now = new Date();
      readVolume();
    }, 1500);
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") invoke("hide_quick");
    };
    window.addEventListener("keydown", onKey);
    return () => {
      unlisten.forEach((fn) => fn());
      clearInterval(timer);
      window.removeEventListener("keydown", onKey);
    };
  });

  const en = $derived(settings?.language === "en");
  const locale = $derived(en ? "en-US" : "pt-BR");
  const dayLine = $derived(now.toLocaleDateString(locale, { weekday: "long", day: "numeric", month: "long" }));
  const timeLine = $derived(now.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit" }));

  function setLevel(e: Event) {
    const level = Number((e.currentTarget as HTMLInputElement).value);
    volume = { level, muted: level === 0 ? volume?.muted ?? false : false };
    invoke("set_volume", { level });
  }
  function toggleMute() {
    if (!volume) return;
    volume = { ...volume, muted: !volume.muted };
    invoke("set_mute", { muted: volume.muted });
  }
  const open = (page: string) => invoke("open_windows_settings", { page });
  const control = (command: string) => invoke("media_control", { command });

  const batteryText = $derived(
    system?.battery != null
      ? `${system.battery}%${system.charging ? (en ? " · charging" : " · carregando") : ""}`
      : en ? "Power options" : "Opções de energia",
  );

  // Pages of Windows' own settings. Tiles open them; nothing here pretends to switch a radio on.
  const tiles = $derived([
    { page: "network", name: en ? "Network" : "Rede", sub: en ? "Wi-Fi and Ethernet" : "Wi-Fi e cabo", icon: "M2.5 9a14 14 0 0 1 19 0M5.5 12.5a9.5 9.5 0 0 1 13 0M8.5 16a5 5 0 0 1 7 0M12 19.5h.01" },
    { page: "bluetooth", name: "Bluetooth", sub: en ? "Devices" : "Dispositivos", icon: "M7 7l10 10-5 4V3l5 4L7 17" },
    { page: "sound", name: en ? "Sound" : "Som", sub: en ? "Speakers" : "Alto-falantes", icon: "M4 9.5h3.5L12 5.5v13l-4.5-4H4zM15.5 9.5a3.5 3.5 0 0 1 0 5M18 7a7 7 0 0 1 0 10" },
    { page: "battery", name: en ? "Power" : "Energia", sub: batteryText, icon: "M3 8h15a1 1 0 0 1 1 1v6a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1V9a1 1 0 0 1 1-1zM22 11v2M10.5 9.5l-2 3h3l-2 3" },
    { page: "display", name: en ? "Display" : "Tela", sub: en ? "Brightness" : "Brilho", icon: "M12 16a4 4 0 1 0 0-8 4 4 0 0 0 0 8zM12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" },
    { page: "notifications", name: en ? "Notifications" : "Notificações", sub: en ? "Alerts" : "Avisos", icon: "M6 16V11a6 6 0 1 1 12 0v5l1.5 2h-15zM10 20.5a2 2 0 0 0 4 0" },
  ]);
</script>

{#if settings}
  <div class="root" data-position={settings.position}>
    <section class="panel" aria-label={en ? "Quick settings" : "Painel rápido"}>
      <header class="head">
        <button class="when" onclick={() => open("datetime")} title={en ? "Date and time" : "Data e hora"}>
          <span class="time">{timeLine}</span>
          <span class="day">{dayLine}</span>
        </button>
        <div class="round-group">
          <button class="round" aria-label={en ? "Windows settings" : "Configurações do Windows"} title={en ? "Windows settings" : "Configurações do Windows"} onclick={() => open("settings")}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 15a3 3 0 1 0 0-6 3 3 0 0 0 0 6z" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-1.8-.3 1.7 1.7 0 0 0-1 1.5V21a2 2 0 0 1-4 0v-.1a1.7 1.7 0 0 0-1.1-1.5 1.7 1.7 0 0 0-1.8.3l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0 .3-1.8 1.7 1.7 0 0 0-1.5-1H3a2 2 0 0 1 0-4h.1a1.7 1.7 0 0 0 1.5-1.1 1.7 1.7 0 0 0-.3-1.8l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 1.8.3H9a1.7 1.7 0 0 0 1-1.5V3a2 2 0 0 1 4 0v.1a1.7 1.7 0 0 0 1 1.5 1.7 1.7 0 0 0 1.8-.3l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0-.3 1.8V9a1.7 1.7 0 0 0 1.5 1H21a2 2 0 0 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1z" /></svg>
          </button>
          <button class="round" aria-label={en ? "Customize DockForge" : "Personalizar o DockForge"} title={en ? "Customize DockForge" : "Personalizar o DockForge"} onclick={() => open("personalize")}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 18h16M4 13h16" /><rect x="3" y="4" width="18" height="5" rx="2" /></svg>
          </button>
          <button class="round" aria-label={en ? "Lock" : "Bloquear"} title={en ? "Lock (Windows+L)" : "Bloquear (Windows+L)"} onclick={() => invoke("lock_computer")}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="5" y="11" width="14" height="9" rx="2" /><path d="M8 11V8a4 4 0 0 1 8 0v3" /></svg>
          </button>
        </div>
      </header>

      {#if volume}
        <div class="slider-row">
          <button class="round small" class:off={volume.muted} aria-label={volume.muted ? (en ? "Unmute" : "Tirar do mudo") : en ? "Mute" : "Silenciar"} onclick={toggleMute}>
            <svg viewBox="0 0 24 24" aria-hidden="true">
              <path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4z" />
              {#if volume.muted}<path d="M16 9.5l5 5M21 9.5l-5 5" />{:else}<path d="M15.5 9.5a3.5 3.5 0 0 1 0 5" /><path d="M18 7a7 7 0 0 1 0 10" />{/if}
            </svg>
          </button>
          <input
            class="slider"
            type="range"
            min="0"
            max="100"
            value={volume.muted ? 0 : volume.level}
            style:--fill="{volume.muted ? 0 : volume.level}%"
            aria-label={en ? "Volume" : "Volume"}
            oninput={setLevel}
          />
          <span class="level">{volume.muted ? 0 : volume.level}</span>
        </div>
      {/if}

      <div class="tiles">
        {#each tiles as t (t.page)}
          <button class="tile" onclick={() => open(t.page)}>
            <span class="tile-icon"><svg viewBox="0 0 24 24" aria-hidden="true"><path d={t.icon} /></svg></span>
            <span class="tile-text"><span class="tile-name">{t.name}</span><span class="tile-sub">{t.sub}</span></span>
            <svg class="chev" viewBox="0 0 24 24" aria-hidden="true"><path d="M9 6l6 6-6 6" /></svg>
          </button>
        {/each}
      </div>

      {#if media}
        <div class="media">
          {#if media.thumbnail}<img class="cover" src={media.thumbnail} alt="" />{:else}<span class="cover"></span>{/if}
          <span class="media-text"><span class="media-title">{media.title}</span><span class="media-sub">{media.artist || media.source}</span></span>
          <button class="round small" aria-label={en ? "Previous" : "Anterior"} onclick={() => control("previous")}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 6v12M18 6l-8 6 8 6z" /></svg></button>
          <button class="round small pri" aria-label={media.playing ? (en ? "Pause" : "Pausar") : en ? "Play" : "Tocar"} onclick={() => control("toggle")}>
            {#if media.playing}<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5v14M16 5v14" /></svg>{:else}<svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5l11 7-11 7z" /></svg>{/if}
          </button>
          <button class="round small" aria-label={en ? "Next" : "Próxima"} onclick={() => control("next")}><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M17 6v12M6 6l8 6-8 6z" /></svg></button>
        </div>
      {/if}

      <footer class="foot">
        {#if system?.battery != null}
          <span class="battery" class:low={system.battery <= 20 && !system.charging}>
            <svg viewBox="0 0 24 24" aria-hidden="true"><rect x="2.5" y="7.5" width="17" height="9" rx="2" /><path d="M21.5 10.5v3" /><rect class="fill" x="4.5" y="9.5" width={13 * system.battery / 100} height="5" rx="0.8" /></svg>
            {batteryText}
          </span>
        {:else}
          <span class="battery muted">{en ? "Plugged in" : "Na tomada"}</span>
        {/if}
        <button class="link" onclick={() => open("taskbar")}>{en ? "Taskbar settings" : "Barra de tarefas"}</button>
      </footer>
    </section>
  </div>
{/if}
