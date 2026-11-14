<script lang="ts">
  import { onMount } from "svelte";
  import Panel from "./Panel.svelte";
  import Store from "./Store.svelte";
  import Pins from "./Pins.svelte";
  import System from "./System.svelte";
  import { hub, start } from "./store.svelte.js";

  type Tab = "store" | "pins" | "system";
  let tab = $state<Tab>("store");
  const tabs: Array<[Tab, string]> = [
    ["store", "Loja"],
    ["pins", "Favoritos e grupos"],
    ["system", "Sistema"],
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
    </header>

    {#if hub.error}
      <p class="error" role="alert">Não deu para salvar: {hub.error}</p>
    {/if}

    <div class="body">
      <Panel />
      <main class="main">
        {#if tab === "store"}
          <Store />
        {:else if tab === "pins"}
          <Pins />
        {:else}
          <System />
        {/if}
      </main>
    </div>
  </div>
{:else if hub.error}
  <p class="error" role="alert">O DockForge não respondeu: {hub.error}</p>
{/if}
