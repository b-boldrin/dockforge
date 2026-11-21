<script lang="ts">
  // A grid of colors, like the style panel of a whiteboard: a few swatches, "none", and any color.
  // `live` is true while a color is being dragged around in the picker, so the editor can keep
  // that run as one undo step.
  let {
    value,
    onpick,
    none = "Nenhum",
    label,
  }: { value: string | null; onpick: (value: string | null, live: boolean) => void; none?: string; label: string } = $props();

  const PALETTE: Array<[string, string]> = [
    ["#ffffff", "Branco"],
    ["#e1e4e8", "Cinza claro"],
    ["#9fa8b2", "Cinza"],
    ["#26262b", "Grafite"],
    ["#101014", "Preto"],
    ["#c8102e", "Vermelho natal"],
    ["#f76707", "Laranja"],
    ["#e0a23a", "Dourado"],
    ["#2f9e44", "Verde"],
    ["#0c8599", "Turquesa"],
    ["#2f6bff", "Azul"],
    ["#7048e8", "Violeta"],
  ];
  const custom = $derived(value != null && !PALETTE.some(([hex]) => hex === value));
</script>

<div class="ms-swatches" role="group" aria-label={label}>
  <button class="ms-sw none" class:on={value === null} aria-pressed={value === null} aria-label={none} title={none} onclick={() => onpick(null, false)}></button>
  {#each PALETTE as [hex, name] (hex)}
    <button class="ms-sw" class:on={value === hex} aria-pressed={value === hex} style:background={hex} aria-label={name} title={name} onclick={() => onpick(hex, false)}></button>
  {/each}
  <label class="ms-sw custom" class:on={custom} title="Outra cor">
    <input type="color" aria-label="Outra cor" value={value ?? "#ffffff"} oninput={(e) => onpick(e.currentTarget.value, true)} />
  </label>
</div>
