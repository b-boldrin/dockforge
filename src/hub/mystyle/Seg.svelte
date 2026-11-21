<script lang="ts" generics="T extends string | number | boolean | null">
  // A row of exclusive choices, like the segmented controls of a design tool.
  let {
    options,
    value,
    onpick,
    label,
    cols = options.length,
  }: { options: Array<[T, string]>; value: T; onpick: (value: T) => void; label: string; cols?: number } = $props();
</script>

<div class="ms-seg" role="group" aria-label={label} style:grid-template-columns="repeat({cols}, minmax(0, 1fr))">
  {#each options as [v, text] (text)}
    <button class:on={value === v} aria-pressed={value === v} onclick={() => onpick(v)}>{text}</button>
  {/each}
</div>
