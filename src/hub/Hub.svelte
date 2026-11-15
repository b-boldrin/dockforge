<script lang="ts">
  import { onMount } from "svelte";
  import Panel from "./Panel.svelte";
  import Store from "./Store.svelte";
  import Pins from "./Pins.svelte";
  import Settings from "./Settings.svelte";
  import { hub, start } from "./store.svelte.js";

  type Tab = "store" | "pins" | "settings";
  let tab = $state<Tab>("store");
  const tabs: Array<[Tab, string]> = [
    ["store", "Loja"],
    ["pins", "Favoritos e grupos"],
  ];

  onMount(() => {
    start().catch((err) => (hub.error = String(err)));
  });
</script>

{#if hub.settings}
  <div class="hub">
    <header class="top">
      <div class="brand">
        <svg width="20" height="20" viewBox="0 0 20 20" aria-hidden="true"><rect x="1" y="1" width="18" height="18" rx="5" class="brand-bg"></rect><rect x="4" y="12" width="12" height="4" rx="2" class="brand-dock"></rect></svg>
        <span>DockForge</span>
      </div>
      <nav aria-label="Seções" class="tabs">
        {#each tabs as [id, label] (id)}
          <button class="tab" class:on={tab === id} aria-current={tab === id ? "page" : undefined} onclick={() => (tab = id)}>{label}</button>
        {/each}
      </nav>
      <button
        class="gear"
        class:on={tab === "settings"}
        aria-label="Configurações"
        title="Configurações"
        aria-current={tab === "settings" ? "page" : undefined}
        onclick={() => (tab = "settings")}
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
      <Panel />
      <main class="main">
        {#key tab}
          <div class="view">
            {#if tab === "store"}
              <Store />
            {:else if tab === "pins"}
              <Pins />
            {:else}
              <Settings />
            {/if}
          </div>
        {/key}
      </main>
    </div>
  </div>
{:else if hub.error}
  <p class="error" role="alert">O DockForge não respondeu: {hub.error}</p>
{/if}
