<script lang="ts">
  import type { Snippet } from "svelte";

  // Material 3 outlined text field: the label floats into a notch in the outline.
  let {
    label,
    value = $bindable(""),
    error = "",
    supporting = "",
    disabled = false,
    inputmode,
    oninput,
    trailing,
  }: {
    label: string;
    value?: string;
    error?: string;
    supporting?: string;
    disabled?: boolean;
    inputmode?: "text" | "url" | "numeric";
    oninput?: () => void;
    trailing?: Snippet;
  } = $props();

  const id = `field-${Math.random().toString(36).slice(2, 9)}`;
  let focused = $state(false);
  const help = $derived(error || supporting);
  const floated = $derived(focused || value !== "");
</script>

<div class="field" class:floated class:focused class:invalid={error} class:disabled>
  <div class="box">
    <input
      {id}
      bind:value
      {disabled}
      {inputmode}
      class:has-trailing={trailing}
      autocomplete="off"
      autocapitalize="off"
      spellcheck="false"
      aria-invalid={error ? "true" : undefined}
      aria-describedby={help ? `${id}-help` : undefined}
      onfocus={() => (focused = true)}
      onblur={() => (focused = false)}
      {oninput}
    />
    <label for={id}>{label}</label>
    <fieldset aria-hidden="true">
      <legend><span>{label}</span></legend>
    </fieldset>
    {#if trailing}<div class="trailing">{@render trailing()}</div>{/if}
  </div>
  <p id="{id}-help" class="help" role={error ? "alert" : undefined}>{help}</p>
</div>

<style>
  .field {
    --outline: var(--md-sys-color-outline);
    --label: var(--md-sys-color-on-surface-variant);
    min-width: 0;
  }
  .box {
    position: relative;
    height: 56px;
  }
  input {
    position: relative;
    z-index: 1;
    width: 100%;
    height: 100%;
    padding: 0 var(--space-4);
    border: none;
    background: transparent;
    color: var(--md-sys-color-on-surface);
    font: var(--md-sys-typescale-body-large);
    caret-color: var(--md-sys-color-primary);
    -webkit-user-select: text;
    user-select: text;
    outline: none;
  }
  input.has-trailing {
    padding-right: 52px;
  }
  label {
    position: absolute;
    z-index: 1;
    left: var(--space-4);
    top: 16px;
    max-width: calc(100% - 2 * var(--space-4));
    overflow: hidden;
    color: var(--label);
    font: var(--md-sys-typescale-body-large);
    white-space: nowrap;
    text-overflow: ellipsis;
    pointer-events: none;
    transform-origin: left center;
    transition:
      transform var(--md-sys-motion-duration-short) var(--md-sys-motion-easing-standard),
      color var(--md-sys-motion-duration-short);
  }
  fieldset {
    position: absolute;
    inset: -6px 0 0;
    margin: 0;
    padding: 0 var(--space-3);
    border: 1px solid var(--outline);
    border-radius: var(--md-sys-shape-corner-extra-small);
    pointer-events: none;
    transition: border-color var(--md-sys-motion-duration-short);
  }
  legend {
    max-width: 0.01px;
    height: 12px;
    padding: 0;
    font: var(--md-sys-typescale-body-small);
    visibility: hidden;
    white-space: nowrap;
    transition: max-width 50ms;
  }
  legend span {
    display: inline-block;
    padding: 0 var(--space-1);
  }
  .trailing {
    position: absolute;
    z-index: 2;
    right: var(--space-1);
    top: 8px;
  }
  .help {
    min-height: 16px;
    margin: var(--space-1) var(--space-4) 0;
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-body-small);
  }

  .box:hover {
    --outline: var(--md-sys-color-on-surface);
  }
  .floated label {
    transform: translateY(-28px) scale(0.75);
  }
  .floated legend {
    max-width: 100%;
    transition: max-width 100ms 50ms;
  }
  .focused {
    --label: var(--md-sys-color-primary);
  }
  .focused fieldset {
    border: 2px solid var(--md-sys-color-primary);
    padding: 0 calc(var(--space-3) - 1px);
  }
  .invalid {
    --outline: var(--md-sys-color-error);
    --label: var(--md-sys-color-error);
  }
  .invalid .box:hover {
    --outline: var(--md-sys-color-on-error-container);
  }
  .invalid.focused fieldset {
    border-color: var(--md-sys-color-error);
  }
  .invalid .help {
    color: var(--md-sys-color-error);
  }
  .disabled {
    opacity: 0.38;
  }
</style>
