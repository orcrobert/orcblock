<script lang="ts">
  import { formatDay, potionsLabel } from "../format";
  import type { Potions } from "../api";
  import Icon from "./Icon.svelte";

  let { potions }: { potions: Potions } = $props();
</script>

<div class="shelf">
  <div class="flasks" aria-hidden="true">
    {#each { length: potions.per_month } as _, i (i)}
      <span class:full={i < potions.available}>
        <Icon name="flask" size={22} outlined={i >= potions.available} />
      </span>
    {/each}
  </div>
  <p>
    {potionsLabel(potions.available)}
    {#if potions.available < potions.per_month}
      <span class="refill">· more on {formatDay(potions.refill_on)}</span>
    {/if}
  </p>
</div>

<style>
  .shelf {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
  }
  .flasks {
    display: flex;
    gap: var(--space-2);
    color: var(--md-sys-color-outline);
  }
  .full {
    color: var(--md-sys-color-tertiary);
  }
  p {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-body-medium);
    text-align: center;
  }
  .refill {
    white-space: nowrap;
  }
</style>
