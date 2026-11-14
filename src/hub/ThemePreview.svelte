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

  // A floating dock with widgets can be wider than a small preview: shrink it to fit.
  let box: HTMLDivElement;
  let fit: HTMLDivElement;
  let zoom = $state(1);
  $effect(() => {
    if (settings.layout === "bar") {
      zoom = 1;
      return;
    }
    const measure = () => {
      const dock = fit.firstElementChild as HTMLElement | null;
      if (!dock) return;
      const pad = 24;
      const w = dock.offsetWidth;
      const h = dock.offsetHeight;
      zoom = Math.min(1, (box.clientWidth - pad) / w, (box.clientHeight - pad) / h);
    };
    const observer = new ResizeObserver(measure);
    observer.observe(box);
    if (fit.firstElementChild) observer.observe(fit.firstElementChild);
    measure();
    return () => observer.disconnect();
  });
</script>

<div bind:this={box} class="df-preview" data-theme={theme} data-position={settings.position} data-layout={settings.layout} style:background-image={wallpaper(wall)}>
  <div bind:this={fit} class="df-fit" style:zoom={zoom}>
    <Dock {apps} {settings} interactive={false} />
  </div>
</div>
