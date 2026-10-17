<script lang="ts">
  import { describeRemaining, formatClock, formatCountdown } from "../format";

  let { remaining, duration, endsAt }: { remaining: number; duration: number; endsAt: string } =
    $props();

  const progress = $derived(duration > 0 ? 1 - remaining / duration : 1);
  // Announced at most once a minute, not every second.
  const announcement = $derived(describeRemaining(remaining));
</script>

<div class="countdown">
  <p class="time" aria-hidden="true">{formatCountdown(remaining)}</p>
  <p class="visually-hidden" aria-live="polite">{announcement}</p>
  <div
    class="track"
    role="progressbar"
    aria-label="Block progress"
    aria-valuemin="0"
    aria-valuemax="100"
    aria-valuenow={Math.round(progress * 100)}
  >
    <div class="bar" style:transform="scaleX({progress})"></div>
  </div>
  <p class="ends">Ends at {formatClock(endsAt)}</p>
</div>

<style>
  .countdown {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    width: 100%;
  }
  .time {
    margin: 0;
    font: var(--md-sys-typescale-display-large);
    font-variant-numeric: tabular-nums;
    letter-spacing: -0.5px;
  }
  .track {
    width: min(100%, 280px);
    height: 4px;
    border-radius: var(--md-sys-shape-corner-full);
    background: var(--md-sys-color-surface-container-highest);
    overflow: hidden;
  }
  .bar {
    height: 100%;
    background: var(--md-sys-color-primary);
    transform-origin: left;
    transition: transform 1s linear;
  }
  .ends {
    margin: 0;
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-body-medium);
  }
</style>
