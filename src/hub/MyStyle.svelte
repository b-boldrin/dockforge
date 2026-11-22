<script lang="ts">
  import { untrack } from "svelte";
  import "./mystyle/mystyle.css";
  import Library from "./mystyle/Library.svelte";
  import Canvas from "./mystyle/Canvas.svelte";
  import Inspector from "./mystyle/Inspector.svelte";
  import Ic from "./mystyle/Ic.svelte";
  import ContextMenu from "./mystyle/ContextMenu.svelte";
  import { hub, nav, patch, addBar, saveBarEdit, showToast } from "./store.svelte.js";
  import { editor } from "./mystyle/editor.svelte";
  import { drag } from "./mystyle/dnd.svelte";

  // MyStyle: build the taskbar like a drawing. Elements on the left, a canvas in the middle, the
  // properties on the right. Nothing reaches the real dock until "Usar na minha dock", and
  // nothing is kept until "Salvar". The draft and its history live in `editor`, so leaving the
  // tab and coming back finds the work where it was.

  // Which bar is open follows nav.editBar (set by "Criar barra", "Configurar" and the picker).
  $effect(() => {
    const id = nav.editBar;
    void nav.fresh;
    untrack(() => {
      editor.show(id, nav.fresh);
      nav.fresh = false;
    });
  });

  const saved = $derived(hub.settings?.saved ?? []);
  const editing = $derived(editor.key && editor.key !== "new" ? editor.key : null);

  async function save() {
    const look = editor.look();
    if (editing) {
      await saveBarEdit(editing, editor.name, look);
      editor.saved(editing);
      showToast("Barra salva");
      return;
    }
    const id = await addBar(editor.name, look);
    if (!id) return;
    editor.saved(id);
    nav.editBar = id;
  }
  async function use() {
    await patch(editor.look());
    showToast("Aplicada na sua dock");
  }

  const typing = (t: EventTarget | null) => t instanceof HTMLElement && !!t.closest("input, select, textarea, [contenteditable]");
  function key(e: KeyboardEvent) {
    if (typing(e.target) || e.defaultPrevented) return;
    const k = e.key.toLowerCase();
    const mod = e.ctrlKey || e.metaKey;
    if (mod) {
      if (k === "z") editor[e.shiftKey ? "redo" : "undo"]();
      else if (k === "y") editor.redo();
      else if (k === "d") editor.duplicate();
      else if (k === "c") editor.copy();
      else if (k === "x") editor.cut();
      else if (k === "v") editor.paste();
      else return;
      e.preventDefault();
      return;
    }
    const el = editor.current;
    const side = editor.draft?.position === "left" || editor.draft?.position === "right";
    if (k === "v") editor.tool = "select";
    else if (k === "h") editor.tool = "hand";
    else if (k === "escape") editor.select("bar");
    else if (k === "+" || k === "=") editor.canvas?.zoomBy(0.1);
    else if (k === "-") editor.canvas?.zoomBy(-0.1);
    else if (k === "0") editor.canvas?.fit();
    else if ((k === "delete" || k === "backspace") && el) editor.remove();
    else if (el && k.startsWith("arrow")) {
      // Along the bar: earlier or later in the flow; a free element moves by pixels instead.
      const dir = k === "arrowleft" || k === "arrowup" ? -1 : 1;
      if (side ? k === "arrowleft" || k === "arrowright" : k === "arrowup" || k === "arrowdown") return;
      if (el.free) editor.setOffset(el.id, el.offset + dir * (e.shiftKey ? 10 : 1), `nudge:${el.id}`);
      else editor.nudge(dir);
    } else return;
    e.preventDefault();
  }
</script>

<svelte:window onkeydown={key} />

<div class="ms">
  {#if editor.draft}
    <header class="ms-top" aria-label="Barra de ferramentas">
      <input class="ms-name" aria-label="Nome da barra" maxlength="40" placeholder="Nome da barra" bind:value={editor.name} />
      {#if editor.dirty}<span class="ms-dirty" title="Alterações ainda não salvas" role="img" aria-label="Alterações ainda não salvas"></span>{/if}
      <select
        class="ms-pick"
        aria-label="Barra em edição"
        value={editing ?? ""}
        onchange={(e) => {
          nav.editBar = e.currentTarget.value || null;
          nav.fresh = false;
        }}
      >
        <option value="">Nova barra</option>
        {#each saved as bar (bar.id)}<option value={bar.id}>{bar.name}</option>{/each}
      </select>

      <span class="ms-sep" aria-hidden="true"></span>
      <button class="ms-ib" disabled={!editor.canUndo} title="Desfazer (Ctrl+Z)" aria-label="Desfazer" onclick={() => editor.undo()}><Ic d="M9 14 4 9l5-5M4 9h10.5a5.5 5.5 0 0 1 0 11H11" /></button>
      <button class="ms-ib" disabled={!editor.canRedo} title="Refazer (Ctrl+Shift+Z)" aria-label="Refazer" onclick={() => editor.redo()}><Ic d="M15 14l5-5-5-5M20 9H9.5a5.5 5.5 0 0 0 0 11H13" /></button>

      <span class="ms-sep" aria-hidden="true"></span>
      <div class="ms-zoom" role="group" aria-label="Zoom">
        <button aria-label="Afastar" title="Afastar (-)" onclick={() => editor.canvas?.zoomBy(-0.1)}>−</button>
        <button class="val" title="Ajustar à tela (0)" onclick={() => editor.canvas?.fit()}>{Math.round(editor.zoom * 100)}%</button>
        <button aria-label="Aproximar" title="Aproximar (+)" onclick={() => editor.canvas?.zoomBy(0.1)}>+</button>
      </div>
      <button class="ms-ib" title="Ajustar à tela (0)" aria-label="Ajustar à tela" onclick={() => editor.canvas?.fit()}><Ic d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" /></button>

      <span class="ms-sep" aria-hidden="true"></span>
      <button class="ms-ib" disabled={!editor.current} title="Excluir (Delete)" aria-label="Excluir selecionado" onclick={() => editor.remove()}><Ic d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" /></button>
      <button class="ms-ib" disabled={!editor.current} title="Duplicar (Ctrl+D)" aria-label="Duplicar selecionado" onclick={() => editor.duplicate()}><Ic d="M8 8h12v12H8zM4 16V4h12" /></button>

      <span class="ms-grow"></span>
      <button class="ms-btn" onclick={use}><Ic d="M7 5l12 7-12 7z" size={14} />Usar na minha dock</button>
      <button class="ms-btn pri" onclick={save}><Ic d="M5 4h11l3 3v13H5zM8 4v5h7V4M8 20v-6h8v6" size={14} />Salvar</button>
    </header>

    <div class="ms-body">
      <Library />
      <Canvas />
      <Inspector />
    </div>

    <ContextMenu />

    {#if drag.on}
      <div class="ms-chip" style:left="{drag.x + 14}px" style:top="{drag.y + 14}px">{drag.label}</div>
    {/if}
  {/if}
</div>
