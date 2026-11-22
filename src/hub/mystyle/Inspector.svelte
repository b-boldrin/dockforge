<script lang="ts">
  import Seg from "./Seg.svelte";
  import Field from "./Field.svelte";
  import Swatches from "./Swatches.svelte";
  import Ic from "./Ic.svelte";
  import { hub, removeBar, openMyStyle } from "../store.svelte.js";
  import { editor } from "./editor.svelte";
  import { KINDS, POWER_ACTIONS, POWER_LABEL, SOURCE_LABEL, SYSTEM_ITEMS, SYSTEM_LABEL, ZONE_LABEL, ZONES, sizeOf, sizeRange, sized, type ElementPatch } from "./model";
  import type { IconsElement, PowerAction, Settings, SystemItem, Zone } from "../../lib/types";

  // The right panel. "Estilo" is for the selected element, "Barra" for the bar itself; selecting
  // something switches to the tab that has its properties.

  let tab = $state<"style" | "bar">("bar");
  $effect(() => {
    tab = editor.selected === "bar" ? "bar" : "style";
  });

  const d = $derived(editor.draft!);
  const el = $derived(editor.current);
  const saved = $derived(editor.key && editor.key !== "new" ? hub.settings?.saved.find((b) => b.id === editor.key) : null);

  // Element edits. A slider or a color being dragged is one undo step per property.
  const edit = (changes: ElementPatch, live = false) => {
    if (el) editor.update(el.id, changes, live ? `${el.id}:${Object.keys(changes).join()}` : undefined);
  };

  const zones = ZONES.map((z) => [z, ZONE_LABEL[z]] as [Zone, string]);
  const flow: Array<[boolean, string]> = [[false, "Fluxo"], [true, "Livre"]];
  const sources = (Object.keys(SOURCE_LABEL) as IconsElement["source"][]).map((s) => [s, SOURCE_LABEL[s]] as [IconsElement["source"], string]);
  const shows: Array<["time" | "date" | "both", string]> = [["both", "Hora e data"], ["time", "Hora"], ["date", "Data"]];
  const clocks: Array<[boolean, string]> = [[false, "24 horas"], [true, "12 horas"]];
  const opens: Array<["quick" | "settings", string]> = [["quick", "Painel rápido"], ["settings", "Configurações"]];
  /** Adds or removes one entry of a list, keeping the canonical order and at least one entry. */
  function toggle<T extends string>(list: T[], all: T[], item: T): T[] {
    const next = all.filter((x) => (x === item ? !list.includes(x) : list.includes(x)));
    return next.length ? next : list;
  }

  // Bar properties.
  const shapes: Array<[Settings["layout"], string]> = [["float", "Flutuante"], ["bar", "Inteira"]];
  const edges: Array<[Settings["position"], string]> = [["bottom", "Embaixo"], ["top", "Em cima"], ["left", "Esquerda"], ["right", "Direita"]];
  const aligns: Array<[Settings["align"], string]> = [["start", "Início"], ["center", "Meio"], ["end", "Fim"]];
  const barCorners: Array<[number | null, string]> = [[null, "Tema"], [0, "0"], [8, "8"], [16, "16"], [24, "24"], [32, "32"]];
  const marks: Array<[Settings["indicator"], string]> = [["theme", "Do tema"], ["none", "Nenhuma"], ["dot", "Ponto"], ["dash", "Traço"], ["bar", "Barra"], ["glow", "Brilho"]];
  const hovers: Array<[number, string]> = [[1, "Nenhum"], [1.15, "Leve"], [1.3, "Ampliar"], [1.5, "Forte"]];
  const magnify = $derived(hovers.reduce((best, h) => (Math.abs(h[0] - d.magnify) < Math.abs(best[0] - d.magnify) ? h : best), hovers[0])[0]);

  const setShape = (layout: Settings["layout"]) => editor.set(layout === "bar" ? { layout, autoHide: false, reserveSpace: true } : { layout });

  async function deleteBar() {
    if (!saved) return;
    await removeBar(saved.id);
    openMyStyle(null);
  }
</script>

