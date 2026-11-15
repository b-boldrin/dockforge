<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Dock from "./lib/Dock.svelte";
  import { createSheet, setSheet } from "./lib/themeSheet";
  import { setMotion } from "./lib/motion";
  import type { AppEntry, MediaInfo, Settings, SystemStats, ThemePayload } from "./lib/types";

  let apps = $state<AppEntry[]>([]);
  let settings = $state<Settings | null>(null);
  let media = $state<MediaInfo | null>(null);
  let system = $state<SystemStats | null>(null);
  let hidden = $state(false);
  let received = $state<{ group: number; at: number } | null>(null);

  const themeSheet = createSheet();
  const applyTheme = (t: ThemePayload) => setSheet(themeSheet, t.css, t.id);

  $effect(() => {
    if (settings) setMotion(settings.motion);
  });

  onMount(() => {
    const unlisten: Array<() => void> = [];
    (async () => {
      unlisten.push(await listen<AppEntry[]>("apps-changed", (e) => (apps = e.payload)));
      unlisten.push(await listen<ThemePayload>("theme-changed", (e) => applyTheme(e.payload)));
      unlisten.push(await listen<Settings>("settings-changed", (e) => (settings = e.payload)));
      unlisten.push(await listen<MediaInfo | null>("media-changed", (e) => (media = e.payload)));
      unlisten.push(await listen<SystemStats | null>("system-changed", (e) => (system = e.payload)));
      unlisten.push(await listen<number>("group-received", (e) => (received = { group: e.payload, at: Date.now() })));
      unlisten.push(await listen<boolean>("dock-hidden", (e) => (hidden = e.payload)));
      applyTheme(await invoke<ThemePayload>("get_theme"));
      settings = await invoke<Settings>("get_settings");
      apps = await invoke<AppEntry[]>("list_apps");
      media = await invoke<MediaInfo | null>("get_media");
      system = await invoke<SystemStats | null>("get_system");
    })();
    return () => unlisten.forEach((fn) => fn());
  });
</script>

{#if settings}
  <div class="stage" class:is-hidden={hidden} data-position={settings.position} data-layout={settings.layout} data-align={settings.align}>
    <Dock {apps} {settings} {media} {system} {received} />
  </div>
{/if}
