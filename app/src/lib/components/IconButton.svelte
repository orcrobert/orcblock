<script lang="ts">
  import Icon, { type IconName } from "./Icon.svelte";

  let {
    icon,
    label,
    title,
    type = "button",
    disabled = false,
    pressed,
    onclick,
  }: {
    icon: IconName;
    /** Accessible name; required because the button shows no text. */
    label: string;
    title?: string;
    type?: "button" | "submit";
    disabled?: boolean;
    pressed?: boolean;
    onclick?: (event: MouseEvent) => void;
  } = $props();
</script>

<button
  {type}
  aria-label={label}
  aria-haspopup={pressed === undefined ? undefined : "menu"}
  aria-expanded={pressed}
  title={title ?? label}
  {disabled}
  {onclick}
>
  <Icon name={icon} />
</button>

<style>
  button {
    position: relative;
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    padding: 0;
    border: none;
    border-radius: var(--md-sys-shape-corner-full);
    background: transparent;
    color: var(--md-sys-color-on-surface-variant);
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
    transition: opacity var(--md-sys-motion-duration-short) var(--md-sys-motion-easing-standard);
  }
  button:hover::before {
    opacity: var(--md-sys-state-hover);
  }
  button:active::before {
    opacity: var(--md-sys-state-pressed);
  }
  button:disabled {
    cursor: default;
    color: color-mix(in srgb, var(--md-sys-color-on-surface) 38%, transparent);
  }
  button:disabled::before {
    opacity: 0;
  }
</style>
