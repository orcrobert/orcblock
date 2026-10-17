<script lang="ts">
  import { api, errorMessage, type Site, type Status } from "../lib/api";
  import Button from "../lib/components/Button.svelte";
  import Countdown from "../lib/components/Countdown.svelte";
  import Dialog from "../lib/components/Dialog.svelte";
  import DurationPicker from "../lib/components/DurationPicker.svelte";
  import OrcBox from "../lib/components/OrcBox.svelte";
  import PotionShelf from "../lib/components/PotionShelf.svelte";
  import SiteRow from "../lib/components/SiteRow.svelte";
  import StatusBar from "../lib/components/StatusBar.svelte";
  import { DEFAULT_MINUTES, pad2 } from "../lib/format";

  let {
    status,
    onstatus,
    onerror,
  }: {
    status: Status;
    onstatus: (status: Status) => void;
    onerror: (message: string) => void;
  } = $props();

  let site = $state("");
  let siteError = $state("");
  let minutes = $state(DEFAULT_MINUTES);
  let confirming = $state(false);
  let orcBox: OrcBox | undefined = $state();
  let busy = $state(false);

  const block = $derived(status.block);
  const empty = $derived(status.sites.length === 0);

  async function run(action: () => Promise<Status>): Promise<boolean> {
    busy = true;
    try {
      onstatus(await action());
      return true;
    } catch (err) {
      onerror(errorMessage(err));
      return false;
    } finally {
      busy = false;
    }
  }

  async function addSite(event: SubmitEvent) {
    event.preventDefault();
    if (site.trim() === "") return;
    try {
      onstatus(await api.addSite(site));
      site = "";
      siteError = "";
    } catch (err) {
      siteError = errorMessage(err);
    }
  }

  function removeSite(entry: Site) {
    void run(() => api.removeSite(entry.domain));
  }

  async function drink() {
    const used = status.potions.available - 1;
    if (await run(api.drinkPotion)) {
      confirming = false;
      orcBox?.drink(used);
    }
  }
</script>

