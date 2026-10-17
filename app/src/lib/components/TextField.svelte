<script lang="ts">
  let {
    label,
    value = $bindable(""),
    error = "",
    supporting = "",
    placeholder = " ",
    disabled = false,
    inputmode,
    oninput,
  }: {
    label: string;
    value?: string;
    error?: string;
    supporting?: string;
    placeholder?: string;
    disabled?: boolean;
    inputmode?: "text" | "url" | "numeric";
    oninput?: () => void;
  } = $props();

  const id = `field-${Math.random().toString(36).slice(2, 9)}`;
  const help = $derived(error || supporting);
</script>

<div class="field" class:invalid={error}>
  <input
    {id}
    bind:value
    {placeholder}
    {disabled}
    {inputmode}
    autocomplete="off"
    autocapitalize="off"
    spellcheck="false"
    aria-invalid={error ? "true" : undefined}
    aria-describedby={help ? `${id}-help` : undefined}
    {oninput}
  />
  <label for={id}>{label}</label>
  {#if help}
    <p id="{id}-help" class="help" role={error ? "alert" : undefined}>{help}</p>
  {/if}
</div>

<style>
  .field {
    position: relative;
    flex: 1;
    min-width: 0;
  }
  input {
    width: 100%;
    height: 48px;
    padding: 0 var(--space-4);
    border: 1px solid var(--md-sys-color-outline);
    border-radius: var(--md-sys-shape-corner-extra-small);
    background: transparent;
    color: var(--md-sys-color-on-surface);
    font: var(--md-sys-typescale-body-large);
    -webkit-user-select: text;
    user-select: text;
    outline: none;
  }
  input::placeholder {
    color: var(--md-sys-color-on-surface-variant);
    opacity: 0;
  }
  input:focus::placeholder {
    opacity: 1;
  }
  input:hover {
    border-color: var(--md-sys-color-on-surface);
  }
  input:focus {
    border: 2px solid var(--md-sys-color-primary);
    padding: 0 calc(var(--space-4) - 1px);
  }
  label {
    position: absolute;
    left: var(--space-3);
    top: 14px;
    padding: 0 var(--space-1);
    background: var(--label-bg, var(--md-sys-color-surface));
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-body-large);
    pointer-events: none;
    transform-origin: left center;
    transition:
      transform var(--md-sys-motion-duration-short) var(--md-sys-motion-easing-standard),
      color var(--md-sys-motion-duration-short);
  }
  input:focus + label,
  input:not(:placeholder-shown) + label {
    transform: translateY(-22px) scale(0.75);
  }
  input:focus + label {
    color: var(--md-sys-color-primary);
  }
  .help {
    margin: var(--space-1) var(--space-4) 0;
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-body-small);
  }
  .invalid input,
  .invalid input:focus {
    border-color: var(--md-sys-color-error);
  }
  .invalid label,
  .invalid input:focus + label,
  .invalid .help {
    color: var(--md-sys-color-error);
  }
</style>
