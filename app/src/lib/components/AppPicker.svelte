<script lang="ts">
  import { api, errorMessage, type InstalledApp } from "../api";
  import { pickable } from "../picker";
  import AppIcon from "./AppIcon.svelte";
  import Button from "./Button.svelte";
  import Dialog from "./Dialog.svelte";

  let {
    open = $bindable(false),
    listed,
    onadd,
  }: {
    open?: boolean;
    /** Bundle IDs already on the list; they are not offered. */
    listed: ReadonlySet<string>;
    /** Adds the app; resolves to an error message, or `null` once added. */
    onadd: (app: InstalledApp) => Promise<string | null>;
  } = $props();

  let query = $state("");
  let error = $state("");
  let busy = $state(false);
  let apps = $state<InstalledApp[] | null>(null);
  let running = $state<ReadonlySet<string>>(new Set());
  let input: HTMLInputElement | undefined = $state();

  const offered = $derived(apps ? pickable(apps, query, listed, running) : []);
  const searching = $derived(query.trim() !== "");
  const openNow = $derived(searching ? [] : offered.filter((app) => running.has(app.bundle_id)));
  const rest = $derived(searching ? offered : offered.filter((app) => !running.has(app.bundle_id)));

  $effect(() => {
    if (!open) return;
    query = "";
    error = "";
    apps = null;
    running = new Set();
    // Every open asks again: apps open and quit between visits, and the
    // installed list is cached on the Rust side.
    void api
      .installedApps()
      .then((found) => (apps = found))
      .catch((err) => {
        apps = [];
        error = errorMessage(err);
      });
    void api
      .runningApps()
      .then((ids) => (running = new Set(ids)))
      .catch(() => (running = new Set()));
    queueMicrotask(() => input?.focus());
  });

  async function add(app: InstalledApp) {
    if (busy) return;
    busy = true;
    try {
      const failed = await onadd(app);
      if (failed) error = failed;
      else open = false;
    } finally {
      busy = false;
    }
  }

  async function browse() {
    if (busy) return;
    try {
      const app = await api.pickAppInFinder();
      if (app) await add(app);
    } catch (err) {
      error = errorMessage(err);
    }
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key !== "Enter") return;
    event.preventDefault();
    const top = openNow[0] ?? rest[0];
    if (top) void add(top);
    else error = "NO MATCH";
  }
</script>

{#snippet grid(label: string, list: InstalledApp[])}
  <section aria-label={label}>
    <h3 class="micro muted">[ {label} ]</h3>
    <ul>
      {#each list as app (app.bundle_id)}
        <li>
          <button
            type="button"
            title={app.bundle_id}
            aria-label="Add {app.name}"
            disabled={busy}
            onclick={() => add(app)}
          >
            <AppIcon bundleId={app.bundle_id} name={app.name} size={32} />
            <span class="name micro">{app.name}</span>
          </button>
        </li>
      {/each}
    </ul>
  </section>
{/snippet}

<Dialog bind:open wide tag="ADD // APP" title="Block an app">
  <label class="prompt">
    <span class="micro" aria-hidden="true">&gt;</span>
    <input
      bind:this={input}
      bind:value={query}
      aria-label="Filter apps"
      aria-describedby="picker-help"
      aria-invalid={error ? "true" : undefined}
      placeholder="TYPE TO FILTER // ENTER ADDS"
      autocomplete="off"
      autocapitalize="off"
      spellcheck="false"
      oninput={() => (error = "")}
      {onkeydown}
    />
  </label>
  <p id="picker-help" class="help" class:error role={error ? "alert" : undefined}>
    {error || (apps === null ? "LOOKING FOR APPS…" : `${offered.length} APPS`)}
  </p>
  <div class="apps">
    {#if apps !== null && offered.length === 0}
      <p class="none muted">NO MATCH // TRY BROWSE IN FINDER</p>
    {:else}
      {#if openNow.length > 0}
        {@render grid("OPEN NOW", openNow)}
      {/if}
      {#if rest.length > 0}
        {@render grid(searching ? "MATCHES" : "ALL APPS", rest)}
      {/if}
    {/if}
  </div>
  {#snippet actions()}
    <Button variant="plain" onclick={() => (open = false)}>CLOSE</Button>
    <Button variant="plain" disabled={busy} onclick={browse}>
      <span>BROWSE IN FINDER…</span><span>&gt;&gt;&gt;</span>
    </Button>
  {/snippet}
</Dialog>

<style>
  .prompt {
    display: grid;
    grid-template-columns: 28px 1fr;
    height: var(--row);
    border: var(--rule-thin) solid var(--rule);
  }
  .prompt > span {
    display: grid;
    place-items: center;
    border-right: var(--rule-thin) solid var(--rule);
    font-weight: 700;
  }
  .prompt:focus-within {
    box-shadow: inset 0 -2px 0 var(--red);
  }
  input {
    min-width: 0;
    padding: 0 var(--gutter);
    border: none;
    background: transparent;
    color: var(--ink);
    font: var(--type-data);
    letter-spacing: 0.04em;
    caret-color: var(--red);
    -webkit-user-select: text;
    user-select: text;
    outline: none;
  }
  input::placeholder {
    color: var(--ink-muted);
    opacity: 1;
  }
  .help {
    margin: 6px 0;
    color: var(--ink-muted);
  }
  .help.error {
    color: var(--red);
  }
  .apps {
    height: min(240px, calc(100vh - 300px));
    overflow: hidden auto;
    overscroll-behavior: none;
    border: var(--rule-thin) solid var(--rule);
  }
  h3 {
    margin: 0;
    padding: 6px 8px;
    font: inherit;
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  ul {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: grid;
  }
  li button {
    display: grid;
    justify-items: center;
    align-content: center;
    gap: 6px;
    height: 84px;
    padding: 6px;
    border: none;
    box-shadow:
      inset -1px 0 0 var(--hatch),
      inset 0 -1px 0 var(--hatch);
    background: transparent;
    color: var(--ink);
    cursor: pointer;
  }
  li button:hover {
    background: var(--ink);
    color: var(--paper);
  }
  li button:disabled {
    cursor: default;
  }
  .name {
    max-width: 100%;
    overflow: hidden;
    text-align: center;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .none {
    display: grid;
    place-items: center;
    height: 100%;
    margin: 0;
    background: repeating-linear-gradient(-45deg, var(--hatch) 0 1px, transparent 1px 8px);
  }
</style>
