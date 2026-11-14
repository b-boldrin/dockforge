<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { AppEntry, MediaInfo, Settings } from "./types";
  import { hue, icons, initial, requestIcon } from "./icons.svelte.js";

  let {
    apps,
    settings,
    media = null,
    interactive = true,
    received = null,
  }: {
    apps: AppEntry[];
    settings: Settings;
    media?: MediaInfo | null;
    /** false for the previews in the settings window: nothing is clickable, the window is not resized. */
    interactive?: boolean;
    /** The group that just got an app (its dot blinks once). */
    received?: { group: number; at: number } | null;
  } = $props();

  const en = $derived(settings.language === "en");
  const vertical = $derived(settings.position === "left" || settings.position === "right");
  const pinned = $derived(apps.filter((a) => a.pinned));
  const others = $derived(apps.filter((a) => !a.pinned));
  const showSeparator = $derived(settings.separator && pinned.length > 0 && others.length > 0);

  $effect(() => {
    for (const app of apps) requestIcon(app.exe);
  });

  // ---- clock -------------------------------------------------------------
  let now = $state(new Date());
  $effect(() => {
    if (!settings.widgetClock) return;
    const timer = setInterval(() => (now = new Date()), 10_000);
    return () => clearInterval(timer);
  });
  const locale = $derived(en ? "en-US" : "pt-BR");
  const time = $derived(now.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit" }));
  const date = $derived(now.toLocaleDateString(locale, { day: "2-digit", month: "2-digit", year: vertical ? undefined : "numeric" }));

  // ---- window size and click-through --------------------------------------
  let dockEl: HTMLDivElement;

  function fitWindow() {
    if (!interactive || !dockEl) return;
    const rect = dockEl.getBoundingClientRect();
    const css = getComputedStyle(dockEl);
    const num = (name: string, fallback: number) => {
      const v = parseFloat(css.getPropertyValue(name));
      return Number.isFinite(v) ? v : fallback;
    };
    const icon = num("--df-icon-size", 48);
    const scale = num("--df-hover-scale", 1.3);
    const lift = Math.abs(num("--df-hover-lift", -8));
    const grow = Math.max(0, icon * (scale - 1));
    const side = Math.ceil(grow / 2 + 12);
    // Room for the hover zoom and the tooltip, on the side away from the screen edge.
    const size = vertical
      ? { width: Math.ceil(rect.width + grow + lift + 240), height: Math.ceil(rect.height) + side * 2 }
      : { width: Math.ceil(rect.width) + side * 2, height: Math.ceil(rect.height + grow + lift + 44 + 8) };
    invoke("fit_dock", size).catch((err) => console.error("fit_dock failed", err));
  }

  function reportHitRect() {
    if (!interactive || !dockEl) return;
    const r = dockEl.getBoundingClientRect();
    invoke("set_hit_rect", { x: r.left, y: r.top, width: r.width, height: r.height }).catch(() => {});
  }

  $effect(() => {
    if (!interactive) return;
    // Re-fit when the layout or the edge changes, not only when the dock's size does.
    void settings.position;
    void settings.layout;
    const observer = new ResizeObserver(() => {
      fitWindow();
      reportHitRect();
    });
    observer.observe(dockEl);
    const onResize = () => reportHitRect();
    window.addEventListener("resize", onResize);
    fitWindow();
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", onResize);
    };
  });

  // ---- actions -------------------------------------------------------------
  function click(app: AppEntry) {
    if (!interactive || justDragged) return;
    if (app.windows.length === 0 && app.exe) {
      invoke("launch_app", { path: app.exe }).catch((err) => console.error("launch_app failed", err));
      return;
    }
    invoke("activate_app", { hwnds: app.windows.map((w) => w.hwnd) }).catch((err) =>
      console.error("activate_app failed", err),
    );
  }

  function menu(event: MouseEvent, app: AppEntry) {
    if (!interactive) return;
    event.preventDefault();
    invoke("show_app_menu", { key: app.key }).catch((err) => console.error("show_app_menu failed", err));
  }

  // ---- groups: click a dot to switch, drag an icon onto a dot to move it ------
  function showGroup(i: number) {
    if (!interactive || i === settings.activeGroup) return;
    invoke("update_settings", { settings: { ...settings, activeGroup: i } }).catch((err) =>
      console.error("update_settings failed", err),
    );
  }

  let pressed: { key: string; x: number; y: number } | null = null;
  let drag = $state<{ key: string; dx: number; dy: number; over: number | null } | null>(null);
  let justDragged = false;

  function press(event: PointerEvent, app: AppEntry) {
    if (!interactive || !settings.groupsEnabled || event.button !== 0 || !app.exe) return;
    pressed = { key: app.key, x: event.clientX, y: event.clientY };
  }

  function groupUnder(x: number, y: number): number | null {
    const dot = document.elementFromPoint(x, y)?.closest<HTMLElement>("[data-group]");
    return dot ? Number(dot.dataset.group) : null;
  }

  function pointerMove(event: PointerEvent) {
    if (!pressed) return;
    if (event.buttons === 0) return pointerEnd(null);
    const dx = event.clientX - pressed.x;
    const dy = event.clientY - pressed.y;
    if (!drag && Math.hypot(dx, dy) < 8) return;
    drag = { key: pressed.key, dx, dy, over: groupUnder(event.clientX, event.clientY) };
  }

  function pointerEnd(event: PointerEvent | null) {
    if (drag) {
      const target = event ? groupUnder(event.clientX, event.clientY) : null;
      if (target != null && target !== settings.activeGroup) {
        invoke("move_app_to_group", { key: drag.key, group: target }).catch((err) =>
          console.error("move_app_to_group failed", err),
        );
      }
      // The click that follows a drag must not open the app.
      justDragged = true;
      setTimeout(() => (justDragged = false), 0);
    }
    pressed = null;
    drag = null;
  }

  function mediaControl(command: "toggle" | "next" | "previous") {
    if (!interactive) return;
    invoke("media_control", { command }).catch((err) => console.error("media_control failed", err));
  }
