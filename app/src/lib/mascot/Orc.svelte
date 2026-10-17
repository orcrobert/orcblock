<script lang="ts">
  import { ANIMATIONS, SIZE, type OrcState } from "./frames";
  import { fillOf, pixelRuns } from "./pixels";
  import { prefersReducedMotion } from "../motion";

  let {
    pose,
    scale = 4,
    ondone,
  }: {
    pose: OrcState;
    /** Screen pixels per art pixel. */
    scale?: number;
    /** Called once a non-looping animation has played and held its last frame. */
    ondone?: () => void;
  } = $props();

  const HOLD_FRAMES = 3;

  let index = $state(0);
  const animation = $derived(ANIMATIONS[pose] ?? ANIMATIONS.sleeping);

  $effect(() => {
    index = 0;
    if (prefersReducedMotion()) {
      if (animation.loop) return;
      // A still of the end pose, so the state still reads without motion.
      index = animation.frames.length - 1;
      const timer = setTimeout(() => ondone?.(), 800);
      return () => clearTimeout(timer);
    }
    let tick = 0;
    const timer = setInterval(() => {
      tick += 1;
      if (animation.loop) {
        index = tick % animation.frames.length;
      } else if (tick < animation.frames.length) {
        index = tick;
      } else if (tick >= animation.frames.length + HOLD_FRAMES) {
        clearInterval(timer);
        ondone?.();
      }
    }, 1000 / animation.fps);
    return () => clearInterval(timer);
  });

  const runs = $derived(pixelRuns(animation.frames[index] ?? []));
</script>

<svg
  width={SIZE * scale}
  height={SIZE * scale}
  viewBox="0 0 {SIZE} {SIZE}"
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
    image-rendering: pixelated;
  }
</style>
