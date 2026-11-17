<script lang="ts">
  import { onMount } from "svelte";
  import Panel from "./Panel.svelte";
  import Store from "./Store.svelte";
  import Pins from "./Pins.svelte";
  import Settings from "./Settings.svelte";
  import Bars from "./Bars.svelte";
  import Creator from "./Creator.svelte";
  import { hub, nav, start } from "./store.svelte.js";
  import { setMotion } from "../lib/motion";

  type Tab = "store" | "pins" | "bars" | "settings";
  const tab = $derived(nav.tab);
  // Minhas barras comes first in the row; the window still opens on the Loja.
  const tabs: Array<[Exclude<Tab, "settings">, string]> = [
    ["bars", "Minhas barras"],
    ["store", "Loja"],
    ["pins", "Favoritos e grupos"],
  ];
  // Small marks for the three sections, so each tab is found by its shape as well as its name.
  const marks: Record<Exclude<Tab, "settings">, string> = {
    store: "M4 9l1.5-5h13L20 9M4 9h16v10H4zM9 19v-5h6v5",
    pins: "M12 3.5l2.6 5.4 5.9.8-4.3 4.1 1 5.9L12 17l-5.2 2.7 1-5.9-4.3-4.1 5.9-.8z",
    bars: "M4 7h16M4 12h16M4 17h10",
  };

  // The "Sua dock" panel slides away to the left when it is not needed. The choice is
  // remembered on this computer; storage can be unavailable, and then it just starts open.
  const PANEL_KEY = "dockforge.panel";
  function remembered(): boolean {
    try {
      return localStorage.getItem(PANEL_KEY) !== "closed";
    } catch {
      return true;
    }
  }
  let panelOpen = $state(remembered());
  function setPanel(open: boolean) {
    panelOpen = open;
    try {
      localStorage.setItem(PANEL_KEY, open ? "open" : "closed");
    } catch {
      // Not being able to remember it is fine.
    }
  }

  let dismissing = false;
  function dismissPanel() {
    if (!panelOpen) return;
    setPanel(false);
    dismissing = true;
    // If no click follows (a drag, a scroll), the next real click must not be eaten.
    setTimeout(() => (dismissing = false), 400);
  }
  function swallowDismissClick(event: MouseEvent) {
    if (!dismissing) return;
    dismissing = false;
    event.stopPropagation();
    event.preventDefault();
  }

  $effect(() => {
    if (hub.settings) setMotion(hub.settings.motion);
    if (hub.settings) document.documentElement.dataset.hub = hub.settings.hubTheme;
  });

  onMount(() => {
    start().catch((err) => (hub.error = String(err)));
  });
</script>

{#if hub.settings}
  <div class="hub">
    {#if tab === "create"}
      <Creator />
    {:else}
    <header class="top">
      <button
        class="gear panel-toggle"
        class:on={panelOpen}
        aria-label={panelOpen ? "Recolher o painel Sua dock" : "Abrir o painel Sua dock"}
        title={panelOpen ? "Recolher o painel" : "Abrir o painel"}
        aria-expanded={panelOpen}
        onclick={() => setPanel(!panelOpen)}
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <rect x="3" y="4" width="18" height="16" rx="3"></rect>
          <path d="M9 4v16"></path>
        </svg>
      </button>
      <div class="brand">
        <svg width="20" height="20" viewBox="0 0 20 20" aria-hidden="true"><rect x="1" y="1" width="18" height="18" rx="5" class="brand-bg"></rect><rect x="4" y="12" width="12" height="4" rx="2" class="brand-dock"></rect></svg>
        <span>DockForge</span>
      </div>
      <nav aria-label="Seções" class="tabs">
        {#each tabs as [id, label] (id)}
          <button class="tab" class:on={tab === id} aria-current={tab === id ? "page" : undefined} onclick={() => (nav.tab = id)}>
            <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d={marks[id]}></path></svg>
            {label}
          </button>
        {/each}
      </nav>
      <button
        class="gear"
        class:on={tab === "settings"}
        aria-label="Configurações"
        title="Configurações"
        aria-current={tab === "settings" ? "page" : undefined}
        onclick={() => (nav.tab = "settings")}
      >
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
          <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"></path>
          <circle cx="12" cy="12" r="3"></circle>
        </svg>
      </button>
    </header>

    {#if hub.error}
      <p class="error" role="alert">Não deu para salvar: {hub.error}</p>
    {/if}

    <div class="body">
      <Panel open={panelOpen} />
      <!-- Clicking anywhere outside the panel puts it away, and that click does nothing else,
           so a theme is not applied by accident. -->
      <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
      <main class="main" onpointerdown={dismissPanel} onclickcapture={swallowDismissClick}>
        {#key tab}
          <div class="view">
            {#if tab === "store"}
              <Store />
            {:else if tab === "pins"}
              <Pins />
            {:else if tab === "bars"}
              <Bars />
            {:else}
              <Settings />
            {/if}
          </div>
        {/key}
      </main>
    </div>
    {/if}
    {#if hub.toast}
      {#key hub.toast.id}
        <div class="toast" role="status">
          <span class="toast-icon">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 4h11l3 3v13H5z"></path><path d="M8 4v5h7V4"></path><path d="M8 20v-6h8v6"></path></svg>
          </span>
          {hub.toast.text}
        </div>
      {/key}
    {/if}
  </div>
{:else if hub.error}
  <p class="error" role="alert">O DockForge não respondeu: {hub.error}</p>
{/if}
