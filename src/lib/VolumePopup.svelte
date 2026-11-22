<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { hue, icons, initial, requestIcon } from "./icons.svelte.js";

  // The volume panel that opens from the volume button: the master volume, and behind an arrow
  // the volume of each app (our own mixer). The Windows mixer is one button away.

  interface AppVolume {
    pid: number;
    name: string;
    exe: string | null;
    level: number;
    muted: boolean;
  }

  let master = $state({ level: 50, muted: false });
  let apps = $state<AppVolume[]>([]);
  let expanded = $state(false);
  let card = $state<HTMLDivElement>();
  /** A slider is being dragged: what Windows reports must not pull it back. */
  let busy = false;

  const close = () => invoke("volume_popup_close").catch(() => {});

  async function readMaster() {
    const v = (await invoke("volume_get").catch(() => null)) as typeof master | null;
    if (v && !busy) master = v;
  }
  async function readApps() {
    const list = (await invoke("mixer_list").catch(() => null)) as AppVolume[] | null;
    if (!list || busy) return;
    apps = list;
    for (const a of list) requestIcon(a.exe);
  }

  function setMaster(level: number) {
    master = { level, muted: level > 0 ? false : master.muted };
    invoke("volume_set", { level }).catch(() => {});
  }
  const toggleMaster = () => invoke("volume_toggle_mute").then((v) => (master = v as typeof master)).catch(() => {});
  function setApp(app: AppVolume, level: number) {
    app.level = level;
    if (level > 0) app.muted = false;
    invoke("mixer_set", { pid: app.pid, level, muted: level > 0 ? false : null }).catch(() => {});
  }
  function muteApp(app: AppVolume) {
    app.muted = !app.muted;
    invoke("mixer_set", { pid: app.pid, level: null, muted: app.muted }).catch(() => {});
  }
  function openWindowsMixer() {
    invoke("open_system", { target: "mixer" }).catch(() => {});
    close();
  }

  /** The speaker, with as many waves as the volume has. */
  const speaker = (level: number, muted: boolean) =>
    muted || level === 0 ? "M4 10v4h4l5 4V6L8 10zM16 9l5 6M21 9l-5 6" : "M4 10v4h4l5 4V6L8 10zM16 9a4 4 0 0 1 0 6" + (level > 55 ? "M18.5 6.5a8 8 0 0 1 0 11" : "");

  onMount(() => {
    readMaster();
    const timer = setInterval(() => {
      readMaster();
      if (expanded) readApps();
    }, 1500);
    // The window is as tall as the panel: tell the app whenever that changes.
    const observer = new ResizeObserver(() => card && invoke("volume_popup_fit", { height: Math.ceil(card.getBoundingClientRect().height) }).catch(() => {}));
    if (card) observer.observe(card);
    return () => {
      clearInterval(timer);
      observer.disconnect();
    };
  });

  $effect(() => {
    if (expanded) readApps();
  });
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && close()} onpointerup={() => (busy = false)} />

