<script lang="ts">
  import { tick } from "svelte";
  import type { List } from "../api";
  import { pad2 } from "../format";
  import { targetCount } from "../lists";
  import { PRESETS, type Preset } from "../presets";
  import Button from "./Button.svelte";
  import Dialog from "./Dialog.svelte";

  let {
    lists,
    current,
    skipped,
    blocked,
    onselect,
    ontoggle,
    oncreate,
    onrename,
    ondelete,
  }: {
    lists: List[];
    current: List;
    /** Lists left out of the next block. */
    skipped: ReadonlySet<number>;
    /** Lists in the running block; empty while idle. */
    blocked: readonly number[];
    onselect: (id: number) => void;
    ontoggle: (id: number) => void;
    /** Each resolves to an error to show in the dialog, or null when done. */
    oncreate: (name: string, sites: readonly string[]) => Promise<string | null>;
    onrename: (name: string) => Promise<string | null>;
    ondelete: () => Promise<void>;
  } = $props();

  const uid = $props.id();
  let open = $state(false);
  let naming = $state(false);
  let kind = $state<"create" | "rename">("create");
  let deleting = $state(false);
  let name = $state("");
  let preset = $state.raw<Preset | null>(null);
  let nameError = $state("");
  let busy = $state(false);
  let switcher: HTMLElement | undefined = $state();
  let toggle: HTMLButtonElement | undefined = $state();
  let input: HTMLInputElement | undefined = $state();

  let dialogWasOpen = false;

  const blocking = $derived(blocked.length > 0);
  const locked = $derived(blocked.includes(current.id));
  const several = $derived(lists.length > 1);

  function close() {
    open = false;
    toggle?.focus();
  }

  function pick(id: number) {
    onselect(id);
    close();
  }

  async function startNaming(next: "create" | "rename") {
    open = false;
    kind = next;
    name = next === "rename" ? current.name : "";
    preset = null;
    nameError = "";
    naming = true;
    await tick();
    input?.select();
  }

  async function submitName(event: SubmitEvent) {
    event.preventDefault();
    busy = true;
    const failed = await (kind === "create" ? oncreate(name, preset?.sites ?? []) : onrename(name));
    busy = false;
    if (failed) nameError = failed;
    else naming = false;
  }

  /** Picks what a new list starts with; a preset also names it unless named. */
  function startFrom(next: Preset | null) {
    const named =
      name.trim() !== "" &&
      !PRESETS.some((p) => p.name.toLowerCase() === name.trim().toLowerCase());
    if (!named) name = next?.name ?? "";
    preset = next;
  }

  async function confirmDelete() {
    busy = true;
    await ondelete();
    busy = false;
    deleting = false;
  }

  $effect(() => {
    const dialogOpen = naming || deleting;
    if (dialogWasOpen && !dialogOpen) toggle?.focus();
    dialogWasOpen = dialogOpen;
  });

  $effect(() => {
    if (!open) return;
    const outside = (event: Event) => {
      if (!switcher?.contains(event.target as Node)) open = false;
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") close();
    };
    window.addEventListener("pointerdown", outside);
    window.addEventListener("focusin", outside);
    window.addEventListener("keydown", escape);
    return () => {
      window.removeEventListener("pointerdown", outside);
      window.removeEventListener("focusin", outside);
      window.removeEventListener("keydown", escape);
    };
  });
</script>

