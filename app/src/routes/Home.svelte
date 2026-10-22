<script lang="ts">
  import { tick } from "svelte";
  import { getCurrentWebview } from "@tauri-apps/api/webview";
  import {
    api,
    errorMessage,
    type App,
    type InstalledApp,
    type Site,
    type Status,
  } from "../lib/api";
  import AppPicker from "../lib/components/AppPicker.svelte";
  import AppRow from "../lib/components/AppRow.svelte";
  import Button from "../lib/components/Button.svelte";
  import Countdown from "../lib/components/Countdown.svelte";
  import Dialog from "../lib/components/Dialog.svelte";
  import DurationPicker from "../lib/components/DurationPicker.svelte";
  import ListSwitcher from "../lib/components/ListSwitcher.svelte";
  import OrcBox from "../lib/components/OrcBox.svelte";
  import PotionShelf from "../lib/components/PotionShelf.svelte";
  import SiteRow from "../lib/components/SiteRow.svelte";
  import StatusBar from "../lib/components/StatusBar.svelte";
  import { DEFAULT_MINUTES } from "../lib/format";
  import {
    armLabel,
    chosenLists,
    currentList,
    loadCurrent,
    loadSkipped,
    saveCurrent,
    saveSkipped,
    targetCount,
  } from "../lib/lists";

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
  let picking = $state(false);
  let dragging = $state(false);
  /** Names of listed apps that are open, shown before a block starts. */
  let closing = $state<string[]>([]);
  let warning = $state(false);
  /** How many open apps the Start warning names before "+N MORE". */
  const MAX_NAMED = 5;
  let orcBox: OrcBox | undefined = $state();
  let control: HTMLElement | undefined = $state();
  let busy = $state(false);
  let remembered = $state(loadCurrent());
  let skipped = $state(loadSkipped());

  const block = $derived(status.block);
  const list = $derived(currentList(status.lists, remembered));
  const chosen = $derived(chosenLists(status.lists, skipped));
  const blockedLists = $derived(block?.lists ?? []);
  const locked = $derived(blockedLists.includes(list.id));
  const empty = $derived(list.sites.length === 0 && list.apps.length === 0);
  const listed = $derived(new Set(list.apps.map((app) => app.bundle_id)));

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
      onstatus(await api.addSite(list.id, site));
      site = "";
      siteError = "";
    } catch (err) {
      siteError = errorMessage(err);
    }
  }

  function removeSite(entry: Site) {
    void run(() => api.removeSite(list.id, entry.domain));
  }

  function removeApp(entry: App) {
    void run(() => api.removeApp(list.id, entry.bundle_id));
  }

  function selectList(id: number) {
    remembered = id;
    saveCurrent(id);
  }

  function toggleList(id: number) {
    const next = new Set(skipped);
    if (!next.delete(id)) next.add(id);
    skipped = next;
    saveSkipped(next, status.lists);
  }

  async function createList(name: string): Promise<string | null> {
    try {
      const next = await api.createList(name);
      onstatus(next);
      const created = next.lists.at(-1);
      if (created) selectList(created.id);
      return null;
    } catch (err) {
      return errorMessage(err);
    }
  }

  async function renameList(name: string): Promise<string | null> {
    try {
      onstatus(await api.renameList(list.id, name));
      return null;
    } catch (err) {
      return errorMessage(err);
    }
  }

  async function deleteList() {
    const id = list.id;
    await run(() => api.deleteList(id));
  }

  async function addApp(app: InstalledApp): Promise<string | null> {
    try {
      onstatus(await api.addApp(list.id, app.bundle_id, app.name));
      return null;
    } catch (err) {
      return errorMessage(err);
    }
  }

  /** Starts the block, or first lists the listed apps it will close. */
  async function arm() {
    const apps = [
      ...new Map(chosen.flatMap((entry) => entry.apps).map((app) => [app.bundle_id, app])).values(),
    ];
    if (apps.length > 0) {
      busy = true;
      // Without the list the block still starts; the warning is a courtesy.
      const open = new Set(await api.runningApps().catch(() => []));
      busy = false;
      const names = apps.filter((app) => open.has(app.bundle_id)).map((app) => app.name);
      if (names.length > 0) {
        closing = names;
        warning = true;
        return;
      }
    }
    await start();
  }

  async function start() {
    // Closed first, so an error shows in the snackbar, not under the modal.
    warning = false;
    await run(() =>
      api.startBlock(
        chosen.map((entry) => entry.id),
        minutes,
      ),
    );
  }

  async function dropApps(paths: string[]) {
    for (const path of paths.filter(isAppPath)) {
      try {
        const app = await api.appFromPath(path);
        const failed = await addApp(app);
        if (failed) onerror(failed);
      } catch (err) {
        onerror(errorMessage(err));
      }
    }
  }

  function isAppPath(path: string): boolean {
    return /\.app\/?$/i.test(path);
  }

  $effect(() => {
    let unlisten: (() => void) | undefined;
    let current = true;
    void getCurrentWebview()
      .onDragDropEvent(({ payload }) => {
        // A dialog covers the list; a drop there would change it unseen.
        const covered = picking || warning || confirming;
        if (payload.type === "enter") dragging = !covered && payload.paths.some(isAppPath);
        else if (payload.type === "leave") dragging = false;
        else if (payload.type === "drop") {
          dragging = false;
          if (covered) return;
          if (payload.paths.some(isAppPath)) void dropApps(payload.paths);
          else onerror("That is not an app; drop an app from Applications or the Dock");
        }
      })
      .then((stop) => {
        if (current) unlisten = stop;
        else stop();
      })
      .catch(() => {});
    return () => {
      current = false;
      unlisten?.();
    };
  });

  async function drink() {
    const used = status.potions.available - 1;
    if (await run(api.drinkPotion)) {
      confirming = false;
      orcBox?.drink(used);
      // The dialog would return focus to the potion button, which is gone
      // now that the block has ended.
      await tick();
      control?.focus();
    }
  }
