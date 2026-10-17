<script lang="ts">
  let { message = $bindable("") }: { message?: string } = $props();

  $effect(() => {
    if (!message) return;
    const timer = setTimeout(() => (message = ""), 4000);
    return () => clearTimeout(timer);
  });
</script>

<div class="region" role="status" aria-live="polite">
  {#if message}
    <div class="snackbar">{message}</div>
  {/if}
</div>

<style>
  .region {
    position: fixed;
    left: var(--space-4);
    right: var(--space-4);
    bottom: var(--space-4);
    display: flex;
    justify-content: center;
    pointer-events: none;
  }
  .snackbar {
    max-width: 480px;
    padding: var(--space-3) var(--space-4);
    border-radius: var(--md-sys-shape-corner-extra-small);
    background: var(--md-sys-color-inverse-surface);
    color: var(--md-sys-color-inverse-on-surface);
    font: var(--md-sys-typescale-body-medium);
    box-shadow: var(--md-sys-elevation-3);
  }
</style>
