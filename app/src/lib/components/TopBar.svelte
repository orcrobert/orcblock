<script lang="ts">
  import ThemeSwitch from "./ThemeSwitch.svelte";

  let { version }: { version: string } = $props();
</script>

<header data-tauri-drag-region>
  <h1 data-tauri-drag-region>[ ORCBLOCK ]</h1>
  <span class="micro muted" data-tauri-drag-region>REV {version}</span>
  <ThemeSwitch />
</header>

<style>
  header {
    display: grid;
    grid-template-columns: 1fr auto auto;
    align-items: center;
    gap: var(--gutter);
    height: 40px;
    /* Room for the macOS traffic lights of the overlay title bar. */
    padding-left: 78px;
    border-bottom: var(--rule-thick) solid var(--rule);
  }
  h1 {
    margin: 0;
    font: var(--type-label);
    letter-spacing: 0.04em;
  }
</style>
