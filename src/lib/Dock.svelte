<script lang="ts">
  /** A widget that is added grows in from a little smaller, and shrinks out the same way. */
  const pop = () => ({ duration: motionOff() ? 0 : 240, start: 0.82 });
  import { invoke } from "@tauri-apps/api/core";
  import { motionOff } from "./motion";
  import { scale } from "svelte/transition";
  import type { AppEntry, Settings } from "./types";
  import { hue, icons, initial, requestIcon } from "./icons.svelte.js";

  let {
    apps,
    settings,
    interactive = true,
    received = null,
  }: {
    apps: AppEntry[];
    settings: Settings;
    /** false for the previews in the settings window: nothing is clickable, the window is not resized. */
    interactive?: boolean;
    /** The group that just got an app (its dot blinks once). */
    received?: { group: number; at: number } | null;
  } = $props();

  const en = $derived(settings.language === "en");
  const vertical = $derived(settings.position === "left" || settings.position === "right");
  const pinned = $derived(apps.filter((a) => a.pinned));
  const others = $derived(apps.filter((a) => !a.pinned));
  const island = $derived(settings.layout === "island");
  // Names next to the icons only fit a horizontal dock that is not an island.
  const labels = $derived(settings.labels && !vertical && settings.layout !== "island");
  const front = $derived(apps.find((a) => a.active) ?? null);
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

  // ---- modules: inside the dock or each in its own block ---------------------------
  // A theme can ask for split modules with --df-modules: detached. Only a floating dock can
  // split them; bars and islands keep them inside.
  let themeModules = $state<"inline" | "detached">("inline");
  function readThemeModules() {
    if (!dockEl) return;
    themeModules = getComputedStyle(dockEl).getPropertyValue("--df-modules").trim() === "detached" ? "detached" : "inline";
  }
  $effect(() => {
    void settings.theme;
    const frame = requestAnimationFrame(readThemeModules);
    // The theme's sheet can arrive a moment after the setting that names it.
    const late = setTimeout(readThemeModules, 400);
    return () => {
      cancelAnimationFrame(frame);
      clearTimeout(late);
    };
  });
  const placement = $derived(
    settings.layout !== "float" ? "inline" : settings.modulesPlacement === "auto" ? themeModules : settings.modulesPlacement,
  );
  const detached = $derived(placement === "detached");

  // ---- quick panel module: the speakers' level, as the taskbar's corner shows it ----
  let volume = $state<{ level: number; muted: boolean } | null>(null);
  $effect(() => {
    if (!settings.widgetQuick) return;
    const read = () =>
      invoke<{ level: number; muted: boolean } | null>("get_volume")
        .then((v) => (volume = v ?? null))
        .catch(() => (volume = null));
    read();
    const timer = setInterval(read, 2000);
    return () => clearInterval(timer);
  });
  function openQuick() {
    invoke("show_tray_menu").catch((err) => console.error("quick panel failed", err));
  }

  // ---- window size and click-through --------------------------------------
  let dockEl: HTMLDivElement;
  let appsEl: HTMLDivElement;
  /** Full width of the app icons: how far the island opens. */
  let appsWidth = $state(0);
  let lastFit = "";

  $effect(() => {
    void apps.length;
    void settings.iconSize;
    void settings.groupsEnabled;
    void showSeparator;
    if (!island || !appsEl) return;
    // Sum of the children: the wrapper itself is squeezed shut while the island is closed.
    const kids = [...appsEl.children] as HTMLElement[];
    const gap = parseFloat(getComputedStyle(appsEl).columnGap) || 0;
    appsWidth = kids.reduce((w, k) => w + k.offsetWidth, 0) + gap * Math.max(0, kids.length - 1) + 16;
  });

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
    // The island's window always has room for the open island, so opening it never has to
    // resize the window in the middle of the animation.
    const width = island && appsEl ? rect.width - appsEl.getBoundingClientRect().width + appsWidth + icon : rect.width;
    // Room for the hover zoom and the tooltip, on the side away from the screen edge.
    const size = vertical
      ? { width: Math.ceil(rect.width + grow + lift + 240), height: Math.ceil(rect.height) + side * 2 }
      : { width: Math.ceil(width) + side * 2, height: Math.ceil(rect.height + grow + lift + 44 + 8) };
    // How far the dock reaches from its screen edge, plus a small gap: the strip of screen an
    // always-visible dock keeps for itself.
    const gap = settings.layout === "bar" ? 0 : 6;
    const reach = {
      bottom: window.innerHeight - rect.top,
      top: rect.bottom,
      left: rect.right,
      right: window.innerWidth - rect.left,
    }[settings.position];
    const thickness = Math.ceil(reach + gap);
    const key = `${size.width}x${size.height}/${thickness}/${settings.position}/${settings.layout}/${settings.align}/${settings.autoHide}/${settings.reserveSpace}`;
    if (key === lastFit) return;
    lastFit = key;
    invoke("fit_dock", { ...size, thickness }).catch((err) => console.error("fit_dock failed", err));
  }

  /** Tells the app where the dock is in its window, so clicks beside it go through.
   *  Measured from the layout, not from what is on screen: while the dock is slid away by
   *  auto-hide, what is on screen is the hidden position, and a dock that came back with
   *  that position recorded would let every click fall through to the window behind it. */
  function reportHitRect() {
    if (!interactive || !dockEl) return;
    let x = 0;
    let y = 0;
    for (let el: HTMLElement | null = dockEl; el; el = el.offsetParent as HTMLElement | null) {
      x += el.offsetLeft;
      y += el.offsetTop;
    }
    invoke("set_hit_rect", { x, y, width: dockEl.offsetWidth, height: dockEl.offsetHeight }).catch(() => {});
  }

  $effect(() => {
    if (!interactive) return;
    // Re-fit when the layout or the edge changes, not only when the dock's size does.
    void settings.position;
    void settings.layout;
    void settings.align;
    void settings.autoHide;
    void settings.reserveSpace;
    void appsWidth;
    // A dock that grew needs its window now, or the new icon is cut off. A dock that shrank
    // (an app closed) waits for the icon's exit to finish, so the window is resized once.
    let lastLength = 0;
    let later: ReturnType<typeof setTimeout> | undefined;
    const observer = new ResizeObserver(() => {
      const length = vertical ? dockEl.offsetHeight : dockEl.offsetWidth;
      clearTimeout(later);
      if (length >= lastLength) fitWindow();
      else later = setTimeout(fitWindow, 260);
      lastLength = length;
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

  /** The app that was just started: its icon bounces until the window shows up. */
  let launching = $state<string | null>(null);

  /** An app that closed: its icon shrinks and fades while the neighbours close the gap. */
  function leave(node: HTMLElement) {
    if (!interactive || motionOff()) return { duration: 0 };
    const upright = vertical;
    const size = upright ? node.offsetHeight : node.offsetWidth;
    const gap = parseFloat(getComputedStyle(node.parentElement ?? node).columnGap) || 0;
    const side = upright ? "block" : "inline";
    return {
      duration: 180,
      easing: (t: number) => 1 - Math.pow(1 - t, 3),
      css: (t: number) =>
        `opacity:${t};scale:${0.85 + 0.15 * t};${upright ? "height" : "width"}:${size * t}px;min-width:0;` +
        `margin-${side}:${(-gap / 2) * (1 - t)}px;pointer-events:none`,
    };
  }

  // ---- actions -------------------------------------------------------------
  function click(app: AppEntry) {
    if (!interactive || justDragged) return;
    if (app.windows.length === 0 && app.exe) {
      launching = app.key;
      setTimeout(() => launching === app.key && (launching = null), 1400);
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

  function dockMenu(event: MouseEvent) {
    if (!interactive || event.defaultPrevented) return;
    event.preventDefault();
    invoke("show_dock_menu").catch((err) => console.error("show_dock_menu failed", err));
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

</script>

{#snippet item(app: AppEntry, index: number)}
  {@const icon = app.exe ? icons[app.exe] : null}
  <button
    class="df-item"
    class:is-running={app.windows.length > 0}
    class:is-active={app.active}
    class:is-pinned={app.pinned}
    class:is-dragged={drag?.key === app.key}
    class:is-launching={launching === app.key && app.windows.length === 0}
    style:--df-i={index}
    style:translate={drag?.key === app.key ? `${drag.dx}px ${drag.dy}px` : undefined}
    aria-label={app.name}
    tabindex={interactive ? 0 : -1}
    onclick={() => click(app)}
    onpointerdown={(e) => press(e, app)}
    oncontextmenu={(e) => menu(e, app)}
    out:leave
  >
    {#if icon}
      <img class="df-icon" src={icon} alt="" draggable="false" />
    {:else}
      <span class="df-icon df-icon--fallback" style:--df-fallback-hue={hue(app.name)}>{initial(app.name)}</span>
    {/if}
    {#if labels && app.windows.length > 0}
      <span class="df-label">{app.name}</span>
    {/if}
    <span class="df-indicator"></span>
    <span class="df-tooltip">{app.name}</span>
  </button>
{/snippet}

<svelte:window onpointermove={pointerMove} onpointerup={pointerEnd} onpointercancel={() => pointerEnd(null)} />

<!-- A right click on the dock itself, away from the apps, opens the dock's own menu. -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
  class="df-dock"
  oncontextmenu={dockMenu}
  class:is-dragging={drag != null}
  class:has-labels={labels}
  data-position={settings.position}
  data-layout={settings.layout}
  data-align={settings.align}
  bind:this={dockEl}
  style:--df-icon-size="{settings.iconSize}px"
  style:--df-hover-scale={labels ? 1 : settings.magnify}
  style:--df-indicator-active-color={settings.accent ?? undefined}
  style:--df-app-gap={settings.gap != null ? `${settings.gap}px` : undefined}
  style:--df-app-scale={settings.appScale}
  data-indicator={settings.indicator}
  data-modules={placement}
  style:--df-bg={settings.fill ?? undefined}
  style:--df-radius={settings.radius != null ? `${settings.radius}px` : undefined}
  style:background={settings.opacity != null && !detached
    ? `color-mix(in srgb, var(--df-bg) ${settings.opacity}%, transparent)`
    : undefined}
  style:--df-segment-bg={settings.opacity != null && detached
    ? `color-mix(in srgb, var(--df-bg) ${settings.opacity}%, transparent)`
    : undefined}
>
  {#if island}
    {@const icon = front?.exe ? icons[front.exe] : null}
    <span class="df-island-now" aria-hidden="true">
      {#if icon}
        <img class="df-island-icon" src={icon} alt="" draggable="false" />
      {:else if front}
        <span class="df-island-icon df-icon--fallback" style:--df-fallback-hue={hue(front.name)}>{initial(front.name)}</span>
      {:else}
        <span class="df-island-glyph"></span>
      {/if}
    </span>
  {/if}

  <!-- The app icons. Outside the island layout this wrapper does not exist for the layout
       (display: contents); in the island it is what opens and closes. -->
  <div class="df-apps" bind:this={appsEl} style:--df-apps-width="{appsWidth}px">
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

    {#each pinned as app, i (app.key)}
      {@render item(app, i)}
    {/each}
    {#if showSeparator}
      <span class="df-separator" role="separator"></span>
    {/if}
    {#each others as app, i (app.key)}
      {@render item(app, pinned.length + i)}
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
  </div>

  <span class="df-spacer"></span>

  <!-- The quick panel: like the corner of the Windows taskbar, the speakers;
       a click opens volume and the Windows options. -->
  {#if settings.widgetQuick}
    <button
      class="df-widget df-module df-quick"
      data-module="quick"
      transition:scale={pop()}
      aria-label={en ? "Quick settings" : "Painel rápido"}
      title={en ? "Quick settings" : "Painel rápido"}
      tabindex={interactive ? 0 : -1}
      onclick={openQuick}
    >
      <svg viewBox="0 0 24 24" aria-hidden="true">
        <path d="M4 9.5h3.5L12 5.5v13l-4.5-4H4z" />
        {#if volume?.muted || volume?.level === 0}
          <path d="M16 9.5l5 5M21 9.5l-5 5" />
        {:else}
          {#if volume == null || volume.level > 0}<path d="M15.5 9.5a3.5 3.5 0 0 1 0 5" />{/if}
          {#if volume == null || volume.level > 50}<path d="M18 7a7 7 0 0 1 0 10" />{/if}
        {/if}
      </svg>
    </button>
  {/if}

  {#if settings.widgetClock}
    <div class="df-widget df-module df-clock" data-module="clock" transition:scale={pop()}>
      <span class="df-clock-time">{time}</span>
      <span class="df-clock-date">{date}</span>
    </div>
  {/if}
</div>
