<script lang="ts">
  import ThemeMenu from "./ThemeMenu.svelte";

  let { dryRun = false }: { dryRun?: boolean } = $props();
</script>

<header data-tauri-drag-region>
  <h1 data-tauri-drag-region>OrcBlock</h1>
  {#if dryRun}<span class="badge" title="The dev daemon logs instead of blocking">Dev</span>{/if}
  <span class="spacer" data-tauri-drag-region></span>
  <ThemeMenu />
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    height: 56px;
    /* Room for the macOS traffic lights of the overlay title bar. */
    padding: 0 var(--space-2) 0 80px;
  }
  h1 {
    margin: 0;
    font: var(--md-sys-typescale-title-medium);
  }
  .badge {
    padding: 2px var(--space-2);
    border-radius: var(--md-sys-shape-corner-small);
    background: var(--md-sys-color-tertiary-container);
    color: var(--md-sys-color-on-tertiary-container);
    font: var(--md-sys-typescale-label-medium);
  }
  .spacer {
    flex: 1;
    align-self: stretch;
  }
</style>
