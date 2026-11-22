<script lang="ts">
  // A few colors, "none", and any color: five swatches are enough, the picker does the rest.
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
    ["#26262b", "Grafite"],
    ["#2f6bff", "Azul"],
    ["#2f9e44", "Verde"],
    ["#c8102e", "Vermelho"],
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