</script>

<div class="screen">
  <section
    class="control"
    bind:this={control}
    tabindex="-1"
    aria-label={block ? "Block in progress" : "New block"}
  >
    <StatusBar armed={block !== null} label={block ? "ARMED" : "IDLE"} mode={status.mode} />
    {#if block}
      <Countdown
        remaining={block.remaining_secs}
        duration={block.duration_secs}
        endsAt={block.ends_at}
        sites={block.domains.length + block.apps.length}
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
        <Button disabled={busy || targetCount(chosen) === 0} onclick={arm}>
          <span class="label">{armLabel(status.lists, chosen)}</span><span>&gt;&gt;&gt;</span>
        </Button>
      {/if}
    </footer>
  </section>

  <section class="targets" class:dragging aria-labelledby="targets-title">
    <ListSwitcher
      lists={status.lists}
      current={list}
      {skipped}
      blocked={blockedLists}
      onselect={selectList}
      ontoggle={toggleList}
      oncreate={createList}
      onrename={renameList}
      ondelete={deleteList}
    />

    <form class="prompt" onsubmit={addSite}>
      <span class="micro" aria-hidden="true">&gt;</span>
      <input
        id="site"
        bind:value={site}
        aria-label="Add a site"
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
      <button type="button" class="micro app" onclick={() => (picking = true)}>+ APP</button>
    </form>
    <p
      id="site-help"
      class="help micro"
      class:error={siteError}
      role={siteError ? "alert" : undefined}
    >
      {siteError ||
        (dragging
          ? "DROP TO ADD THE APP"
          : locked
            ? "NEW TARGETS ARE BLOCKED AT ONCE"
            : block
              ? "THIS LIST IS NOT IN THE RUNNING BLOCK"
              : "PASTE A LINK, TYPE A NAME OR DROP AN APP")}
    </p>

    <div class="list">
      {#if empty}
        <p class="none micro muted">NO TARGETS // ADD A SITE OR AN APP ABOVE</p>
      {:else}
        <ul>
          {#each list.sites as entry, i (entry.domain)}
            <SiteRow site={entry} index={i + 1} {locked} onremove={removeSite} />
          {/each}
          {#each list.apps as entry, i (entry.bundle_id)}
            <AppRow app={entry} index={list.sites.length + i + 1} {locked} onremove={removeApp} />
          {/each}
        </ul>
      {/if}
    </div>
  </section>
</div>

<AppPicker bind:open={picking} {listed} onadd={addApp} />

<Dialog bind:open={warning} tag="CONFIRM // CLOSE APPS" title="Close open apps?">
  {closing.slice(0, MAX_NAMED).join(", ")}{closing.length > MAX_NAMED
    ? ` +${closing.length - MAX_NAMED} MORE`
    : ""}
  {closing.length === 1 ? "WILL" : "WILL ALL"} BE CLOSED. SAVE YOUR WORK.
  {#snippet actions()}
    <Button variant="plain" onclick={() => (warning = false)}>ABORT</Button>
    <Button disabled={busy} onclick={start}>
      <span>CLOSE + ARM</span><span>&gt;&gt;&gt;</span>
    </Button>
  {/snippet}
</Dialog>

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
    outline: none;
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
    container: targets / inline-size;
    display: grid;
    grid-template-rows: 28px 40px auto 1fr;
    min-height: 0;
  }
  .action .label {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .prompt {
    display: grid;
    grid-template-columns: 28px 1fr auto auto;
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  .prompt > span {
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
    opacity: 1;
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
  .prompt button.app {
    background: transparent;
    color: var(--ink);
  }
  .prompt button.app:hover {
    background: var(--ink);
    color: var(--paper);
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
  .targets.dragging .list {
    outline: var(--rule-thick) dashed var(--red);
    outline-offset: -4px;
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
