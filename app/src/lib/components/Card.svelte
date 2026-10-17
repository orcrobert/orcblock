<script lang="ts">
  import type { Snippet } from "svelte";

  let { title, children }: { title?: string; children: Snippet } = $props();
</script>

<section class="card" aria-label={title}>
  {#if title}<h2>{title}</h2>{/if}
  {@render children()}
</section>

<style>
  .card {
    --label-bg: var(--md-sys-color-surface-container-low);
    padding: var(--space-4);
    border-radius: var(--md-sys-shape-corner-large);
    background: var(--md-sys-color-surface-container-low);
  }
  h2 {
    margin: 0 0 var(--space-3);
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-title-small);
  }
</style>
