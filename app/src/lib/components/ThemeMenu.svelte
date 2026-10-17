<script lang="ts">
  import { THEMES, applyTheme, loadTheme, type ThemeChoice } from "../theme";
  import Icon from "./Icon.svelte";
  import IconButton from "./IconButton.svelte";

  let choice = $state<ThemeChoice>(loadTheme());
  let open = $state(false);

  function choose(value: ThemeChoice) {
    choice = value;
    applyTheme(value);
    open = false;
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Escape") open = false;
  }
</script>

<svelte:window {onkeydown} />

<div class="anchor">
  <IconButton icon="contrast" label="Theme" pressed={open} onclick={() => (open = !open)} />
  {#if open}
    <button class="dismiss" type="button" aria-label="Close menu" onclick={() => (open = false)}
    ></button>
    <ul role="menu" aria-label="Theme">
      {#each THEMES as theme (theme.value)}
        <li role="none">
          <button
            type="button"
            role="menuitemradio"
            aria-checked={choice === theme.value}
            onclick={() => choose(theme.value)}
          >
            <span class="check">
              {#if choice === theme.value}<Icon name="check" size={20} />{/if}
            </span>
            {theme.label}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .anchor {
    position: relative;
  }
  .dismiss {
    position: fixed;
    inset: 0;
    border: none;
    background: transparent;
    cursor: default;
  }
  ul {
    position: absolute;
    right: 0;
    top: 44px;
    z-index: 10;
    min-width: 160px;
    margin: 0;
    padding: var(--space-2) 0;
    list-style: none;
    border-radius: var(--md-sys-shape-corner-extra-small);
    background: var(--md-sys-color-surface-container);
    box-shadow: var(--md-sys-elevation-2);
  }
  ul button {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    width: 100%;
    height: 48px;
    padding: 0 var(--space-3);
    border: none;
    background: transparent;
    color: var(--md-sys-color-on-surface);
    font: var(--md-sys-typescale-label-large);
    text-align: left;
    cursor: pointer;
  }
  ul button:hover {
    background: color-mix(in srgb, var(--md-sys-color-on-surface) 8%, transparent);
  }
  .check {
    display: grid;
    place-items: center;
    width: 20px;
  }
</style>
