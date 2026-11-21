<script lang="ts">
  import { clamp } from "./model";

  // A number with an optional slider: the label, the slider, and a box to type the exact value.
  let {
    label,
    value,
    min,
    max,
    step = 1,
    unit = "",
    slider = true,
    onchange,
  }: { label: string; value: number; min: number; max: number; step?: number; unit?: string; slider?: boolean; onchange: (value: number) => void } = $props();

  const typed = (e: Event) => {
    const n = Number((e.currentTarget as HTMLInputElement).value);
    if (Number.isFinite(n)) onchange(clamp(n, min, max));
  };
</script>

<div class="ms-row">
  <span class="ms-row-l">{label}</span>
  {#if slider}
    <input class="ms-range" type="range" aria-label={label} {min} {max} {step} {value} oninput={(e) => onchange(Number(e.currentTarget.value))} />
  {/if}
  <span class="ms-num">
    <input type="number" aria-label="{label} (valor)" {min} {max} {step} {value} onchange={typed} />
    {#if unit}<i>{unit}</i>{/if}
  </span>
</div>