<div class="vp" bind:this={card}>
  <div class="vp-row vp-main">
    <button class="vp-ic" aria-label={master.muted ? "Reativar o som" : "Silenciar"} title={master.muted ? "Reativar o som" : "Silenciar"} onclick={toggleMaster}>
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"><path d={speaker(master.level, master.muted)} /></svg>
    </button>
    <input
      class="vp-slider"
      type="range"
      min="0"
      max="100"
      aria-label="Volume"
      value={master.muted ? 0 : master.level}
      style:--v="{master.muted ? 0 : master.level}%"
      onpointerdown={() => (busy = true)}
      oninput={(e) => setMaster(Number(e.currentTarget.value))}
    />
    <span class="vp-val">{master.muted ? 0 : master.level}</span>
    <button class="vp-more" class:open={expanded} aria-expanded={expanded} aria-label="Volume por app" title="Volume por app" onclick={() => (expanded = !expanded)}>
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 9l6 6 6-6" /></svg>
    </button>
  </div>

  {#if expanded}
    <div class="vp-mixer">
      {#each apps as app (app.pid)}
        {@const icon = app.exe ? icons[app.exe] : null}
        <div class="vp-row vp-app">
          <button class="vp-app-ic" aria-label={app.muted ? `Reativar ${app.name}` : `Silenciar ${app.name}`} title={app.name} onclick={() => muteApp(app)} class:muted={app.muted}>
            {#if icon}
              <img src={icon} alt="" draggable="false" />
            {:else}
              <span class="vp-fallback" style:--hue={hue(app.name)}>{initial(app.name)}</span>
            {/if}
          </button>
          <div class="vp-app-body">
            <span class="vp-app-name">{app.name}</span>
            <input
              class="vp-slider small"
              type="range"
              min="0"
              max="100"
              aria-label="Volume de {app.name}"
              value={app.muted ? 0 : app.level}
              style:--v="{app.muted ? 0 : app.level}%"
              onpointerdown={() => (busy = true)}
              oninput={(e) => setApp(app, Number(e.currentTarget.value))}
            />
          </div>
          <span class="vp-val">{app.muted ? 0 : app.level}</span>
        </div>
      {:else}
        <p class="vp-empty">Nenhum app está tocando som agora.</p>
      {/each}
      <button class="vp-link" onclick={openWindowsMixer}>Abrir o mixer do Windows</button>
    </div>
  {/if}
</div>

<style>
  :global(html),
  :global(body),
  :global(#app) {
    margin: 0;
    height: 100%;
    background: transparent;
    overflow: hidden;
  }
  .vp {
    --bg: rgba(32, 32, 38, 0.96);
    --fg: #f2f2f5;
    --fg-2: #a7a7b2;
    --track: rgba(255, 255, 255, 0.16);
    --hover: rgba(255, 255, 255, 0.1);
    --accent: #60a5fa;
    box-sizing: border-box;
    width: 100%;
    padding: 10px 12px;
    border-radius: 14px;
    border: 1px solid rgba(255, 255, 255, 0.12);
    background: var(--bg);
    color: var(--fg);
    font: 12.5px/1.3 "Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif;
    user-select: none;
    -webkit-user-select: none;
  }
  @media (prefers-color-scheme: light) {
    .vp {
      --bg: rgba(248, 248, 250, 0.97);
      --fg: #1a1a1e;
      --fg-2: #5c5c66;
      --track: rgba(0, 0, 0, 0.16);
      --hover: rgba(0, 0, 0, 0.07);
      --accent: #2563eb;
      border-color: rgba(0, 0, 0, 0.12);
    }
  }
  .vp-row { display: flex; align-items: center; gap: 10px; }
  .vp-main { min-height: 36px; }
  button { font: inherit; color: inherit; border: 0; background: transparent; cursor: pointer; padding: 0; }
  .vp-ic, .vp-more {
    flex: none;
    width: 32px;
    height: 32px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    transition: background 120ms;
  }
  .vp-ic svg { width: 20px; height: 20px; }
  .vp-ic:hover, .vp-more:hover { background: var(--hover); }
  .vp-more svg { transition: rotate 160ms; }
  .vp-more.open svg { rotate: 180deg; }
  .vp-val { flex: none; width: 26px; text-align: right; font-variant-numeric: tabular-nums; color: var(--fg-2); }

  .vp-slider {
    flex: 1;
    min-width: 0;
    height: 22px;
    margin: 0;
    background: transparent;
    appearance: none;
    -webkit-appearance: none;
    cursor: pointer;
  }
  .vp-slider::-webkit-slider-runnable-track {
    height: 6px;
    border-radius: 3px;
    background: linear-gradient(to right, var(--accent) var(--v), var(--track) var(--v));
  }
  .vp-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 16px;
    height: 16px;
    margin-top: -5px;
    border-radius: 50%;
    border: 0;
    background: #fff;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
    transition: scale 120ms;
  }
  .vp-slider:hover::-webkit-slider-thumb, .vp-slider:active::-webkit-slider-thumb { scale: 1.15; }
  .vp-slider.small { height: 16px; }
  .vp-slider.small::-webkit-slider-runnable-track { height: 4px; }
  .vp-slider.small::-webkit-slider-thumb { width: 12px; height: 12px; margin-top: -4px; }

  .vp-mixer { margin-top: 8px; padding-top: 8px; border-top: 1px solid var(--track); display: flex; flex-direction: column; gap: 8px; }
  .vp-app { gap: 10px; }
  .vp-app-ic { flex: none; width: 28px; height: 28px; border-radius: 7px; display: grid; place-items: center; transition: opacity 120ms; }
  .vp-app-ic:hover { background: var(--hover); }
  .vp-app-ic.muted { opacity: 0.4; }
  .vp-app-ic img { width: 22px; height: 22px; }
  .vp-fallback { width: 22px; height: 22px; border-radius: 6px; display: grid; place-items: center; background: hsl(var(--hue) 55% 48%); color: #fff; font-weight: 600; font-size: 11px; }
  .vp-app-body { flex: 1; min-width: 0; display: flex; flex-direction: column; }
  .vp-app-name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; }
  .vp-empty { margin: 2px 0; color: var(--fg-2); text-align: center; }
  .vp-link { align-self: stretch; height: 30px; border-radius: 8px; color: var(--accent); font-weight: 500; }
  .vp-link:hover { background: var(--hover); }
</style>
