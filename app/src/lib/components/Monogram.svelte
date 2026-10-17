<script lang="ts">
  import { monogram } from "../format";

  let { domain, display }: { domain: string; display: string } = $props();
  const mark = $derived(monogram(domain, display));
</script>

<span class="monogram" style:--hue={mark.hue} aria-hidden="true">{mark.letter}</span>

<style>
  .monogram {
    display: grid;
    place-items: center;
    width: 24px;
    height: 24px;
    border-radius: var(--md-sys-shape-corner-full);
    background: oklch(88% 0.06 var(--hue));
    color: oklch(32% 0.08 var(--hue));
    font: var(--md-sys-typescale-label-medium);
  }
  :global(:root[data-theme="dark"]) .monogram {
    background: oklch(36% 0.06 var(--hue));
    color: oklch(90% 0.05 var(--hue));
  }
  @media (prefers-color-scheme: dark) {
    :global(:root[data-theme="system"]) .monogram {
      background: oklch(36% 0.06 var(--hue));
      color: oklch(90% 0.05 var(--hue));
    }
  }
</style>
