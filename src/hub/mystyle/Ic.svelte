<script lang="ts">
  // One 24px line icon. `d` may hold several sub-paths.
  let { d, size = 16 }: { d: string; size?: number } = $props();
</script>

<svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path {d} /></svg>
