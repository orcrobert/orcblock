<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    variant = "solid",
    type = "button",
    disabled = false,
    onclick,
    children,
  }: {
    /** `solid` is the one main action; `alert` is the potion; `plain` is secondary. */
    variant?: "solid" | "alert" | "plain";
    type?: "button" | "submit";
    disabled?: boolean;
    onclick?: (event: MouseEvent) => void;
    children: Snippet;
  } = $props();
</script>

<button class={variant} {type} {disabled} {onclick}>{@render children()}</button>

<style>
  button {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--gutter);
    width: 100%;
    height: 100%;
    min-height: var(--row);
    padding: 0 var(--gutter);
    border: none;
    font: var(--type-label);
    letter-spacing: 0.02em;
    text-align: left;
    text-transform: uppercase;
    cursor: pointer;
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
  button:not(:disabled):hover {
    filter: invert(1);
  }
  .plain:not(:disabled):hover {
    filter: none;
    background: var(--ink);
    color: var(--paper);
  }
  button:not(:disabled):active {
    transform: translateY(1px);
  }
  button:disabled {
    background: repeating-linear-gradient(-45deg, var(--hatch) 0 2px, transparent 2px 7px);
    color: var(--ink-muted);
    cursor: default;
  }
</style>
