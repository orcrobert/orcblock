<script lang="ts">
  import { api, errorMessage } from "../lib/api";
  import Button from "../lib/components/Button.svelte";
  import Orc from "../lib/mascot/Orc.svelte";

  let { onerror }: { onerror: (message: string) => void } = $props();

  let installing = $state(false);

  async function install() {
    installing = true;
    try {
      await api.installHelper();
    } catch (err) {
      onerror(errorMessage(err));
    } finally {
      installing = false;
    }
  }
</script>

<main>
  <section class="panel">
    <span class="mark"><Orc state="sleeping" size={32} /></span>
    <h2>Install the helper</h2>
    <p>
      A small background helper keeps blocks running when this window is closed, after a restart,
      and until the timer ends. macOS asks for your password once.
    </p>
    <Button wide icon="shield" disabled={installing} onclick={install}>
      {installing ? "Installing…" : "Install helper"}
    </Button>
  </section>
</main>

<style>
  main {
    max-width: 480px;
    margin: 0 auto;
    padding: var(--space-2) var(--space-4) var(--space-6);
  }
  .panel {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-3);
    padding: var(--space-5);
    border-radius: var(--md-sys-shape-corner-extra-large);
    background: var(--md-sys-color-surface-container-low);
  }
  .mark {
    display: grid;
    place-items: center;
    width: 56px;
    height: 56px;
    border-radius: var(--md-sys-shape-corner-large);
    background: var(--md-sys-color-primary-container);
    color: var(--md-sys-color-on-primary-container);
  }
  h2 {
    margin: var(--space-2) 0 0;
    font: var(--md-sys-typescale-headline-small);
  }
  p {
    margin: 0 0 var(--space-3);
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-body-medium);
  }
</style>
