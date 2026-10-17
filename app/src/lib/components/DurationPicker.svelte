<script lang="ts">
  import {
    PRESETS,
    formatEnds,
    formatHoursMinutes,
    formatMinutes,
    parseDuration,
    presetLabel,
    stepDuration,
  } from "../format";

  let { minutes = $bindable(60) }: { minutes?: number } = $props();

  let now = $state(new Date());
  $effect(() => {
    const timer = setInterval(() => (now = new Date()), 15_000);
    return () => clearInterval(timer);
  });

  let editing = $state(false);
  let draft = $state("");
  const parsed = $derived(parseDuration(draft));
  const invalid = $derived(editing && draft.trim() !== "" && parsed === undefined);
  const shown = $derived(editing ? (parsed ?? minutes) : minutes);

  function edit() {
    draft = formatHoursMinutes(minutes);
    editing = true;
  }

  function commit() {
    if (!editing) return;
    if (parsed !== undefined) minutes = parsed;
    editing = false;
  }

  function onkeydown(event: KeyboardEvent) {
    if (event.key === "Enter") {
      event.preventDefault();
      if (parsed !== undefined) commit();
    } else if (event.key === "Escape") {
      event.preventDefault();
      editing = false;
    }
  }

  function focusAndSelect(node: HTMLInputElement) {
    node.focus();
    node.select();
  }
</script>

<div class="picker">
  <p class="micro muted">[ DURATION ]</p>
  {#if editing}
    <input
      class="readout tabular"
      class:long={draft.trim().length > 5}
      class:invalid
      bind:value={draft}
      use:focusAndSelect
      {onkeydown}
      onblur={commit}
      maxlength="8"
      spellcheck="false"
      autocomplete="off"
      aria-label="Custom duration"
      aria-invalid={invalid}
      aria-describedby="duration-hint"
    />
    <p id="duration-hint" class="micro" class:alert={invalid}>
      {#if invalid}
        OUT OF RANGE <span class="muted">// 1M TO 7D</span>
      {:else}
        ENDS {formatEnds(now, shown)} <span class="muted">// ENTER SET // ESC</span>
      {/if}
    </p>
  {:else}
    <button
      type="button"
      class="readout tabular"
      class:long={formatHoursMinutes(minutes).length > 5}
      ondblclick={edit}
      onkeydown={(event) => event.key === "Enter" && (event.preventDefault(), edit())}
      aria-label="Duration {formatMinutes(minutes)}. Press Enter to type a custom duration."
      title="Double-click to type a duration"
      ><output aria-live="polite">{formatHoursMinutes(minutes)}</output></button
    >
    <p class="micro">
      ENDS {formatEnds(now, minutes)} <span class="muted hint">// DBL-CLICK TO TYPE</span>
    </p>
  {/if}
</div>

<div class="cells" role="group" aria-label="Duration">
  <button type="button" aria-label="Shorter" onclick={() => (minutes = stepDuration(minutes, -1))}
    >−</button
  >
  {#each PRESETS as preset (preset)}
    <button
      type="button"
      class:on={minutes === preset}
      aria-pressed={minutes === preset}
      aria-label={formatMinutes(preset)}
      onclick={() => (minutes = preset)}>{presetLabel(preset)}</button
    >
  {/each}
  <button type="button" aria-label="Longer" onclick={() => (minutes = stepDuration(minutes, 1))}
    >+</button
  >
</div>

<style>
  .picker {
    display: grid;
    align-content: start;
    gap: 8px;
    padding: var(--gutter);
  }
  p {
    margin: 0;
  }
  .readout {
    display: block;
    width: 100%;
    margin: 4px 0 2px -3px;
    padding: 0;
    border: none;
    border-bottom: var(--rule-thick) solid transparent;
    background: none;
    color: var(--ink);
    font: var(--type-macro);
    letter-spacing: var(--tracking-macro);
    text-align: left;
    cursor: text;
  }
  /* Seven-day blocks read 168:00; shrink so they stay inside the column. */
  .readout.long {
    font-size: 0.78em;
  }
  button.readout:hover {
    border-bottom-color: var(--ink);
  }
  input.readout {
    border-bottom-color: var(--red);
    caret-color: var(--red);
    outline: none;
    -webkit-user-select: text;
    user-select: text;
  }
  input.readout::selection {
    background: var(--ink);
    color: var(--paper);
  }
  input.readout.invalid {
    color: var(--red);
  }
  .alert {
    color: var(--red);
  }
  .hint {
    opacity: 0;
    transition: opacity var(--motion-fast);
  }
  .picker:hover .hint,
  button.readout:focus-visible + p .hint {
    opacity: 1;
  }
  .cells {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: var(--rule-thin);
    border-top: var(--rule-thin) solid var(--rule);
    background: var(--rule);
  }
  .cells button {
    height: var(--row);
    padding: 0;
    border: none;
    background: var(--paper);
    font: var(--type-data);
    letter-spacing: 0.04em;
    cursor: pointer;
  }
  .cells button:hover {
    background: var(--ink);
    color: var(--paper);
  }
  .cells .on {
    background: var(--ink);
    color: var(--paper);
  }
</style>
