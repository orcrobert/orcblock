<script lang="ts">
  import { api, errorMessage, type Status } from "../lib/api";
  import Button from "../lib/components/Button.svelte";
  import StatusBar from "../lib/components/StatusBar.svelte";
  import Orc from "../lib/mascot/Orc.svelte";
  import { waitFor } from "../lib/wait";

  let {
    onready,
    onerror,
  }: { onready: (status: Status) => void; onerror: (message: string) => void } = $props();

  type Phase = "idle" | "installing" | "starting" | "failed";
  let phase = $state<Phase>("idle");

  async function install() {
    phase = "installing";
    try {
      await api.installHelper();
    } catch (err) {
      phase = "idle";
      onerror(errorMessage(err));
      return;
    }
    phase = "starting";
    const status = await waitFor(api.status, { attempts: 20, intervalMs: 500 });
    if (status) onready(status);
    else phase = "failed";
  }

  const LABELS: Record<Phase, string> = {
    idle: "INSTALL HELPER",
    installing: "INSTALLING…",
    starting: "STARTING HELPER…",
    failed: "TRY AGAIN",
  };

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
      <div>
        <h2>Helper<br />required</h2>
        <p class="micro problem" role="alert">
          {#if phase === "failed"}
            INSTALLED BUT NOT STARTED // SEE /var/log/orcblockd.log
          {/if}
        </p>
      </div>
    </div>
    <figure class="orc">
      <Orc pose="sleeping" scale={5} />
      <figcaption class="micro">ORC-01 // ASLEEP</figcaption>
    </figure>
    <footer class="action">
      <Button disabled={phase === "installing" || phase === "starting"} onclick={install}>
        <span>{LABELS[phase]}</span><span>&gt;&gt;&gt;</span>
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
  .problem {
    min-height: 1lh;
    margin-top: 8px;
    color: var(--red);
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
