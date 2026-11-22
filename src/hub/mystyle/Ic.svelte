<script lang="ts">
  // One 24px icon: a line drawing (`d`, may hold several sub-paths), or `parts` for icons made
  // of several drawings, each placed with its own transform and either stroked or filled.
  export interface Part {
    d: string;
    /** SVG transform, e.g. "translate(2 7) scale(.4)". */
    t?: string;
    fill?: boolean;
  }
  let { d = "", parts, size = 16 }: { d?: string; parts?: Part[]; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
  {#if parts}
    {#each parts as p, i (i)}
      <!-- A scaled part keeps the stroke width of the rest of the icon. -->
      <path d={p.d} transform={p.t} fill={p.fill ? "currentColor" : "none"} stroke={p.fill ? "none" : "currentColor"} vector-effect={p.t ? "non-scaling-stroke" : undefined} />
    {/each}
  {:else}
    <path {d} />
  {/if}
</svg>
