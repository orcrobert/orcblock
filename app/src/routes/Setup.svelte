<script lang="ts">
  import { api, errorMessage } from "../lib/api";
  import Button from "../lib/components/Button.svelte";
  import StatusBar from "../lib/components/StatusBar.svelte";
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

  const FACTS = [
    "RUNS IN THE BACKGROUND AS A LAUNCHDAEMON",
    "KEEPS BLOCKS ACTIVE WHEN THIS WINDOW CLOSES",
    "SURVIVES RESTARTS UNTIL THE TIMER ENDS",
    "ASKS FOR YOUR PASSWORD ONCE",
  ];
</script>

<div class="screen">
  <section class="control">
    <StatusBar armed={false} label="HELPER MISSING" />
    <div class="copy">
      <p class="micro muted">[ SETUP // STEP 01 ]</p>
      <h2>Helper<br />required</h2>
    </div>
    <figure class="orc">
      <Orc pose="sleeping" scale={5} />
      <figcaption class="micro">ORC-01 // ASLEEP</figcaption>
    </figure>
    <footer class="action">
      <Button disabled={installing} onclick={install}>
        <span>{installing ? "INSTALLING…" : "INSTALL HELPER"}</span><span>&gt;&gt;&gt;</span>
      </Button>
    </footer>
  </section>

  <section class="facts">
    <header class="micro"><h3>[ WHAT IT DOES ]</h3></header>
    <ol class="micro">
      {#each FACTS as fact, i (fact)}
        <li><span class="muted">0{i + 1}</span>{fact}</li>
      {/each}
    </ol>
  </section>
</div>

<style>
  .screen {
    display: grid;
    grid-template-columns: clamp(500px, 56%, 600px) 1fr;
    height: 100%;
    min-height: 0;
  }
  .control {
    display: grid;
    grid-template-columns: 1fr 176px;
    grid-template-rows: auto 1fr 64px;
    border-right: var(--rule-thick) solid var(--rule);
  }
  .control > :global(:first-child),
  .action {
    grid-column: 1 / -1;
  }
  .copy {
    display: grid;
    align-content: space-between;
    padding: var(--gutter);
  }
  p {
    margin: 0;
  }
  h2 {
    margin: 0;
    font: var(--type-headline);
    font-size: 60px;
    letter-spacing: var(--tracking-macro);
    text-transform: uppercase;
  }
  .orc {
    display: grid;
    grid-template-rows: 1fr auto;
    justify-items: center;
    align-items: end;
    margin: 0;
    border-left: var(--rule-thin) solid var(--rule);
  }
  figcaption {
    width: 100%;
    padding: 6px 0;
    border-top: var(--rule-thin) solid var(--rule);
    text-align: center;
  }
  .action {
    display: grid;
    border-top: var(--rule-thick) solid var(--rule);
  }
  .facts header {
    display: flex;
    align-items: center;
    height: 28px;
    padding: 0 var(--gutter);
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  h3 {
    margin: 0;
    font: inherit;
  }
  ol {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: grid;
    grid-template-columns: 28px 1fr;
    gap: 10px;
    align-items: center;
    height: var(--row);
    padding-left: var(--gutter);
    border-bottom: var(--rule-thin) solid var(--rule);
  }
</style>
