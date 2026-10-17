<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon from "./Icon.svelte";

  let {
    selected = false,
    onclick,
    children,
  }: { selected?: boolean; onclick?: () => void; children: Snippet } = $props();
</script>

<button type="button" class:selected aria-pressed={selected} {onclick}>
  {#if selected}<Icon name="check" size={18} />{/if}
  {@render children()}
</button>

<style>
  button {
    position: relative;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    height: 32px;
    padding: 0 var(--space-4);
    border: 1px solid var(--md-sys-color-outline);
    border-radius: var(--md-sys-shape-corner-small);
    background: transparent;
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-label-large);
    cursor: pointer;
    isolation: isolate;
  }
  button::before {
    content: "";
    position: absolute;
    inset: 0;
    z-index: -1;
    border-radius: inherit;
    background: currentColor;
    opacity: 0;
  }
  button:hover::before {
    opacity: var(--md-sys-state-hover);
  }
  .selected {
    padding-left: var(--space-2);
    border-color: transparent;
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
</style>
