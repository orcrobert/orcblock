<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    open = $bindable(false),
    title,
    children,
    actions,
  }: { open?: boolean; title: string; children: Snippet; actions: Snippet } = $props();

  let dialog: HTMLDialogElement | undefined = $state();

  // The native <dialog> traps focus, closes on Escape and renders a backdrop.
  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  });
</script>

<dialog bind:this={dialog} aria-labelledby="dialog-title" onclose={() => (open = false)}>
  <h2 id="dialog-title">{title}</h2>
  <div class="body">{@render children()}</div>
  <div class="actions">{@render actions()}</div>
</dialog>

<style>
  dialog {
    width: min(calc(100vw - 2 * var(--space-6)), 312px);
    padding: var(--space-5);
    border: none;
    border-radius: var(--md-sys-shape-corner-extra-large);
    background: var(--md-sys-color-surface-container-high);
    color: var(--md-sys-color-on-surface);
  }
  dialog::backdrop {
    background: color-mix(in srgb, var(--md-sys-color-scrim) 32%, transparent);
  }
  h2 {
    margin: 0 0 var(--space-4);
    font: var(--md-sys-typescale-headline-small);
  }
  .body {
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-body-medium);
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
    margin-top: var(--space-5);
  }
</style>
