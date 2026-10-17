<script lang="ts">
  import type { Potions } from "../api";
  import Vial from "../mascot/Vial.svelte";

  let {
    potions,
    away = -1,
  }: {
    potions: Potions;
    /** The slot whose vial is off the shelf being drunk; drawn as a gap. */
    away?: number;
  } = $props();
</script>

<div
  class="vials"
  role="img"
  aria-label="{potions.available} of {potions.per_month} sleep potions left"
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
