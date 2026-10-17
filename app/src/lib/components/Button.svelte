<script lang="ts">
  import type { Snippet } from "svelte";
  import Icon, { type IconName } from "./Icon.svelte";

  let {
    variant = "filled",
    icon,
    type = "button",
    disabled = false,
    onclick,
    children,
  }: {
    variant?: "filled" | "tonal" | "outlined" | "text";
    icon?: IconName;
    type?: "button" | "submit";
    disabled?: boolean;
    onclick?: (event: MouseEvent) => void;
    children: Snippet;
  } = $props();
</script>

<button class={variant} class:has-icon={icon} {type} {disabled} {onclick}>
  {#if icon}<Icon name={icon} size={18} />{/if}
  {@render children()}
</button>

<style>
  button {
    position: relative;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    height: 40px;
    padding: 0 var(--space-5);
    border: none;
    border-radius: var(--md-sys-shape-corner-full);
    font: var(--md-sys-typescale-label-large);
    cursor: pointer;
    overflow: hidden;
    isolation: isolate;
    transition: box-shadow var(--md-sys-motion-duration-short) var(--md-sys-motion-easing-standard);
  }
  button.has-icon {
    padding-left: var(--space-4);
  }
  button::before {
    content: "";
    position: absolute;
    inset: 0;
    z-index: -1;
    background: currentColor;
    opacity: 0;
    transition: opacity var(--md-sys-motion-duration-short) var(--md-sys-motion-easing-standard);
  }
  button:hover::before {
    opacity: var(--md-sys-state-hover);
  }
  button:focus-visible::before {
    opacity: var(--md-sys-state-focus);
  }
  button:active::before {
    opacity: var(--md-sys-state-pressed);
  }
  .filled {
    background: var(--md-sys-color-primary);
    color: var(--md-sys-color-on-primary);
  }
  .filled:hover {
    box-shadow: var(--md-sys-elevation-1);
  }
  .tonal {
    background: var(--md-sys-color-secondary-container);
    color: var(--md-sys-color-on-secondary-container);
  }
  .outlined {
    background: transparent;
    color: var(--md-sys-color-primary);
    border: 1px solid var(--md-sys-color-outline);
  }
  .text {
    background: transparent;
    color: var(--md-sys-color-primary);
    padding: 0 var(--space-3);
  }
  button:disabled {
    cursor: default;
    box-shadow: none;
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 12%, transparent);
    color: color-mix(in srgb, var(--md-sys-color-on-surface) 38%, transparent);
    border-color: color-mix(in srgb, var(--md-sys-color-on-surface) 12%, transparent);
  }
  .text:disabled {
    background: transparent;
  }
  button:disabled::before {
    opacity: 0;
  }
</style>
