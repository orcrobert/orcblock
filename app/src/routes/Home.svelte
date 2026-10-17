<script lang="ts">
  import { api, errorMessage, type Site, type Status } from "../lib/api";
  import Button from "../lib/components/Button.svelte";
  import Card from "../lib/components/Card.svelte";
  import Chip from "../lib/components/Chip.svelte";
  import Countdown from "../lib/components/Countdown.svelte";
  import Dialog from "../lib/components/Dialog.svelte";
  import PotionShelf from "../lib/components/PotionShelf.svelte";
  import SiteRow from "../lib/components/SiteRow.svelte";
  import TextField from "../lib/components/TextField.svelte";
  import { MAX_MINUTES, MIN_MINUTES, formatMinutes, parseMinutes } from "../lib/format";
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

  const PRESETS = [25, 60, 120, 240];

  let site = $state("");
  let siteError = $state("");
  let preset = $state<number | "custom">(60);
  let custom = $state("");
  let confirming = $state(false);
  let busy = $state(false);

  const block = $derived(status.block);
  const customMinutes = $derived(parseMinutes(custom));
  const customError = $derived.by(() => {
    if (preset !== "custom" || custom.trim() === "") return "";
    if (customMinutes === null) return "Try 45, 90m or 1h30m";
    if (customMinutes < MIN_MINUTES) return "A block lasts at least one minute";
    if (customMinutes > MAX_MINUTES) return "A block lasts at most seven days";
    return "";
  });
  const minutes = $derived(preset === "custom" ? (customError ? null : customMinutes) : preset);

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

  function start() {
    if (minutes !== null) void run(() => api.startBlock(minutes));
  }

  async function drink() {
    if (await run(api.drinkPotion)) confirming = false;
  }
</script>

<main>
  <div class="hero">
    <Orc state={block ? "guarding" : "sleeping"} size={168} />
    <h2>{block ? "None shall pass" : "The path is open"}</h2>
    {#if block}
      <Countdown
        remaining={block.remaining_secs}
        duration={block.duration_secs}
        endsAt={block.ends_at}
      />
    {:else}
      <p class="lede">Add the sites that steal your time, then set the orc on guard.</p>
    {/if}
  </div>

  <Card title="Sites">
    <form onsubmit={addSite}>
      <TextField
        label="Add a site"
        bind:value={site}
        error={siteError}
        supporting={block ? "New sites join the running block" : "Paste a link or type a name"}
        inputmode="url"
        oninput={() => (siteError = "")}
      />
      <Button type="submit" variant="tonal" icon="add" disabled={site.trim() === ""}>Add</Button>
    </form>
    {#if status.sites.length > 0}
      <ul>
        {#each status.sites as entry (entry.domain)}
          <SiteRow site={entry} locked={block !== null} onremove={removeSite} />
        {/each}
      </ul>
    {:else}
      <p class="empty">No sites yet.</p>
    {/if}
  </Card>

  {#if block}
    <Card title="Sleep potions">
      <PotionShelf potions={status.potions} />
      <div class="action">
        <Button
          variant="tonal"
          icon="flask"
          disabled={busy || status.potions.available === 0}
          onclick={() => (confirming = true)}
        >
          Drink a sleep potion
        </Button>
      </div>
    </Card>
  {:else}
    <Card title="Duration">
      <div class="chips" role="group" aria-label="Duration">
        {#each PRESETS as value (value)}
          <Chip selected={preset === value} onclick={() => (preset = value)}
            >{formatMinutes(value)}</Chip
          >
        {/each}
        <Chip selected={preset === "custom"} onclick={() => (preset = "custom")}>Custom</Chip>
      </div>
      {#if preset === "custom"}
        <div class="custom">
          <TextField
            label="Minutes"
            bind:value={custom}
            error={customError}
            supporting={customMinutes ? formatMinutes(customMinutes) : "Like 45, 90m or 1h30m"}
          />
        </div>
      {/if}
      <div class="action">
        <Button
          icon="shield"
          disabled={busy || minutes === null || status.sites.length === 0}
          onclick={start}
        >
          Start blocking
        </Button>
      </div>
    </Card>
  {/if}
</main>

<Dialog
  bind:open={confirming}
  title="Use 1 of {status.potions.available} potions to end this block now?"
>
  The orc falls asleep and the sites open again straight away.
  {#snippet actions()}
    <Button variant="text" onclick={() => (confirming = false)}>Keep blocking</Button>
    <Button variant="text" disabled={busy} onclick={drink}>Drink</Button>
  {/snippet}
</Dialog>

<style>
  main {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-width: 480px;
    margin: 0 auto;
    padding: 0 var(--space-4) var(--space-6);
  }
  .hero {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) 0 var(--space-3);
    text-align: center;
  }
  h2 {
    margin: 0;
    font: var(--md-sys-typescale-headline-small);
  }
  .lede,
  .empty {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-body-medium);
  }
  .empty {
    padding: var(--space-3) var(--space-1) 0;
  }
  form {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
  }
  form :global(.field) {
    flex: 1;
  }
  form :global(button) {
    margin-top: var(--space-2);
  }
  ul {
    margin: var(--space-2) 0 0;
    padding: 0;
    list-style: none;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
  }
  .custom {
    margin-top: var(--space-3);
  }
  .action {
    display: flex;
    justify-content: center;
    margin-top: var(--space-4);
  }
</style>
