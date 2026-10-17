<script lang="ts">
  import { DURATION_STOPS, durationParts, formatEnds, formatMinutes } from "../format";
  import Slider from "./Slider.svelte";

  let { minutes = $bindable(60) }: { minutes?: number } = $props();

  const last = DURATION_STOPS.length - 1;
  let index = $state(
    Math.max(0, DURATION_STOPS.indexOf(minutes as (typeof DURATION_STOPS)[number])),
  );
  let now = $state(new Date());

  $effect(() => {
    minutes = DURATION_STOPS[index] ?? DURATION_STOPS[0];
  });

  $effect(() => {
    const timer = setInterval(() => (now = new Date()), 15_000);
    return () => clearInterval(timer);
  });
</script>

<div class="picker">
  <p class="readout tabular" aria-hidden="true">
    {#each durationParts(minutes) as part (part.unit)}
      <span class="value">{part.value}</span><span class="unit">{part.unit}</span>
    {/each}
  </p>
  <p class="ends">Ends at {formatEnds(now, minutes)}</p>
  <Slider
    bind:value={index}
    min={0}
    max={last}
    label="Duration"
    valuetext={formatMinutes(minutes)}
  />
  <div class="scale" aria-hidden="true">
    <span>{formatMinutes(DURATION_STOPS[0])}</span>
    <span>{formatMinutes(DURATION_STOPS[last] ?? 0)}</span>
  </div>
</div>

<style>
  .readout {
    display: flex;
    align-items: baseline;
    gap: var(--space-1);
    margin: 0;
    color: var(--md-sys-color-on-surface);
  }
  .value {
    font: var(--md-sys-typescale-display-medium);
    letter-spacing: -0.5px;
  }
  .unit {
    margin-right: var(--space-3);
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-title-large);
  }
  .ends {
    margin: var(--space-1) 0 var(--space-3);
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-body-medium);
  }
  .scale {
    display: flex;
    justify-content: space-between;
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-label-medium);
  }
</style>
