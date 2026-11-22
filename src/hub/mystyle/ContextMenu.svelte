<script lang="ts">
  import Ic from "./Ic.svelte";
  import { editor } from "./editor.svelte";

  // The right-click menu of the canvas and the layers: what can be done to the selected element.

  let el = $state<HTMLDivElement>();
  // Kept on screen: a click near the right or bottom edge opens the menu toward the inside.
  let pos = $state({ x: 0, y: 0 });
  $effect(() => {
    const at = editor.menu;
    if (!at || !el) return;
    const r = el.getBoundingClientRect();
    pos = { x: Math.min(at.x, innerWidth - r.width - 8), y: Math.min(at.y, innerHeight - r.height - 8) };
  });

  const close = () => (editor.menu = null);
  function run(action: () => void) {
    action();
    close();
  }
  const element = $derived(editor.current);

  interface Item {
    label: string;
    keys: string;
    icon: string;
    enabled: boolean;
    action: () => void;
    danger?: boolean;
  }
  const items = $derived<Array<Item | "sep">>([
    { label: "Copiar", keys: "Ctrl+C", icon: "M8 8h12v12H8zM4 16V4h12", enabled: !!element, action: () => editor.copy() },
    { label: "Recortar", keys: "Ctrl+X", icon: "M6 9a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM6 21a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM8.1 7.9 20 20M8.1 16.1 20 4", enabled: !!element, action: () => editor.cut() },
    { label: "Colar", keys: "Ctrl+V", icon: "M9 4h6v3H9zM7 5.5H5V21h14V5.5h-2", enabled: !!editor.clipboard, action: () => editor.paste() },
    { label: "Duplicar", keys: "Ctrl+D", icon: "M12 5v14M5 12h14", enabled: !!element, action: () => editor.duplicate() },
    "sep",
    { label: "Excluir", keys: "Delete", icon: "M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3", enabled: !!element, action: () => editor.remove(), danger: true },
  ]);

  function outside(e: PointerEvent) {
    if (editor.menu && el && !el.contains(e.target as Node)) close();
  }
</script>

<svelte:window onpointerdown={outside} onblur={close} onkeydown={(e) => editor.menu && e.key === "Escape" && (e.preventDefault(), close())} onresize={close} />

{#if editor.menu}
  <div class="ms-menu" role="menu" tabindex="-1" aria-label="Ações do elemento" bind:this={el} style:left="{pos.x}px" style:top="{pos.y}px" oncontextmenu={(e) => e.preventDefault()}>
    {#each items as item, i (i)}
      {#if item === "sep"}
        <div class="ms-menu-sep" role="separator"></div>
      {:else}
        <button role="menuitem" class:danger={item.danger} disabled={!item.enabled} onclick={() => run(item.action)}>
          <Ic d={item.icon} size={14} />
          <span class="ms-menu-l">{item.label}</span>
          <kbd>{item.keys}</kbd>
        </button>
      {/if}
    {/each}
  </div>
{/if}
