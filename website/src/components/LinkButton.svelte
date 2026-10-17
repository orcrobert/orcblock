<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    href,
    variant = "solid",
    download = false,
    children,
  }: {
    /** No href draws the hatched, disabled look of the app's buttons. */
    href?: string;
    variant?: "solid" | "alert" | "plain";
    download?: boolean;
    children: Snippet;
  } = $props();
</script>

{#if href}
  <a class="button {variant}" {href} download={download ? "" : undefined}>{@render children()}</a>
{:else}
  <span class="button off" aria-disabled="true">{@render children()}</span>
{/if}

<style>
  /* The app's Button, as a link. */
  .button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--gutter);
    min-height: 56px;
    padding: 0 var(--gutter);
    font: var(--type-label);
    letter-spacing: 0.02em;
    text-decoration: none;
    text-transform: uppercase;
  }
  .solid {
    background: var(--ink);
    color: var(--paper);
  }
  .alert {
    background: var(--red);
    color: var(--on-red);
  }
  .plain {
    background: var(--paper);
    color: var(--ink);
  }
  a:hover {
    color: var(--paper);
    filter: invert(1);
  }
  .plain:hover {
    filter: none;
    background: var(--ink);
  }
  a:active {
    transform: translateY(1px);
  }
  .off {
    background: repeating-linear-gradient(-45deg, var(--hatch) 0 2px, transparent 2px 7px);
    color: var(--ink-muted);
    cursor: not-allowed;
  }
</style>
