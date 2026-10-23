<script lang="ts">
  import ThemeSwitch from "./ThemeSwitch.svelte";
  import UpdateButton from "./UpdateButton.svelte";

  let { version, onerror }: { version: string; onerror: (message: string) => void } = $props();
</script>

<header data-tauri-drag-region>
  <h1 data-tauri-drag-region>[ ORCBLOCK ]</h1>
  <UpdateButton {onerror} />
  <span class="micro muted" data-tauri-drag-region>REV {version}</span>
  <ThemeSwitch />
</header>

<style>
  header {
    display: flex;
    align-items: center;
    gap: var(--gutter);
    height: 40px;
    /* Room for the macOS traffic lights of the overlay title bar. */
    padding-left: 78px;
    border-bottom: var(--rule-thick) solid var(--rule);
  }
  h1 {
    flex: 1;
    margin: 0;
    font: var(--type-label);
    letter-spacing: 0.04em;
  }
</style>
