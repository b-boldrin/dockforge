<script lang="ts">
  import Dock from "../lib/Dock.svelte";
  import type { AppEntry, Settings } from "../lib/types";
  import { loadPreviewTheme } from "./themes.svelte.js";
  import { wallpaper } from "./walls";

  let {
    theme,
    apps,
    settings,
    wall = theme,
  }: { theme: string; apps: AppEntry[]; settings: Settings; wall?: string } = $props();

  $effect(() => loadPreviewTheme(theme));

  // A big dock can be bigger than a small preview: shrink it to fit.
  let box: HTMLDivElement;
  let fit: HTMLDivElement;
  let zoom = $state(1);
  $effect(() => {
    const bar = settings.layout === "bar";
    const upright = settings.position === "left" || settings.position === "right";
    const measure = () => {
      const dock = fit.firstElementChild as HTMLElement | null;
      if (!dock) return;
      if (!bar) {
        const pad = 24;
        zoom = Math.min(1, (box.clientWidth - pad) / dock.offsetWidth, (box.clientHeight - pad) / dock.offsetHeight);
        return;
      }
      // A bar is as long as the preview, so measure what it holds instead.
      const parts = [...dock.querySelectorAll<HTMLElement>(".df-item, .df-separator, .df-group")];
      const css = getComputedStyle(dock);
      const gap = parseFloat(css.columnGap) || 0;
      const padding = upright
        ? parseFloat(css.paddingTop) + parseFloat(css.paddingBottom)
        : parseFloat(css.paddingLeft) + parseFloat(css.paddingRight);
      const needed = parts.reduce((sum, el) => sum + (upright ? el.offsetHeight : el.offsetWidth) + gap, padding + 16);
      zoom = Math.min(1, (upright ? box.clientHeight : box.clientWidth) / needed);
    };
    const observer = new ResizeObserver(measure);
    observer.observe(box);
    if (fit.firstElementChild) observer.observe(fit.firstElementChild);
    measure();
    return () => observer.disconnect();
  });
</script>

<div bind:this={box} class="df-preview" data-theme={theme} data-position={settings.position} data-layout={settings.layout} data-align={settings.align} style:background-image={wallpaper(wall)}>
  <div bind:this={fit} class="df-fit" style:zoom={zoom}>
    <Dock {apps} {settings} interactive={false} />
  </div>
</div>
