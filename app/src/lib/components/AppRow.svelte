<script lang="ts">
  import type { App } from "../api";
  import { formatIsoDay, pad2 } from "../format";
  import AppIcon from "./AppIcon.svelte";

  let {
    app,
    index,
    locked,
    onremove,
  }: { app: App; index: number; locked: boolean; onremove: (app: App) => void } = $props();
</script>

<li>
  <span class="index micro muted">{pad2(index)}</span>
  <AppIcon bundleId={app.bundle_id} name={app.name} size={16} />
  <span class="name" title="{app.name} ({app.bundle_id})">{app.name}</span>
  <span class="kind micro muted">APP</span>
  {#if locked}
    <span class="state micro locked">LOCKED</span>
  {:else}
    <span class="state date micro muted">{formatIsoDay(app.added_at)}</span>
    <button type="button" class="micro" aria-label="Remove {app.name}" onclick={() => onremove(app)}
      >[ × ]</button
    >
  {/if}
</li>

<style>
  li {
    display: grid;
    grid-template-columns: 28px 16px 1fr auto auto auto;
    align-items: center;
    gap: 10px;
    height: var(--row);
    padding-left: var(--gutter);
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  .name {
    overflow: hidden;
    font: var(--type-data);
    letter-spacing: 0.02em;
    text-transform: uppercase;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .kind {
    padding: 1px 4px;
    border: var(--rule-thin) solid var(--ink-muted);
  }
  .state {
    padding-right: var(--gutter);
  }
  @container targets (max-width: 380px) {
    .date {
      display: none;
    }
  }
  .locked {
    color: var(--red);
  }
  button {
    align-self: stretch;
    padding: 0 var(--gutter);
    border: none;
    border-left: var(--rule-thin) solid var(--rule);
    background: transparent;
    cursor: pointer;
  }
  button:hover {
    background: var(--red);
    color: var(--on-red);
  }
</style>
