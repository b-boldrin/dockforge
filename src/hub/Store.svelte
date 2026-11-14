<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import ThemePreview from "./ThemePreview.svelte";
  import { hub, previewApps, useTheme } from "./store.svelte.js";
  import type { Settings } from "../lib/types";

  const s = $derived(hub.settings!);

  const TAG_LABELS: Record<string, string> = {
    holiday: "Natal",
    dark: "Escuros",
    escuro: "Escuros",
    light: "Claros",
    claro: "Claros",
    minimal: "Minimalistas",
    minimalista: "Minimalistas",
    glass: "Vidro",
    neon: "Neon",
    retro: "Retrô",
    "retrô": "Retrô",
    pastel: "Pastel",
  };
  const label = (tag: string) => TAG_LABELS[tag] ?? tag;

  let filter = $state("all");
  let query = $state("");

  const filters = $derived.by(() => {
    const seen = new Map<string, string>();
    for (const t of hub.themes) for (const tag of t.tags) if (!seen.has(label(tag))) seen.set(label(tag), label(tag));
    return [["all", "Todos"], ...[...seen.keys()].sort().map((l) => [l, l])] as Array<[string, string]>;
  });

  const visible = $derived(
    hub.themes.filter((t) => {
      if (filter !== "all" && !t.tags.some((tag) => label(tag) === filter)) return false;
      const q = query.trim().toLowerCase();
      return !q || `${t.name} ${t.author}`.toLowerCase().includes(q);
    }),
  );

  const hero = $derived(hub.themes.find((t) => t.id === "natal-cristal") ?? hub.themes[0]);

  // Previews: the same choices for every theme, so only the theme changes.
  const base = $derived<Settings>({
    ...s,
    position: "bottom",
    layout: "float",
    accent: null,
    radius: null,
    opacity: null,
    groupsEnabled: false,
    widgetMedia: false,
  });
  const heroSettings = $derived<Settings>({ ...base, iconSize: 44, widgetClock: true });
  const cardSettings = $derived<Settings>({ ...base, iconSize: 22, widgetClock: false, separator: false });
</script>

{#if hero}
  <section class="hero" aria-label="Tema em destaque">
    <div class="hero-stage">
      <ThemePreview theme={hero.id} apps={previewApps(7)} settings={heroSettings} />
    </div>
    <div class="hero-text">
      <p class="hero-kicker">{hero.tags.includes("holiday") ? "Coleção de Natal" : "Em destaque"}</p>
      <h1 class="hero-title">{hero.name}</h1>
      <p class="hero-copy">
        {#if hero.id === "natal-cristal"}
          Vidro verde-pinheiro, borda de gelo ciano e um ponto vermelho em cada app aberto.
        {:else}
          Por {hero.author}.
        {/if}
      </p>
      {#if s.theme === hero.id}
        <span class="badge">Em uso</span>
      {:else}
        <button class="btn pri" onclick={() => useTheme(hero.id)}>Usar este tema</button>
      {/if}
    </div>
  </section>
{/if}

<div class="list-head">
  <div class="list-title">
    <h2>Temas</h2>
    <div class="filters" role="group" aria-label="Filtrar temas">
      {#each filters as [id, text] (id)}
        <button class="flt" class:on={filter === id} aria-pressed={filter === id} onclick={() => (filter = id)}>{text}</button>
      {/each}
    </div>
  </div>
  <input class="search" type="search" placeholder="Buscar por nome ou autor" aria-label="Buscar temas" bind:value={query} />
</div>

<div class="grid">
  {#each visible as t (t.id)}
    <div class="card">
      <button class="card-pic" aria-label="Usar o tema {t.name}" onclick={() => useTheme(t.id)}>
        <ThemePreview theme={t.id} apps={previewApps(5)} settings={cardSettings} />
      </button>
      <div class="card-meta">
        <div>
          <div class="card-name">{t.name}</div>
          <div class="sub">por {t.author}</div>
        </div>
        {#if s.theme === t.id}
          <span class="in-use">Em uso</span>
        {:else}
          <button class="link" onclick={() => useTheme(t.id)}>Usar</button>
        {/if}
      </div>
    </div>
  {/each}
</div>
{#if visible.length === 0}
  <p class="empty">Nenhum tema com esse nome. Tente outra palavra ou volte para Todos.</p>
{/if}

<section class="more">
  <h2>Mais temas</h2>
  <p>
    A galeria online, com os temas da comunidade, chega na próxima versão. Por enquanto, um tema novo entra
    colocando a pasta dele na pasta de temas e clicando em Recarregar temas no ícone da bandeja.
  </p>
  <div class="row-actions">
    <button class="btn" onclick={() => invoke("open_themes_folder")}>Abrir pasta de temas</button>
    <button class="link" onclick={() => invoke("open_link", { url: "https://github.com/dockforge/dockforge-themes" })}>Como criar um tema</button>
  </div>
</section>
