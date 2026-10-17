<script lang="ts">
  import { api, errorMessage, type Site, type Status } from "../lib/api";
  import Button from "../lib/components/Button.svelte";
  import Countdown from "../lib/components/Countdown.svelte";
  import Dialog from "../lib/components/Dialog.svelte";
  import DurationPicker from "../lib/components/DurationPicker.svelte";
  import IconButton from "../lib/components/IconButton.svelte";
  import PotionShelf from "../lib/components/PotionShelf.svelte";
  import SiteRow from "../lib/components/SiteRow.svelte";
  import TextField from "../lib/components/TextField.svelte";
  import { DEFAULT_MINUTES } from "../lib/format";
  import Orc from "../lib/mascot/Orc.svelte";

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
    if (await run(api.drinkPotion)) confirming = false;
  }
</script>

<main>
  <section class="panel" aria-label={block ? "Block in progress" : "New block"}>
    <p class="state" class:active={block}>
      <Orc state={block ? "guarding" : "sleeping"} size={20} />
      {block ? "Guarding" : "Idle"}
    </p>
    {#if block}
      <Countdown
        remaining={block.remaining_secs}
        duration={block.duration_secs}
        endsAt={block.ends_at}
        sites={block.domains.length}
      />
    {:else}
      <DurationPicker bind:minutes />
      <div class="start">
        <Button wide disabled={busy || empty} onclick={() => run(() => api.startBlock(minutes))}>
          Start blocking
        </Button>
        {#if empty}<p class="hint">Add a site below to start.</p>{/if}
      </div>
    {/if}
  </section>

  <section aria-labelledby="sites-title">
    <h2 id="sites-title">
      Sites{#if !empty}<span class="count">{status.sites.length}</span>{/if}
    </h2>
    <form onsubmit={addSite}>
      <TextField
        label="Add a site"
        bind:value={site}
        error={siteError}
        supporting={block
          ? "New sites are blocked straight away"
          : "Paste a link or type a name, like youtube.com"}
        inputmode="url"
        oninput={() => (siteError = "")}
      >
        {#snippet trailing()}
          <IconButton icon="add" label="Add site" type="submit" disabled={site.trim() === ""} />
        {/snippet}
      </TextField>
    </form>
    {#if !empty}
      <ul>
        {#each status.sites as entry (entry.domain)}
          <SiteRow site={entry} locked={block !== null} onremove={removeSite} />
        {/each}
      </ul>
    {/if}
  </section>

  {#if block}
    <section aria-labelledby="potions-title">
      <h2 id="potions-title">Sleep potions</h2>
      <div class="potions">
        <PotionShelf potions={status.potions} />
        <Button
          wide
          variant="tonal"
          disabled={busy || status.potions.available === 0}
          onclick={() => (confirming = true)}
        >
          End early with a potion
        </Button>
      </div>
    </section>
  {/if}
</main>

<Dialog
  bind:open={confirming}
  title="Use 1 of {status.potions.available} potions to end this block now?"
>
  The sites open again straight away. Potions refill on the first of each month.
  {#snippet actions()}
    <Button variant="text" onclick={() => (confirming = false)}>Keep blocking</Button>
    <Button variant="text" disabled={busy} onclick={drink}>End block</Button>
  {/snippet}
</Dialog>

<style>
  main {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    max-width: 480px;
    margin: 0 auto;
    padding: var(--space-2) var(--space-4) var(--space-6);
  }
  .panel {
    padding: var(--space-5);
    border-radius: var(--md-sys-shape-corner-extra-large);
    background: var(--md-sys-color-surface-container-low);
  }
  .state {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0 0 var(--space-3);
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-label-large);
  }
  .state.active {
    color: var(--md-sys-color-primary);
  }
  .start {
    margin-top: var(--space-4);
  }
  .hint {
    margin: var(--space-2) 0 0;
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-body-small);
    text-align: center;
  }
  h2 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0 0 var(--space-3) var(--space-1);
    color: var(--md-sys-color-on-surface);
    font: var(--md-sys-typescale-title-medium);
  }
  .count {
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-label-large);
  }
  ul {
    margin: var(--space-2) 0 0;
    padding: 0;
    overflow: hidden;
    list-style: none;
    border-radius: var(--md-sys-shape-corner-large);
    background: var(--md-sys-color-surface-container-low);
  }
  .potions {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4);
    border-radius: var(--md-sys-shape-corner-large);
    background: var(--md-sys-color-surface-container-low);
  }
</style>
