<script lang="ts">
  import type { Potions } from "../api";
  import Vial from "../mascot/Vial.svelte";

  let {
    potions,
    away = -1,
    announce = true,
  }: {
    potions: Potions;
    /** The slot whose vial is off the shelf being drunk; drawn as a gap. */
    away?: number;
    /** False while the potion shelf already states the count in text. */
    announce?: boolean;
  } = $props();
</script>

<div
  class="vials"
  role={announce ? "img" : undefined}
  aria-hidden={announce ? undefined : "true"}
  aria-label={announce
    ? `${potions.available} of ${potions.per_month} sleep potions left`
    : undefined}
>
  {#each { length: potions.per_month } as _, i (i)}
    <span class="slot" class:away={i === away} data-slot={i}>
      <Vial full={i < potions.available} />
    </span>
  {/each}
</div>

<style>
  .vials {
    display: flex;
    justify-content: center;
    gap: 8px;
    padding: 8px 0 0;
  }
  .slot {
    display: block;
    width: 12px;
    height: 18px;
  }
  .away {
    visibility: hidden;
  }
</style>
