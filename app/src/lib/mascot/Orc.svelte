<script lang="ts" module>
  export type OrcState = "sleeping" | "guarding";
</script>

<script lang="ts">
  // A monochrome line mark drawn in currentColor; the state shows in the eyes.
  let { state, size = 24 }: { state: OrcState; size?: number } = $props();
</script>

<svg
  class="orc"
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width="1.6"
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  <path d="M6 9.5C6 6 8.7 4 12 4s6 2 6 5.5V13c0 4-2.5 7-6 7s-6-3-6-7z" />
  <path d="M6 10 2.8 8.2 6 12.6M18 10l3.2-1.8-3.2 4.4" />
  <path d="M9 16.2c2 1 4 1 6 0M9.9 16.6v-1.7M14.1 16.6v-1.7" />
  {#if state === "guarding"}
    <path d="m8.4 10.2 2.4.9m4.8-.9-2.4.9" />
    <g class="eyes" fill="currentColor" stroke="none">
      <circle cx="10" cy="12.6" r="0.95" />
      <circle cx="14" cy="12.6" r="0.95" />
    </g>
  {:else}
    <path d="M8.6 12.3c.8.7 1.6.7 2.4 0m2 0c.8.7 1.6.7 2.4 0" />
  {/if}
</svg>

<style>
  .orc {
    flex: none;
  }
  .eyes {
    transform-origin: 12px 12.6px;
    animation: blink 7s infinite;
  }
  @keyframes blink {
    0%,
    96%,
    100% {
      transform: scaleY(1);
    }
    98% {
      transform: scaleY(0.1);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .eyes {
      animation: none;
    }
  }
</style>
