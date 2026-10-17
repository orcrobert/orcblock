<script lang="ts">
  import { formatDay, potionsSummary } from "../format";
  import type { Potions } from "../api";

  let { potions }: { potions: Potions } = $props();
</script>

<div class="shelf">
  <div class="meter" aria-hidden="true">
    {#each { length: potions.per_month } as _, i (i)}
      <span class:full={i < potions.available}></span>
    {/each}
  </div>
  <p>
    {potionsSummary(potions.available, potions.per_month)}
    {#if potions.available < potions.per_month}
      <span class="refill">· more on {formatDay(potions.refill_on)}</span>
    {/if}
  </p>
</div>

<style>
  .shelf {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }
  .meter {
    display: flex;
    gap: var(--space-1);
  }
  .meter span {
    width: 16px;
    height: 6px;
    border-radius: var(--md-sys-shape-corner-full);
    background: var(--md-sys-color-outline-variant);
  }
  .meter .full {
    background: var(--md-sys-color-primary);
  }
  p {
    margin: 0;
    color: var(--md-sys-color-on-surface);
    font: var(--md-sys-typescale-body-medium);
  }
  .refill {
    color: var(--md-sys-color-on-surface-variant);
  }
</style>