<div class="screen">
  <section class="control" aria-label={block ? "Block in progress" : "New block"}>
    <StatusBar armed={block !== null} label={block ? "ARMED" : "IDLE"} mode={status.mode} />
    {#if block}
      <Countdown
        remaining={block.remaining_secs}
        duration={block.duration_secs}
        endsAt={block.ends_at}
        sites={block.domains.length}
      />
    {:else}
      <DurationPicker bind:minutes />
    {/if}
    <OrcBox bind:this={orcBox} potions={status.potions} guarding={block !== null} />
    <footer class="action" class:split={block}>
      {#if block}
        <PotionShelf potions={status.potions} />
        <Button
          variant="alert"
          disabled={busy || status.potions.available === 0}
          onclick={() => (confirming = true)}
        >
          <span>DRINK</span><span>&gt;&gt;&gt;</span>
        </Button>
      {:else}
        <Button disabled={busy || empty} onclick={() => run(() => api.startBlock(minutes))}>
          <span>ARM // START BLOCK</span><span>&gt;&gt;&gt;</span>
        </Button>
      {/if}
    </footer>
  </section>

  <section class="targets" aria-labelledby="targets-title">
    <header class="micro">
      <h2 id="targets-title">[ TARGETS ]</h2>
      <span class="muted">{pad2(status.sites.length)}</span>
    </header>

    <form class="prompt" onsubmit={addSite}>
      <label for="site" class="micro">&gt;</label>
      <input
        id="site"
        bind:value={site}
        placeholder="YOUTUBE.COM"
        inputmode="url"
        autocomplete="off"
        autocapitalize="off"
        spellcheck="false"
        aria-invalid={siteError ? "true" : undefined}
        aria-describedby="site-help"
        oninput={() => (siteError = "")}
      />
      <button type="submit" class="micro" disabled={site.trim() === ""}>ADD +</button>
    </form>
    <p
      id="site-help"
      class="help micro"
      class:error={siteError}
      role={siteError ? "alert" : undefined}
    >
      {siteError || (block ? "NEW TARGETS ARE BLOCKED AT ONCE" : "PASTE A LINK OR TYPE A NAME")}
    </p>

    <div class="list">
      {#if empty}
        <p class="none micro muted">NO TARGETS // ADD A SITE ABOVE</p>
      {:else}
        <ul>
          {#each status.sites as entry, i (entry.domain)}
            <SiteRow site={entry} index={i + 1} locked={block !== null} onremove={removeSite} />
          {/each}
        </ul>
      {/if}
    </div>
  </section>
</div>

<Dialog bind:open={confirming} tag="CONFIRM // SLEEP POTION" title="End block now?">
  USES 1 OF {status.potions.available} POTIONS. TARGETS UNLOCK AT ONCE. REFILL {status.potions
    .refill_on}.
  {#snippet actions()}
    <Button variant="plain" onclick={() => (confirming = false)}>ABORT</Button>
    <Button variant="alert" disabled={busy} onclick={drink}>
      <span>DRINK</span><span>&gt;&gt;&gt;</span>
    </Button>
  {/snippet}
</Dialog>

<style>
  .screen {
    display: grid;
    grid-template-columns: clamp(500px, 56%, 600px) 1fr;
    height: 100%;
    min-height: 0;
  }
  /* Left compartment: status, readout and orc, the preset or progress
     strip, and the one action. The components' parts land in its cells. */
  .control {
    display: grid;
    grid-template-columns: 1fr 176px;
    grid-template-rows: auto 1fr var(--row) 64px;
    min-height: 0;
    border-right: var(--rule-thick) solid var(--rule);
  }
  .control > :global(:first-child),
  .control :global(.cells),
  .control :global(.track),
  .action {
    grid-column: 1 / -1;
  }
  .control > :global(.orc) {
    grid-column: 2;
    grid-row: 2;
  }
  .action {
    display: grid;
    border-top: var(--rule-thick) solid var(--rule);
  }
  .action.split {
    grid-template-columns: 1fr 160px;
  }
  .action.split > :global(button) {
    border-left: var(--rule-thin) solid var(--rule);
  }

  /* Right compartment: the target list, which alone scrolls. */
  .targets {
    display: grid;
    grid-template-rows: 28px 40px auto 1fr;
    min-height: 0;
  }
  .targets header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 var(--gutter);
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  h2 {
    margin: 0;
    font: inherit;
  }
  .prompt {
    display: grid;
    grid-template-columns: 28px 1fr auto;
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  .prompt label {
    display: grid;
    place-items: center;
    border-right: var(--rule-thin) solid var(--rule);
    font-weight: 700;
  }
  input {
    min-width: 0;
    padding: 0 var(--gutter);
    border: none;
    background: transparent;
    font: var(--type-data);
    letter-spacing: 0.04em;
    text-transform: uppercase;
    caret-color: var(--red);
    -webkit-user-select: text;
    user-select: text;
    outline: none;
  }
  input::placeholder {
    color: var(--ink-muted);
    opacity: 0.6;
  }
  .prompt:focus-within {
    box-shadow: inset 0 -2px 0 var(--red);
  }
  .prompt button {
    padding: 0 var(--gutter);
    border: none;
    border-left: var(--rule-thin) solid var(--rule);
    background: var(--ink);
    color: var(--paper);
    cursor: pointer;
  }
  .prompt button:disabled {
    background: transparent;
    color: var(--ink-muted);
    cursor: default;
  }
  .help {
    margin: 0;
    padding: 6px var(--gutter);
    border-bottom: var(--rule-thin) solid var(--rule);
    color: var(--ink-muted);
  }
  .help.error {
    color: var(--red);
  }
  .list {
    min-height: 0;
    overflow: hidden auto;
    overscroll-behavior: none;
  }
  ul {
    margin: 0;
    padding: 0;
    list-style: none;
  }
  .none {
    display: grid;
    place-items: center;
    height: 100%;
    margin: 0;
    background: repeating-linear-gradient(-45deg, var(--hatch) 0 1px, transparent 1px 8px);
  }
</style>
