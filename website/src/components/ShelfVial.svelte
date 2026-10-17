<script lang="ts">
  import { VIAL_EMPTY, VIAL_FULL } from "$app/mascot/frames";
  import { pixelRuns } from "$app/mascot/pixels";
  import { prefersReducedMotion } from "$app/motion";

  let { full, index }: { full: boolean; index: number } = $props();

  type Phase = "idle" | "gone" | "dropping" | "filling";

  const WIDTH = (VIAL_FULL[0] ?? "").length;
  const HEIGHT = VIAL_FULL.length;
  const GLASS = pixelRuns(VIAL_EMPTY);
  // The liquid as one block, so it can rise from the bottom while filling.
  const LIQUID = (() => {
    const rows = VIAL_FULL.flatMap((row, y) => (row.includes("p") ? [y] : []));
    const top = Math.min(...rows);
    const x = Math.min(...VIAL_FULL.map((row) => row.indexOf("p")).filter((at) => at >= 0));
    const width = (VIAL_FULL[top] ?? "").split("").filter((pixel) => pixel === "p").length;
    return { x, y: top, width, height: Math.max(...rows) - top + 1 };
  })();

  const GONE_MS = 2000;
  const DROP_MS = 520;
  const FILL_MS = 900;

  let phase = $state<Phase>("idle");
  let art: HTMLElement | undefined = $state();
  const label = $derived(`NO.${String(index + 1).padStart(2, "0")}`);

  /** One flying pixel: launched from where it sat in the vial, it arcs out
      and falls under gravity until it is below the bottom of the window. */
  function fling(x: number, y: number, size: number, colour: string, spread: number) {
    const bit = document.createElement("span");
    Object.assign(bit.style, {
      position: "fixed",
      left: `${x}px`,
      top: `${y}px`,
      width: `${size}px`,
      height: `${size}px`,
      background: colour,
      pointerEvents: "none",
      zIndex: "50",
    });
    document.body.append(bit);
    const vx = (Math.random() - 0.5) * spread;
    const vy = -(120 + Math.random() * 380);
    const gravity = 1800;
    const spin = (Math.random() - 0.5) * 720;
    // Long enough to clear the bottom of the window from wherever it starts.
    const fall = window.innerHeight - y + 80;
    const seconds = (-vy + Math.sqrt(vy * vy + 2 * gravity * fall)) / gravity;
    const frames = Array.from({ length: 16 }, (_, i) => {
      const t = (seconds * i) / 15;
      const dx = vx * t;
      const dy = vy * t + 0.5 * gravity * t * t;
      return { transform: `translate(${dx}px, ${dy}px) rotate(${spin * t}deg)` };
    });
    bit.animate(frames, { duration: seconds * 1000, easing: "linear" }).onfinish = () =>
      bit.remove();
  }

  function shatter() {
    if (!art) return;
    const box = art.getBoundingClientRect();
    const px = box.width / WIDTH;
    VIAL_FULL.forEach((row, y) => {
      row.split("").forEach((pixel, x) => {
        const left = box.left + x * px;
        const top = box.top + y * px;
        if (pixel === "#") {
          fling(left, top, px, "var(--ink)", 600);
        } else if (pixel === "p" && full) {
          // Liquid splits into droplets that mostly drop straight down.
          for (const [ox, oy] of [
            [0, 0],
            [0.5, 0],
            [0, 0.5],
            [0.5, 0.5],
          ] as const) {
            fling(left + ox * px, top + oy * px, px / 2, "var(--potion)", 260);
          }
        }
      });
    });
  }

  function smash() {
    if (phase !== "idle") return;
    if (!prefersReducedMotion()) shatter();
    phase = "gone";
  }

  $effect(() => {
    if (phase === "idle") return;
    const still = prefersReducedMotion();
    const wait = {
      gone: GONE_MS,
      dropping: still ? 0 : DROP_MS,
      filling: still || !full ? 0 : FILL_MS,
    }[phase];
    const next: Phase = phase === "gone" ? "dropping" : phase === "dropping" ? "filling" : "idle";
    const timer = setTimeout(() => (phase = next), wait);
    return () => clearTimeout(timer);
  });
</script>

<button
  type="button"
  class="vial-button"
  class:full
  data-phase={phase}
  onclick={smash}
  aria-label="Smash potion {label}"
>
  <span class="drop">
    <span class="art" bind:this={art}>
      <svg
        viewBox="0 0 {WIDTH} {HEIGHT}"
        preserveAspectRatio="none"
        shape-rendering="crispEdges"
        aria-hidden="true"
      >
        {#each GLASS as run, i (i)}
          <rect x={run.x} y={run.y} width={run.width} height="1" fill="var(--ink)" />
        {/each}
        {#if full && phase !== "dropping"}
          <rect
            class="liquid"
            x={LIQUID.x}
            y={LIQUID.y}
            width={LIQUID.width}
            height={LIQUID.height}
            fill="var(--potion)"
          />
        {/if}
      </svg>
    </span>
  </span>
  <span class="micro muted">{label}</span>
</button>

<style>
  .vial-button {
    display: grid;
    justify-items: center;
    gap: 10px;
    padding: 0 0 10px;
    border: none;
    background: none;
    cursor: pointer;
  }
  .art {
    display: block;
    width: clamp(40px, 7vw, 64px);
    aspect-ratio: 4 / 6;
    transition: scale 140ms steps(2, end);
  }
  svg {
    display: block;
    width: 100%;
    height: 100%;
    overflow: visible;
  }
  .vial-button[data-phase="idle"]:hover .art,
  .vial-button[data-phase="idle"]:focus-visible .art {
    scale: 1.14;
  }
  .full[data-phase="idle"] .art {
    filter: drop-shadow(0 0 10px color-mix(in srgb, var(--potion) 55%, transparent));
    animation: bob 2.4s steps(4, end) infinite;
    animation-delay: calc(var(--i) * -0.5s);
  }
  [data-phase="gone"] .art {
    visibility: hidden;
  }
  [data-phase="dropping"] .drop {
    display: block;
    animation: drop 520ms cubic-bezier(0.55, 0, 1, 0.45);
  }
  [data-phase="filling"] .liquid {
    transform-box: fill-box;
    transform-origin: bottom;
    animation: fill 900ms steps(6, end);
  }
  .drop {
    display: block;
  }
  @keyframes bob {
    50% {
      translate: 0 -4px;
    }
  }
  @keyframes drop {
    from {
      translate: 0 -260px;
    }
  }
  @keyframes fill {
    from {
      transform: scaleY(0);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .art {
      transition: none;
    }
    .full[data-phase="idle"] .art,
    [data-phase="dropping"] .drop,
    [data-phase="filling"] .liquid {
      animation: none;
    }
  }
</style>
