<script lang="ts">
  import { VIAL_EMPTY, VIAL_FULL } from "./frames";
  import { fillOf, pixelRuns } from "./pixels";

  let { full, scale = 3 }: { full: boolean; scale?: number } = $props();

  const runs = $derived(pixelRuns(full ? VIAL_FULL : VIAL_EMPTY));
  const width = (VIAL_FULL[0] ?? "").length;
  const height = VIAL_FULL.length;
</script>

<svg
  width={width * scale}
  height={height * scale}
  viewBox="0 0 {width} {height}"
  preserveAspectRatio="none"
  shape-rendering="crispEdges"
  aria-hidden="true"
>
  {#each runs as run, i (i)}
    <rect x={run.x} y={run.y} width={run.width} height="1" fill={fillOf(run.pixel)} />
  {/each}
</svg>

<style>
  svg {
    display: block;
    width: 100%;
    height: 100%;
  }
</style>