<div class="switcher" bind:this={switcher}>
  <header class="micro">
    {#if several && !blocking}
      <button
        class="tick"
        role="checkbox"
        aria-checked={!skipped.has(current.id)}
        aria-label="Block {current.name} next time"
        onclick={() => ontoggle(current.id)}
      ></button>
    {/if}
    <h2>
      <button
        class="name"
        bind:this={toggle}
        aria-expanded={open}
        aria-controls="{uid}-menu"
        title={current.name}
        onclick={() => (open = !open)}
      >
        <span id="targets-title">[ {current.name} ]</span>
        <span aria-hidden="true">{open ? "▴" : "▾"}</span>
      </button>
    </h2>
    <span class="muted">{locked ? "LOCKED // " : ""}{pad2(targetCount([current]))}</span>
  </header>

  {#if open}
    <div class="menu micro" id="{uid}-menu">
      <ul aria-label="Lists">
        {#each lists as list (list.id)}
          {@const inBlock = blocked.includes(list.id)}
          <li class:current={list.id === current.id}>
            {#if blocking}
              <span class="state" class:red={inBlock}>{inBlock ? "■" : "□"}</span>
            {:else}
              <button
                class="tick"
                role="checkbox"
                aria-checked={!skipped.has(list.id)}
                aria-label="Block {list.name} next time"
                onclick={() => ontoggle(list.id)}
              ></button>
            {/if}
            <button
              class="pick"
              aria-current={list.id === current.id ? "true" : undefined}
              title={list.name}
              onclick={() => pick(list.id)}>{list.name}</button
            >
            <span class="muted">{inBlock ? "LOCKED" : pad2(targetCount([list]))}</span>
          </li>
        {/each}
      </ul>
      <p class="hint muted">
        {#if blocking}
          <span class="red">■</span> IN THE RUNNING BLOCK
        {:else}
          TICKED LISTS ARE BLOCKED ON START
        {/if}
      </p>
      <div class="actions">
        <button onclick={() => startNaming("create")}>+ NEW LIST</button>
        <button onclick={() => startNaming("rename")}>RENAME</button>
        <button
          disabled={!several || locked}
          onclick={() => {
            open = false;
            deleting = true;
          }}>DELETE</button
        >
      </div>
    </div>
  {/if}
</div>

<Dialog
  bind:open={naming}
  tag={kind === "create" ? "NEW // LIST" : "RENAME // LIST"}
  title={kind === "create" ? "Name the list" : `Rename ${current.name}`}
>
  <form id="{uid}-name" class="field" onsubmit={submitName}>
    <input
      bind:this={input}
      bind:value={name}
      aria-label="List name"
      placeholder="DEEP WORK"
      maxlength="40"
      autocomplete="off"
      spellcheck="false"
      aria-invalid={nameError ? "true" : undefined}
      aria-describedby="{uid}-name-error"
      oninput={() => (nameError = "")}
    />
    <p id="{uid}-name-error" class="error" role={nameError ? "alert" : undefined}>{nameError}</p>
    {#if kind === "create"}
      <div class="from micro" role="group" aria-label="Start from">
        <span class="muted">START FROM</span>
        <button
          type="button"
          disabled={busy}
          aria-pressed={preset === null}
          onclick={() => startFrom(null)}>EMPTY</button
        >
        {#each PRESETS as option (option.name)}
          <button
            type="button"
            disabled={busy}
            aria-pressed={preset === option}
            onclick={() => startFrom(option)}>{option.name}</button
          >
        {/each}
      </div>
      {#if preset}
        <p class="credit micro muted" title={preset.url}>
          {pad2(preset.sites.length)} SITES // BASED ON {preset.credit}
        </p>
      {/if}
    {/if}
  </form>
  {#snippet actions()}
    <Button variant="plain" onclick={() => (naming = false)}>ABORT</Button>
    <Button type="submit" form="{uid}-name" disabled={busy || name.trim() === ""}>
      <span>{kind === "create" ? "CREATE" : "RENAME"}</span><span>&gt;&gt;&gt;</span>
    </Button>
  {/snippet}
</Dialog>

<Dialog bind:open={deleting} tag="CONFIRM // DELETE LIST" title="Delete {current.name}?">
  {pad2(targetCount([current]))} TARGETS GO WITH IT. THIS CANNOT BE UNDONE.
  {#snippet actions()}
    <Button variant="plain" onclick={() => (deleting = false)}>ABORT</Button>
    <Button disabled={busy} onclick={confirmDelete}>
      <span>DELETE</span><span>&gt;&gt;&gt;</span>
    </Button>
  {/snippet}
</Dialog>

<style>
  .switcher {
    position: relative;
    z-index: 2;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 100%;
    padding: 0 var(--gutter);
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  header .muted {
    margin-left: auto;
  }
  h2 {
    display: flex;
    min-width: 0;
    margin: 0;
    font: inherit;
  }
  button {
    border: none;
    background: transparent;
    color: inherit;
    font: inherit;
    letter-spacing: inherit;
    text-transform: inherit;
    cursor: pointer;
  }
  button:focus-visible {
    outline: var(--rule-thick) solid var(--red);
    outline-offset: 1px;
  }
  .name {
    display: flex;
    gap: 8px;
    min-width: 0;
    padding: 0;
  }
  .name > span:first-child {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  /* A 24px hit area around a 12px box. */
  .tick {
    display: grid;
    flex: none;
    place-items: center;
    width: 24px;
    height: 24px;
    margin: 0 -6px;
    padding: 0;
  }
  .tick::before {
    width: 12px;
    height: 12px;
    box-sizing: border-box;
    border: var(--rule-thin) solid var(--ink);
    content: "";
  }
  .tick[aria-checked="true"]::before {
    background: var(--ink);
  }
  .menu {
    position: absolute;
    inset: 100% 0 auto;
    border-bottom: var(--rule-thick) solid var(--ink);
    background: var(--paper);
  }
  ul {
    max-height: 240px;
    margin: 0;
    padding: 0;
    overflow: hidden auto;
    list-style: none;
  }
  li {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 32px;
    padding: 0 var(--gutter);
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  li.current {
    background: repeating-linear-gradient(-45deg, var(--hatch) 0 1px, transparent 1px 8px);
  }
  .pick {
    flex: 1;
    min-width: 0;
    height: 100%;
    padding: 0;
    overflow: hidden;
    text-align: left;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .state {
    width: 12px;
    text-align: center;
  }
  .state.red,
  .hint .red {
    color: var(--red);
  }
  .hint {
    margin: 0;
    padding: 6px var(--gutter);
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  .actions {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    height: 32px;
  }
  .actions button + button {
    border-left: var(--rule-thin) solid var(--rule);
  }
  .actions button:hover:not(:disabled) {
    background: var(--ink);
    color: var(--paper);
  }
  .actions button:disabled {
    background: repeating-linear-gradient(-45deg, var(--hatch) 0 1px, transparent 1px 8px);
    color: var(--ink-muted);
    cursor: default;
  }
  .field {
    display: grid;
    gap: 6px;
  }
  .field input {
    height: var(--row);
    padding: 0 10px;
    border: var(--rule-thin) solid var(--ink);
    background: transparent;
    color: var(--ink);
    font: var(--type-data);
    -webkit-user-select: text;
    user-select: text;
    outline: none;
  }
  .field input::placeholder {
    color: var(--ink-muted);
  }
  .field input:focus-visible {
    outline: var(--rule-thick) solid var(--red);
    outline-offset: 1px;
  }
  .error {
    margin: 0;
    color: var(--red);
  }
  .error:empty {
    display: none;
  }
  .from {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .from button {
    height: 28px;
    padding: 0 10px;
    border: var(--rule-thin) solid var(--ink);
  }
  .from button[aria-pressed="true"] {
    background: var(--ink);
    color: var(--paper);
  }
  .credit {
    margin: 0;
  }
</style>
