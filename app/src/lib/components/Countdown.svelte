<script lang="ts">
  import { describeRemaining, formatClock, formatCountdown } from "../format";

  let {
    remaining,
    duration,
    endsAt,
    sites,
  }: { remaining: number; duration: number; endsAt: string; sites: number } = $props();

  const progress = $derived(duration > 0 ? Math.min(1, 1 - remaining / duration) : 1);
  const clock = $derived(formatCountdown(remaining));
  // Split off the seconds so the hours and minutes carry the weight.
  const main = $derived(clock.slice(0, clock.lastIndexOf(":")));
  const seconds = $derived(clock.slice(clock.lastIndexOf(":")));
  const announcement = $derived(describeRemaining(remaining));
</script>

<div class="countdown">
  <p class="micro muted">[ TIME REMAINING ]</p>
  <p class="time tabular" aria-hidden="true">
    {main}<span class="seconds">{seconds}</span>
  </p>
  <p class="visually-hidden" aria-live="polite">{announcement}</p>
  <p class="micro">ENDS {formatClock(endsAt)} <span class="muted">// {sites} TARGETS</span></p>
</div>

<div
  class="track"
  role="progressbar"
  aria-label="Block progress"
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={Math.round(progress * 100)}
>
  <div class="fill" style:width="{progress * 100}%"></div>
  <span class="pct micro">{Math.round(progress * 100)}%</span>
</div>

<style>
  .countdown {
    display: grid;
    align-content: start;
    gap: 8px;
    padding: var(--gutter);
  }
  p {
    margin: 0;
  }
  .time {
    margin: 4px 0 2px -3px;
    font: var(--type-macro);
    letter-spacing: var(--tracking-macro);
  }
  .seconds {
    font: var(--type-headline);
    letter-spacing: var(--tracking-macro);
    color: var(--red);
  }
  .track {
    position: relative;
    height: var(--row);
    border-top: var(--rule-thin) solid var(--rule);
    background: repeating-linear-gradient(
      90deg,
      transparent 0 calc(10% - 1px),
      var(--hatch) calc(10% - 1px) 10%
    );
  }
  .fill {
    height: 100%;
    background: var(--red);
    transition: width 1s linear;
  }
  .pct {
    position: absolute;
    right: var(--gutter);
    top: 50%;
    transform: translateY(-50%);
    mix-blend-mode: difference;
    color: var(--paper);
  }
</style>
