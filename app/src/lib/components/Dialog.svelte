<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    open = $bindable(false),
    wide = false,
    tag,
    title,
    children,
    actions,
  }: {
    open?: boolean;
    /** Room for a grid, such as the app picker. */
    wide?: boolean;
    /** Micro label in the red header strip. */
    tag: string;
    title: string;
    children: Snippet;
    actions: Snippet;
  } = $props();

  const uid = $props.id();
  let dialog: HTMLDialogElement | undefined = $state();

  // The native <dialog> traps focus, closes on Escape and renders a backdrop.
  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  });
</script>

<dialog bind:this={dialog} class:wide aria-labelledby="{uid}-title" onclose={() => (open = false)}>
  <p class="tag micro">[ {tag} ]</p>
  <h2 id="{uid}-title">{title}</h2>
  <div class="body micro">{@render children()}</div>
  <div class="actions">{@render actions()}</div>
</dialog>

<style>
  dialog {
    width: min(480px, calc(100vw - 2 * var(--gutter)));
    padding: 0;
    border: var(--rule-thick) solid var(--ink);
    background: var(--paper);
    color: var(--ink);
  }
  dialog.wide {
    width: min(720px, calc(100vw - 2 * var(--gutter)));
  }
  dialog::backdrop {
    background: repeating-linear-gradient(-45deg, var(--hatch) 0 2px, transparent 2px 6px);
  }
  .tag {
    margin: 0;
    padding: 8px var(--gutter);
    background: var(--red);
    color: var(--on-red);
  }
  h2 {
    margin: 0;
    padding: 20px var(--gutter) 8px;
    font: var(--type-headline);
    letter-spacing: var(--tracking-macro);
    text-transform: uppercase;
  }
  .body {
    padding: 0 var(--gutter) 20px;
  }
  .actions {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: var(--rule-thin);
    height: 44px;
    border-top: var(--rule-thick) solid var(--ink);
    background: var(--ink);
  }
</style>