<aside class="ms-right" aria-label="Propriedades">
  <div class="ms-tabs" role="tablist" aria-label="Seções">
    <button role="tab" aria-selected={tab === "style"} class:on={tab === "style"} onclick={() => (tab = "style")}>Estilo</button>
    <button role="tab" aria-selected={tab === "bar"} class:on={tab === "bar"} onclick={() => (tab = "bar")}>Barra</button>
  </div>

  <div class="ms-props">
    {#if tab === "style"}
      {#if el}
        <p class="ms-title">{KINDS[el.kind].name(el)}</p>

        <section class="ms-sec">
          <h3>Alinhamento</h3>
          <Seg label="Zona da barra" options={zones} value={el.zone} onpick={(zone) => editor.move(el.id, { zone, before: null })} />
        </section>

        <section class="ms-sec">
          <h3>Posição</h3>
          <Seg label="Posição" options={flow} value={el.free} onpick={(free) => edit({ free })} />
          {#if el.free}
            <Field label="Deslocamento" value={el.offset} min={0} max={600} unit="px" onchange={(offset) => edit({ offset }, true)} />
            <p class="ms-hint">Livre: fica a esta distância do começo da zona e os vizinhos não se mexem.</p>
          {:else}
            <p class="ms-hint">Em fluxo: cresce com os ícones e empurra os vizinhos.</p>
          {/if}
        </section>

        {#if el.kind === "icons"}
          <section class="ms-sec">
            <h3>Mostrar</h3>
            <Seg label="Apps mostrados" options={sources} value={el.source} onpick={(source) => edit({ source })} />
          </section>

          <section class="ms-sec">
            <h3>Ícones</h3>
            <Field label="Tamanho do ícone" value={el.iconSize} min={24} max={72} unit="px" onchange={(iconSize) => edit({ iconSize }, true)} />
            <Field label="Espaço entre ícones" value={el.gap} min={0} max={24} unit="px" onchange={(gap) => edit({ gap }, true)} />
          </section>
        {:else}
          <section class="ms-sec">
            <h3>Tamanho</h3>
            <Field label="Altura" value={sizeOf(el)} min={sizeRange(el)[0]} max={sizeRange(el)[1]} unit="px" onchange={(n) => edit(sized(el, n), true)} />
            <p class="ms-hint">Também dá para arrastar os quadrados nos cantos da seleção.</p>
          </section>

          <section class="ms-sec">
            <h3>Cor do texto e dos ícones</h3>
            <Swatches label="Cor do conteúdo" value={el.color} none="Do tema" onpick={(color, isLive) => edit({ color }, isLive)} />
          </section>

          {#if el.kind === "clock"}
            <section class="ms-sec">
              <h3>Data e hora</h3>
              <Seg label="O que mostrar" options={shows} value={el.show} onpick={(show) => edit({ show })} />
              {#if el.show !== "date"}
                <Seg label="Formato da hora" options={clocks} value={el.hour12} onpick={(hour12) => edit({ hour12 })} />
                <div class="ms-toggle">
                  <span><b>Segundos</b></span>
                  <button class="tg" aria-label="Segundos" aria-pressed={el.seconds} onclick={() => edit({ seconds: !el.seconds })}></button>
                </div>
              {/if}
              {#if el.show !== "time"}
                <div class="ms-toggle">
                  <span><b>Dia da semana</b></span>
                  <button class="tg" aria-label="Dia da semana" aria-pressed={el.weekday} onclick={() => edit({ weekday: !el.weekday })}></button>
                </div>
              {/if}
            </section>
          {:else if el.kind === "audio"}
            <section class="ms-sec">
              <h3>Volume</h3>
              <div class="ms-toggle">
                <span><b>Mostrar o número</b><i>Ex.: 60%</i></span>
                <button class="tg" aria-label="Mostrar o número" aria-pressed={el.percent} onclick={() => edit({ percent: !el.percent })}></button>
              </div>
              <div class="ms-toggle">
                <span><b>Controle deslizante</b><i>Só em barras na horizontal.</i></span>
                <button class="tg" aria-label="Controle deslizante" aria-pressed={el.slider} onclick={() => edit({ slider: !el.slider })}></button>
              </div>
              <p class="ms-hint">Clique silencia. A roda do mouse muda o volume.</p>
            </section>
          {:else if el.kind === "power"}
            <section class="ms-sec">
              <h3>Itens do menu</h3>
              {#each POWER_ACTIONS as action (action)}
                <div class="ms-toggle">
                  <span><b>{POWER_LABEL[action]}</b></span>
                  <button class="tg" aria-label={POWER_LABEL[action]} aria-pressed={el.actions.includes(action)} onclick={() => edit({ actions: toggle<PowerAction>(el.actions, POWER_ACTIONS, action) })}></button>
                </div>
              {/each}
              <p class="ms-hint">O clique abre o menu; escolher um item é a confirmação.</p>
            </section>
          {:else if el.kind === "system"}
            <section class="ms-sec">
              <h3>Ícones</h3>
              {#each SYSTEM_ITEMS as item (item)}
                <div class="ms-toggle">
                  <span><b>{SYSTEM_LABEL[item]}</b></span>
                  <button class="tg" aria-label={SYSTEM_LABEL[item]} aria-pressed={el.items.includes(item)} onclick={() => edit({ items: toggle<SystemItem>(el.items, SYSTEM_ITEMS, item) })}></button>
                </div>
              {/each}
            </section>
            <section class="ms-sec">
              <h3>Ao clicar</h3>
              <Seg label="Ao clicar" options={opens} value={el.opens} onpick={(o) => edit({ opens: o })} />
            </section>
          {/if}
        {/if}

        <section class="ms-sec">
          <h3>Preenchimento</h3>
          <Swatches label="Cor de preenchimento" value={el.fill} onpick={(fill, isLive) => edit({ fill }, isLive)} />
          <Field label="Opacidade" value={el.opacity} min={0} max={100} unit="%" onchange={(opacity) => edit({ opacity }, true)} />
        </section>

        <section class="ms-sec">
          <h3>Cantos</h3>
          <Field label="Raio" value={el.radius} min={0} max={40} unit="px" onchange={(radius) => edit({ radius }, true)} />
        </section>

        <section class="ms-sec">
          <h3>Margem interna</h3>
          <Field label="Margem" value={el.padding} min={0} max={24} unit="px" onchange={(padding) => edit({ padding }, true)} />
        </section>

        <section class="ms-sec">
          <h3>Borda</h3>
          <Swatches label="Cor da borda" value={el.border} none="Sem borda" onpick={(border, isLive) => edit({ border }, isLive)} />
        </section>

        <div class="ms-actions">
          <button class="ms-btn" onclick={() => editor.duplicate()}><Ic d="M8 8h12v12H8zM4 16V4h12" size={14} />Duplicar</button>
          <button class="ms-btn danger" onclick={() => editor.remove()}><Ic d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" size={14} />Excluir</button>
        </div>
      {:else}
        <p class="ms-empty">Selecione um elemento na barra ou nas camadas para mudar o estilo dele.</p>
      {/if}
    {:else}
      <section class="ms-sec">
        <h3>Tema base</h3>
        <select aria-label="Tema base" value={d.theme} onchange={(e) => editor.set({ theme: e.currentTarget.value })}>
          {#each hub.themes as t (t.id)}<option value={t.id}>{t.name}</option>{/each}
        </select>
      </section>

      <section class="ms-sec">
        <h3>Formato</h3>
        <Seg label="Formato" options={shapes} value={d.layout} onpick={setShape} />
        {#if d.layout === "float"}
          <Seg label="Posição na borda" options={aligns} value={d.align} onpick={(align) => editor.set({ align })} />
        {/if}
      </section>

      <section class="ms-sec">
        <h3>Borda da tela</h3>
        <Seg label="Borda da tela" options={edges} value={d.position} onpick={(position) => editor.set({ position })} />
      </section>

      <section class="ms-sec">
        <h3>Preenchimento</h3>
        <Swatches label="Cor da barra" value={d.fill} none="Do tema" onpick={(fill, isLive) => editor.set({ fill }, isLive ? "bar:fill" : undefined)} />
        <Field label="Opacidade" value={d.opacity ?? 100} min={30} max={100} unit="%" onchange={(opacity) => editor.set({ opacity }, "bar:opacity")} />
      </section>

      <section class="ms-sec">
        <h3>Cantos</h3>
        <Seg label="Cantos da barra" options={barCorners} value={d.radius} onpick={(radius) => editor.set({ radius })} />
      </section>

      <section class="ms-sec">
        <h3>Marca de app aberto</h3>
        <Seg label="Marca de app aberto" options={marks} cols={3} value={d.indicator} onpick={(indicator) => editor.set({ indicator })} />
        <Swatches label="Cor da marca" value={d.accent} none="Do tema" onpick={(accent, isLive) => editor.set({ accent }, isLive ? "bar:accent" : undefined)} />
      </section>

      <section class="ms-sec">
        <h3>Ao passar o mouse</h3>
        <Seg label="Ao passar o mouse no app" options={hovers} value={magnify} onpick={(m) => editor.set({ magnify: m })} />
      </section>

      <section class="ms-sec">
        <h3>Comportamento</h3>
        <div class="ms-toggle">
          <span><b>Ocultar sozinha</b><i>Some e volta ao encostar o mouse na borda.</i></span>
          <button class="tg" aria-label="Ocultar sozinha" aria-pressed={d.autoHide} onclick={() => editor.set({ autoHide: !d.autoHide })}></button>
        </div>
        {#if d.autoHide}
          <div class="ms-toggle">
            <span><b>Só com janela maximizada</b><i>Fica à vista sobre janelas soltas.</i></span>
            <button class="tg" aria-label="Só com janela maximizada" aria-pressed={d.hideOnlyMaximized} onclick={() => editor.set({ hideOnlyMaximized: !d.hideOnlyMaximized })}></button>
          </div>
        {/if}
        <div class="ms-toggle">
          <span><b>Empurrar janelas</b><i>Janelas maximizadas terminam na barra.</i></span>
          <button class="tg" aria-label="Empurrar janelas" aria-pressed={d.reserveSpace} onclick={() => editor.set({ reserveSpace: !d.reserveSpace })}></button>
        </div>
      </section>

      {#if saved}
        <div class="ms-actions">
          <button class="ms-btn danger" onclick={deleteBar}><Ic d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3" size={14} />Excluir esta barra</button>
        </div>
      {/if}
    {/if}
  </div>
</aside>