</script>

{#snippet item(app: AppEntry)}
  {@const icon = app.exe ? icons[app.exe] : null}
  <button
    class="df-item"
    class:is-running={app.windows.length > 0}
    class:is-active={app.active}
    class:is-pinned={app.pinned}
    class:is-dragged={drag?.key === app.key}
    style:translate={drag?.key === app.key ? `${drag.dx}px ${drag.dy}px` : undefined}
    aria-label={app.name}
    tabindex={interactive ? 0 : -1}
    onclick={() => click(app)}
    onpointerdown={(e) => press(e, app)}
    oncontextmenu={(e) => menu(e, app)}
  >
    {#if icon}
      <img class="df-icon" src={icon} alt="" draggable="false" />
    {:else}
      <span class="df-icon df-icon--fallback" style:--df-fallback-hue={hue(app.name)}>{initial(app.name)}</span>
    {/if}
    <span class="df-indicator"></span>
    <span class="df-tooltip">{app.name}</span>
  </button>
{/snippet}

<svelte:window onpointermove={pointerMove} onpointerup={pointerEnd} onpointercancel={() => pointerEnd(null)} />

<div
  class="df-dock"
  class:is-dragging={drag != null}
  data-position={settings.position}
  data-layout={settings.layout}
  bind:this={dockEl}
  style:--df-icon-size="{settings.iconSize}px"
  style:--df-hover-scale={settings.magnify}
  style:--df-indicator-active-color={settings.accent ?? undefined}
  style:--df-radius={settings.radius != null ? `${settings.radius}px` : undefined}
  style:background={settings.opacity != null
    ? `color-mix(in srgb, var(--df-bg) ${settings.opacity}%, transparent)`
    : undefined}
>
  {#if settings.groupsEnabled}
    <span class="df-group" role="group" aria-label={en ? "Groups" : "Grupos"}>
      {#each settings.groupNames as name, i (i)}
        {#key received?.group === i ? received.at : 0}
          <button
            class="df-group-dot"
            class:is-current={i === settings.activeGroup}
            class:is-over={drag?.over === i}
            class:is-received={received?.group === i}
            data-group={i}
            data-n={i + 1}
            title="{name} (Alt+{i + 1})"
            aria-label="{name} (Alt+{i + 1})"
            aria-pressed={i === settings.activeGroup}
            tabindex={interactive ? 0 : -1}
            onclick={() => showGroup(i)}
          ></button>
        {/key}
      {/each}
    </span>
  {/if}

  {#each pinned as app (app.key)}
    {@render item(app)}
  {/each}
  {#if showSeparator}
    <span class="df-separator" role="separator"></span>
  {/if}
  {#each others as app (app.key)}
    {@render item(app)}
  {/each}
  {#if apps.length === 0}
    <span class="df-empty">
      {#if settings.groupsEnabled && settings.activeGroup > 0}
        {en ? "Empty group · Alt+Shift+" : "Grupo vazio · Alt+Shift+"}{settings.activeGroup + 1}
        {en ? "brings the app in front here" : "traz o app em foco para cá"}
      {:else}
        {en ? "Nothing open or pinned" : "Nada aberto ou fixado"}
      {/if}
    </span>
  {/if}

  <span class="df-spacer"></span>

  {#if settings.widgetMedia && media}
    <div class="df-widget df-media" title="{media.title} — {media.artist} ({media.source})">
      {#if media.thumbnail}
        <img class="df-media-cover" src={media.thumbnail} alt="" draggable="false" />
      {:else}
        <span class="df-media-cover"></span>
      {/if}
      <span class="df-media-text">
        <span class="df-media-title">{media.title}</span>
        <span class="df-media-artist">{media.artist || media.source}</span>
      </span>
      <span class="df-media-buttons">
        <button class="df-media-button" aria-label={en ? "Previous" : "Anterior"} tabindex={interactive ? 0 : -1} onclick={() => mediaControl("previous")}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 6h2v12H7zM10 12l9-6v12z" /></svg>
        </button>
        <button class="df-media-button is-toggle" aria-label={media.playing ? (en ? "Pause" : "Pausar") : en ? "Play" : "Tocar"} tabindex={interactive ? 0 : -1} onclick={() => mediaControl("toggle")}>
          {#if media.playing}
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 5h4v14H7zM13 5h4v14h-4z" /></svg>
          {:else}
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M8 5l11 7-11 7z" /></svg>
          {/if}
        </button>
        <button class="df-media-button" aria-label={en ? "Next" : "Próxima"} tabindex={interactive ? 0 : -1} onclick={() => mediaControl("next")}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M15 6h2v12h-2zM14 12L5 18V6z" /></svg>
        </button>
      </span>
    </div>
  {/if}

  {#if settings.widgetClock}
    <div class="df-widget df-clock">
      <span class="df-clock-time">{time}</span>
      <span class="df-clock-date">{date}</span>
    </div>
  {/if}
</div>
