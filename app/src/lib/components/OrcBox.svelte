<script lang="ts">
  import type { Potions } from "../api";
  import { VIAL_AT_MOUTH, VIAL_FULL, VIAL_IN_HAND } from "../mascot/frames";
  import Orc from "../mascot/Orc.svelte";
  import Vial from "../mascot/Vial.svelte";
  import { prefersReducedMotion } from "../motion";
  import PotionVials from "./PotionVials.svelte";

  let { potions, guarding }: { potions: Potions; guarding: boolean } = $props();

  const SCALE = 5;
  const VIAL_WIDTH = (VIAL_FULL[0] ?? "").length;
  const VIAL_HEIGHT = VIAL_FULL.length;

  type Phase = "fall" | "drink" | "rise";
  let phase = $state<Phase | null>(null);
  let slot = $state(-1);
  let figure: HTMLElement | undefined = $state();
  let sprite: HTMLElement | undefined = $state();
  let flyer: HTMLElement | undefined = $state();

  const pose = $derived(
    phase === "drink"
      ? "drinking"
      : phase === "rise" || (phase === null && !guarding)
        ? "sleeping"
        : "guarding",
  );
  const caption = $derived(
    { drinking: "DRINKING", guarding: "ON GUARD", sleeping: "ASLEEP" }[pose],
  );

  /** Plays the potion: the vial in `used` drops into the orc's hand, he
      drinks, falls asleep, and the empty vial climbs back to its slot. */
  export function drink(used: number) {
    slot = used;
    phase = "fall";
  }

  function boxIn(parent: DOMRect, rect: DOMRect | { left: number; top: number }) {
    return { left: rect.left - parent.left, top: rect.top - parent.top };
  }

  $effect(() => {
    if ((phase !== "fall" && phase !== "rise") || !figure || !sprite || !flyer) return;
    const rising = phase === "rise";
    const frame = figure.getBoundingClientRect();
    const art = sprite.getBoundingClientRect();
    const shelf = figure.querySelector(`[data-slot="${slot}"]`)?.getBoundingClientRect();
    const at = rising ? VIAL_AT_MOUTH : VIAL_IN_HAND;
    const hand = {
      ...boxIn(frame, { left: art.left + at.x * SCALE, top: art.top + at.y * SCALE }),
      width: VIAL_WIDTH * SCALE,
      height: VIAL_HEIGHT * SCALE,
    };
    const home = shelf
      ? { ...boxIn(frame, shelf), width: shelf.width, height: shelf.height }
      : { ...hand, top: 0 };
    const [from, to] = rising ? [hand, home] : [home, hand];
    const px = (box: typeof hand) => ({
      left: `${box.left}px`,
      top: `${box.top}px`,
      width: `${box.width}px`,
      height: `${box.height}px`,
    });
    const animation = flyer.animate([px(from), px(to)], {
      duration: prefersReducedMotion() ? 0 : rising ? 900 : 520,
      // Falls under gravity; climbs back with a gentle start and stop.
      easing: rising ? "cubic-bezier(0.45, 0, 0.25, 1)" : "cubic-bezier(0.55, 0, 1, 0.45)",
      fill: "forwards",
    });
    animation.onfinish = () => {
      phase = rising ? null : "drink";
    };
    return () => animation.cancel();
  });
</script>

<figure class="orc" bind:this={figure}>
  <PotionVials {potions} away={phase === null ? -1 : slot} announce={!guarding} />
  <div class="sprite" bind:this={sprite}>
    <Orc {pose} scale={SCALE} ondone={() => (phase = "rise")} />
  </div>
  {#if phase === "fall" || phase === "rise"}
    <span class="flyer" bind:this={flyer}><Vial full={phase === "fall"} /></span>
  {/if}
  <figcaption class="micro">ORC-01 // {caption}</figcaption>
</figure>

<style>
  .orc {
    position: relative;
    display: grid;
    grid-template-rows: auto 1fr auto;
    justify-items: center;
    align-items: end;
    margin: 0;
    border-left: var(--rule-thin) solid var(--rule);
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
  figcaption {
    width: 100%;
    padding: 6px 0;
    border-top: var(--rule-thin) solid var(--rule);
    text-align: center;
  }
  .flyer {
    position: absolute;
    left: 0;
    top: 0;
    display: block;
    pointer-events: none;
  }
</style>
