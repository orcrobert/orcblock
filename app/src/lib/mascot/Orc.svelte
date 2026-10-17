<script lang="ts" module>
  export type OrcState = "sleeping" | "guarding";
</script>

<script lang="ts">
  import Guarding from "./art/Guarding.svelte";
  import Sleeping from "./art/Sleeping.svelte";

  let { state, size = 200 }: { state: OrcState; size?: number } = $props();
</script>

<div class="orc" style:width="{size}px" style:height="{size}px" aria-hidden="true">
  {#if state === "guarding"}
    <Guarding />
  {:else}
    <Sleeping />
  {/if}
</div>

<style>
  .orc {
    flex: none;
    max-width: 60vw;
    max-height: 60vw;
  }
  .orc :global(svg) {
    display: block;
    width: 100%;
    height: 100%;
    overflow: visible;
  }
  @media (prefers-reduced-motion: reduce) {
    .orc :global(*) {
      animation: none !important;
    }
  }
</style>
