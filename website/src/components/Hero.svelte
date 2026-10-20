<script lang="ts">
  import { formatCountdown } from "$app/format";
  import Orc from "$app/mascot/Orc.svelte";
  import PotionVials from "$app/components/PotionVials.svelte";
  import { prefersReducedMotion } from "$app/motion";
  import type { Platform } from "../lib/platform";
  import HeroDownload from "./HeroDownload.svelte";

  let { platform }: { platform: Platform } = $props();

  const DURATION = 90 * 60;
  const QUIPS = [
    "BACK TO WORK.",
    "NOT TODAY.",
    "SLACK CAN WAIT.",
    "POKING ME WON'T HELP.",
    "I DON'T SLEEP. MOSTLY.",
    "GO WRITE SOMETHING.",
  ];

  let remaining = $state(DURATION - 1);
  let quip = $state<string | null>(null);
  let poke = $state(0);
  let shaking = $state(false);

  $effect(() => {
    if (prefersReducedMotion()) return;
    const timer = setInterval(() => {
      remaining = remaining <= 1 ? DURATION - 1 : remaining - 1;
    }, 1000);
    return () => clearInterval(timer);
  });

  $effect(() => {
    if (quip === null) return;
    const timer = setTimeout(() => (quip = null), 2200);
    return () => clearTimeout(timer);
  });

  function pokeOrc() {
    quip = QUIPS[poke % QUIPS.length] ?? null;
    poke += 1;
    shaking = false;
    requestAnimationFrame(() => (shaking = true));
  }

  const clock = $derived(formatCountdown(remaining));
  const progress = $derived(1 - remaining / DURATION);
</script>

<section class="band hero" id="top" aria-labelledby="hero-title">
  <div class="page">
    <div class="bar micro">
      <span>UNIT / ORC-01</span>
      <span class="muted wide">DISTRACTION BLOCKER // MACOS</span>
      <span class="status"><span class="light" aria-hidden="true"></span>STATUS: ARMED</span>
    </div>

    <div class="grid">
      <div class="copy">
        <p class="micro muted">[ 001 // THE PITCH ]</p>
        <h1 id="hero-title" class="headline">
          No way<br />out<span class="red">.</span>
        </h1>
        <p class="lead">
          Pick the sites and apps. Pick the time. An orc stands guard until it's over. Closing
          OrcBlock, quitting it or restarting your Mac changes nothing. The only way out early is a
          sleep potion, and you get five a month.
        </p>
        <HeroDownload {platform} />
      </div>

      <figure class="orc">
        <PotionVials potions={{ available: 5, per_month: 5, refill_on: "" }} />
        <button
          type="button"
          class="sprite"
          class:shaking
          onclick={pokeOrc}
          onanimationend={() => (shaking = false)}
          aria-label="Poke the orc"
        >
          <Orc pose="guarding" scale={8} />
        </button>
        {#if quip}
          <p class="quip micro" role="status">{quip}</p>
        {/if}
        <figcaption class="micro">
          ORC-01 // ON GUARD <span class="muted">// POKE HIM</span>
        </figcaption>
      </figure>
    </div>

    <div class="countdown">
      <p class="micro muted">[ TIME REMAINING ]</p>
      <p class="time tabular" aria-hidden="true">
        {clock.slice(0, clock.lastIndexOf(":"))}<span class="red seconds"
          >{clock.slice(clock.lastIndexOf(":"))}</span
        >
      </p>
      <p class="micro">ENDS WHEN IT ENDS <span class="muted">// 04 TARGETS</span></p>
    </div>
    <div class="track" aria-hidden="true">
      <div class="fill" style:width="{progress * 100}%"></div>
    </div>
  </div>
</section>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: var(--gutter);
    height: 28px;
    padding: 0 var(--gutter);
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  .status {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
    color: var(--red);
  }
  .light {
    width: 8px;
    height: 8px;
    background: var(--red);
    animation: blink 1.2s steps(1, end) infinite;
  }
  .grid {
    display: grid;
    grid-template-columns: 1fr minmax(260px, 340px);
  }
  .copy {
    display: grid;
    align-content: start;
    gap: 24px;
    padding: clamp(24px, 5vw, 56px) var(--pad);
  }
  .copy > p:first-child {
    margin: 0;
  }
  .headline {
    font-size: clamp(64px, 13vw, 168px);
  }
  .orc {
    position: relative;
    display: grid;
    grid-template-rows: auto 1fr auto;
    justify-items: center;
    align-items: end;
    margin: 0;
    border-left: var(--rule-thin) solid var(--rule);
    background: repeating-linear-gradient(-45deg, var(--hatch) 0 1px, transparent 1px 10px);
  }
  .orc::before,
  .orc::after {
    content: "+";
    position: absolute;
    top: 2px;
    font: var(--type-micro);
    color: var(--ink-muted);
  }
  .orc::before {
    left: 4px;
  }
  .orc::after {
    right: 4px;
  }
  .orc :global(.vials) {
    padding-top: 20px;
  }
  .sprite {
    padding: 24px 0 0;
    border: none;
    background: none;
    cursor: pointer;
  }
  .sprite:hover {
    translate: 0 -2px;
  }
  .shaking {
    animation: shake 320ms steps(6, end);
  }
  .quip {
    position: absolute;
    top: 40%;
    left: 50%;
    margin: 0;
    padding: 6px 10px;
    translate: -50% 0;
    white-space: nowrap;
    background: var(--ink);
    color: var(--paper);
    animation: pop 160ms steps(2, end);
  }
  figcaption {
    width: 100%;
    padding: 6px 0;
    border-top: var(--rule-thin) solid var(--rule);
    background: var(--paper);
    text-align: center;
  }
  .countdown {
    display: grid;
    gap: 8px;
    padding: var(--gutter) var(--pad);
    border-top: var(--rule-thick) solid var(--rule);
  }
  .countdown p {
    margin: 0;
  }
  .time {
    margin-left: -3px;
    font: var(--type-macro);
    font-size: clamp(64px, 14vw, 160px);
    letter-spacing: var(--tracking-macro);
  }
  .seconds {
    font: var(--type-headline);
    font-size: clamp(28px, 5vw, 56px);
    letter-spacing: var(--tracking-macro);
  }
  .track {
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
    transition: width var(--motion-progress) linear;
  }
  @keyframes blink {
    50% {
      opacity: 0;
    }
  }
  @keyframes shake {
    20% {
      translate: -6px 0;
    }
    40% {
      translate: 5px 0;
    }
    60% {
      translate: -4px 0;
    }
    80% {
      translate: 3px 0;
    }
  }
  @keyframes pop {
    from {
      scale: 0.6;
    }
  }
  @media (max-width: 760px) {
    .grid {
      grid-template-columns: 1fr;
    }
    .orc {
      border-left: none;
      border-top: var(--rule-thick) solid var(--rule);
    }
    .wide {
      display: none;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .light,
    .shaking,
    .quip {
      animation: none;
    }
  }
</style>
