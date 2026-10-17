<script lang="ts">
  let { message = $bindable("") }: { message?: string } = $props();

  $effect(() => {
    if (!message) return;
    const timer = setTimeout(() => (message = ""), 5000);
    return () => clearTimeout(timer);
  });
</script>

<div class="region" role="status" aria-live="polite">
  {#if message}
    <p class="micro"><strong>ERR //</strong> {message}</p>
  {/if}
</div>

<style>
  .region {
    position: fixed;
    left: 0;
    right: 0;
    bottom: 0;
    z-index: 50;
    pointer-events: none;
  }
  p {
    margin: 0;
    padding: 10px var(--gutter);
    border-top: var(--rule-thick) solid var(--ink);
    background: var(--red);
    color: var(--on-red);
  }
</style>
