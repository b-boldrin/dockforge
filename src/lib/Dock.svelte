<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { motionOff } from "./motion";
  import type { AppEntry, BarElement, ClockElement, IconsElement, PowerElement, Settings, StartElement, SystemElement, Zone } from "./types";
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

  // A bar made in MyStyle: three zones, each holding the fields (elements) that flow in it. With
  // no elements (or as an island) the dock below renders exactly as it always did.
  const elements = $derived(settings.elements ?? []);
  const zoned = $derived(!island && elements.length > 0);
  const zoneNames: Zone[] = ["start", "center", "end"];
  const inZone = (zone: Zone) => elements.filter((el) => el.zone === zone);
  // In a bar the three zones always exist (so the middle stays in the middle); otherwise only
  // the ones that hold something.
  const shownZones = $derived(zoneNames.filter((z) => settings.layout === "bar" || inZone(z).length > 0));
  // The group dots go once, before the first field.
  const dotsZone = $derived(zoneNames.find((z) => inZone(z).length > 0));
  const appsOf = (el: IconsElement) => (el.source === "pinned" ? pinned : el.source === "open" ? others : apps);
  // Hover room and the window's size follow the biggest icons on the bar.
  const dockIconSize = $derived(zoned ? Math.max(...elements.map((el) => (el.kind === "icons" ? el.iconSize : el.size))) : settings.iconSize);

  // ---- modules: clock, volume, power, Windows settings --------------------------
  let now = $state(new Date());
  const clocks = $derived(elements.filter((el): el is ClockElement => el.kind === "clock"));
  $effect(() => {
    if (!zoned || clocks.length === 0) return;
    now = new Date();
    const timer = setInterval(() => (now = new Date()), clocks.some((c) => c.seconds) ? 1000 : 10_000);
    return () => clearInterval(timer);
  });
  const locale = $derived(en ? "en-US" : "pt-BR");
  const timeText = (el: ClockElement) => now.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit", second: el.seconds ? "2-digit" : undefined, hour12: el.hour12 });
  const dateText = (el: ClockElement) =>
    now.toLocaleDateString(locale, { weekday: el.weekday ? "short" : undefined, day: "2-digit", month: "2-digit", year: "numeric" });

  // The volume is read from Windows every couple of seconds while a volume module is on the bar.
  let volume = $state({ level: 60, muted: false });
  const hasAudio = $derived(zoned && elements.some((el) => el.kind === "audio" || el.kind === "system"));
  const gotVolume = (v: unknown) => {
    const next = v as { level: number; muted: boolean } | null;
    if (next && typeof next.level === "number") volume = next;
  };
  $effect(() => {
    if (!hasAudio || !interactive) return;
    const read = () => invoke("volume_get").then(gotVolume).catch(() => {});
    read();
    const timer = setInterval(read, 2000);
    return () => clearInterval(timer);
  });
  const setVolume = (level: number) => {
    const next = Math.max(0, Math.min(100, Math.round(level)));
    volume = { ...volume, level: next, muted: next > 0 ? false : volume.muted };
    invoke("volume_set", { level: next }).then(gotVolume).catch(() => {});
  };
  const toggleMute = () => interactive && invoke("volume_toggle_mute").then(gotVolume).catch(() => {});
  const wheelVolume = (e: WheelEvent) => {
    if (!interactive) return;
    e.preventDefault();
    setVolume(volume.level + (e.deltaY < 0 ? 5 : -5));
  };
  const powerMenu = (el: PowerElement) => interactive && invoke("show_power_menu", { actions: el.actions }).catch(() => {});
  const openSystem = (el: SystemElement) => interactive && invoke("open_system", { target: el.opens }).catch(() => {});
  const openStart = (_el: StartElement) => interactive && invoke("open_system", { target: "start" }).catch(() => {});

  const GLYPH = {
    power: "M12 3v8M7 6.5a7 7 0 1 0 10 0",
    wifi: "M2.5 9a15 15 0 0 1 19 0M5.5 12.5a10 10 0 0 1 13 0M8.5 16a5 5 0 0 1 7 0M12 19.5h.01",
    bluetooth: "M7 7l10 10-5 4V3l5 4L7 17",
    muted: "M4 10v4h4l5 4V6L8 10zM16 9l5 6M21 9l-5 6",
    start: "M4 4h7v7H4zM13 4h7v7h-7zM4 13h7v7H4zM13 13h7v7h-7z",
  };
  /** The speaker with as many waves as the volume has. */
  const speaker = (level: number, muted: boolean) =>
    muted || level === 0 ? GLYPH.muted : "M4 10v4h4l5 4V6L8 10zM16 9a4 4 0 0 1 0 6" + (level > 55 ? "M18.5 6.5a8 8 0 0 1 0 11" : "");

  $effect(() => {
    for (const app of apps) requestIcon(app.exe);
  });

  // ---- window size and click-through --------------------------------------
  let dockEl: HTMLDivElement;
  let appsEl = $state<HTMLDivElement>();
  /** Full width of the app icons: how far the island opens. */
  let appsWidth = $state(0);
  let lastFit = "";

  $effect(() => {
    void apps.length;
    void settings.iconSize;
    void settings.groupsEnabled;
    void zoned;
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
    // From the layout, not from what is on screen: while the dock slides in after auto-hide
    // is turned off, its rectangle is still at the hidden position, which would measure a
    // negative reach and reserve no strip at all.
    const box = layoutBox();
    const reach = {
      bottom: window.innerHeight - box.y,
      top: box.y + box.height,
      left: box.x + box.width,
      right: window.innerWidth - box.x,
    }[settings.position];
    const thickness = Math.ceil(reach + gap);
    const key = `${size.width}x${size.height}/${thickness}/${settings.position}/${settings.layout}/${settings.align}/${settings.autoHide}/${settings.reserveSpace}`;
    if (key === lastFit) return;
    lastFit = key;
    invoke("fit_dock", { ...size, thickness }).catch((err) => console.error("fit_dock failed", err));
  }

  /** Where the dock is in its window, from the layout and not from what is on screen: while
   *  the dock is slid away by auto-hide, what is on screen is the hidden position, and a dock
   *  that came back with that position recorded would let every click fall through to the
   *  window behind it. */
  function layoutBox() {
    let x = 0;
    let y = 0;
    for (let el: HTMLElement | null = dockEl; el; el = el.offsetParent as HTMLElement | null) {
      x += el.offsetLeft;
      y += el.offsetTop;
    }
    return { x, y, width: dockEl.offsetWidth, height: dockEl.offsetHeight };
  }

  /** Tells the app where the dock is in its window, so clicks beside it go through. */
  function reportHitRect() {
    if (!interactive || !dockEl) return;
    invoke("set_hit_rect", layoutBox()).catch(() => {});
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

{#snippet groupDots()}
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
{/snippet}

<!-- One element of a MyStyle bar. A new kind of element is a new branch here. -->
{#snippet glyph(d: string)}
  <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path {d} /></svg>
{/snippet}

{#snippet field(el: BarElement)}
  {@const list = el.kind === "icons" ? appsOf(el) : []}
  <div
    class="df-field"
    data-el={el.id}
    data-kind={el.kind}
    data-free={el.free ? "" : undefined}
    style:--df-icon-size="{el.kind === "icons" ? el.iconSize : el.size}px"
    style:--df-mod-size="{el.kind === "icons" ? 0 : el.size}px"
    style:--df-field-gap={el.kind === "icons" ? `${el.gap}px` : undefined}
    style:color={el.kind === "icons" ? undefined : (el.color ?? "var(--df-fg)")}
    style:padding="{el.padding}px"
    style:border-radius="{el.radius}px"
    style:background={el.fill ? `color-mix(in srgb, ${el.fill} ${el.opacity}%, transparent)` : undefined}
    style:border={el.border ? `1px solid ${el.border}` : undefined}
    style:left={el.free && !vertical ? `${el.offset}px` : undefined}
    style:top={el.free && vertical ? `${el.offset}px` : undefined}
  >
    {#if el.kind === "icons"}
      {#each list as app, i (app.key)}
        {@render item(app, i)}
      {/each}
      {#if list.length === 0 && !interactive}
        <span class="df-field-ghost" aria-hidden="true"></span>
      {/if}
    {:else if el.kind === "start"}
      <button class="df-tool" tabindex={interactive ? 0 : -1} title={en ? "Start" : "Iniciar"} aria-label={en ? "Start" : "Iniciar"} onclick={() => openStart(el)}>
        {@render glyph(GLYPH.start)}
      </button>
    {:else if el.kind === "clock"}
      <span class="df-clock" class:df-clock--both={el.show === "both"} style:height="{el.size}px" style:font-size="{el.size * (el.show === "both" ? 0.4 : 0.55)}px">
        {#if el.show !== "date"}<b>{timeText(el)}</b>{/if}
        {#if el.show !== "time"}<span>{dateText(el)}</span>{/if}
      </span>
    {:else if el.kind === "audio"}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <span class="df-tool-row" onwheel={wheelVolume}>
        <button class="df-tool" tabindex={interactive ? 0 : -1} title="Volume: {volume.muted ? (en ? "muted" : "sem som") : volume.level + "%"}" aria-label={en ? "Mute" : "Silenciar"} onclick={toggleMute}>
          {@render glyph(speaker(volume.level, volume.muted))}
          {#if el.percent}<span class="df-tool-text">{volume.muted ? 0 : volume.level}%</span>{/if}
        </button>
        {#if el.slider && !vertical}
          <input class="df-slider" type="range" min="0" max="100" value={volume.muted ? 0 : volume.level} tabindex={interactive ? 0 : -1} aria-label="Volume" disabled={!interactive} oninput={(e) => setVolume(Number(e.currentTarget.value))} />
        {/if}
      </span>
    {:else if el.kind === "power"}
      <button class="df-tool" tabindex={interactive ? 0 : -1} title={en ? "Power" : "Energia"} aria-label={en ? "Power" : "Energia"} onclick={() => powerMenu(el)}>
        {@render glyph(GLYPH.power)}
      </button>
    {:else if el.kind === "system"}
      <button class="df-tool" tabindex={interactive ? 0 : -1} title={en ? "Quick settings" : "Configurações rápidas"} aria-label={en ? "Quick settings" : "Configurações rápidas"} onclick={() => openSystem(el)}>
        {#each el.items as item (item)}
          {@render glyph(item === "sound" ? speaker(volume.level, volume.muted) : GLYPH[item])}
        {/each}
      </button>
    {/if}
  </div>
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
  data-zoned={zoned ? "" : undefined}
  style:--df-icon-size="{dockIconSize}px"
  style:--df-hover-scale={labels ? 1 : settings.magnify}
  style:--df-indicator-active-color={settings.accent ?? undefined}
  style:--df-app-gap={settings.gap != null ? `${settings.gap}px` : undefined}
  style:--df-app-scale={settings.appScale}
  data-indicator={settings.indicator}
  style:--df-bg={settings.fill ?? undefined}
  style:--df-radius={settings.radius != null ? `${settings.radius}px` : undefined}
  style:background={settings.opacity != null
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

  {#if zoned}
    {#each shownZones as zone (zone)}
      <div class="df-zone" data-zone={zone}>
        {#if zone === dotsZone}{@render groupDots()}{/if}
        {#each inZone(zone) as el (el.id)}
          {@render field(el)}
        {/each}
      </div>
    {/each}
  {:else}
  <!-- The app icons. Outside the island layout this wrapper does not exist for the layout
       (display: contents); in the island it is what opens and closes. -->
  <div class="df-apps" bind:this={appsEl} style:--df-apps-width="{appsWidth}px">
    {@render groupDots()}

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
  {/if}
</div>
