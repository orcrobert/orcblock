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
  <Orc state="sleeping" size={180} />
  <h2>The orc needs a post</h2>
  <p>
    OrcBlock installs a small helper that keeps blocks in place even when this window is closed.
    macOS will ask for your password once.
  </p>
  <Button icon="shield" disabled={installing} onclick={install}>
    {installing ? "Installing…" : "Install the helper"}
  </Button>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-6) var(--space-5);
    text-align: center;
  }
  h2 {
    margin: 0;
    font: var(--md-sys-typescale-headline-small);
  }
  p {
    max-width: 320px;
    margin: 0 0 var(--space-2);
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-body-medium);
  }
</style>
