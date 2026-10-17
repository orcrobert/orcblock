<script lang="ts">
  import { describeRemaining, formatClock, formatCountdown } from "../format";

  let {
    remaining,
    duration,
    endsAt,
    sites,
  }: { remaining: number; duration: number; endsAt: string; sites: number } = $props();

  const progress = $derived(duration > 0 ? 1 - remaining / duration : 1);
  // Announced at most once a minute, not every second.
  const announcement = $derived(describeRemaining(remaining));
</script>

<div class="countdown">
  <p class="time tabular" aria-hidden="true">{formatCountdown(remaining)}</p>
  <p class="visually-hidden" aria-live="polite">{announcement}</p>
  <div
    class="track"
    role="progressbar"
    aria-label="Block progress"
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={Math.round(progress * 100)}
  >
    <div class="bar" style:width="{progress * 100}%"></div>
    <div class="rest"></div>
  </div>
  <p class="meta">
    Blocking {sites} site{sites === 1 ? "" : "s"} · until {formatClock(endsAt)}
  </p>
</div>

<style>
  .time {
    margin: 0;
    color: var(--md-sys-color-on-surface);
    font: var(--md-sys-typescale-display-large);
    letter-spacing: -1px;
  }
  .track {
    display: flex;
    gap: var(--space-1);
    height: 4px;
    margin: var(--space-4) 0 var(--space-3);
  }
  .bar,
  .rest {
    border-radius: var(--md-sys-shape-corner-full);
  }
  .bar {
    background: var(--md-sys-color-primary);
    transition: width var(--md-sys-motion-duration-medium) linear;
  }
  .rest {
    flex: 1;
    background: var(--md-sys-color-secondary-container);
  }
  .meta {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-body-medium);
  }
</style>
