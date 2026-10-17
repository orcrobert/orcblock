<script lang="ts">
  import { DOWNLOAD_MAC } from "../lib/links";
  import type { Platform } from "../lib/platform";
  import BrandIcon from "./BrandIcon.svelte";
  import LinkButton from "./LinkButton.svelte";

  let { platform }: { platform: Platform } = $props();

  const other = $derived(
    platform === "windows"
      ? ({ icon: "windows", name: "WINDOWS" } as const)
      : platform === "linux"
        ? ({ icon: "linux", name: "LINUX" } as const)
        : null,
  );
</script>

<div class="download">
  {#if other}
    <LinkButton>
      <span class="with-icon"><BrandIcon name={other.icon} />{other.name} // COMING SOON</span>
      <span>···</span>
    </LinkButton>
    <p class="micro muted">THE ORC ONLY GUARDS MACS FOR NOW. GOT ONE NEARBY?</p>
  {/if}
  <LinkButton href={DOWNLOAD_MAC} variant={other ? "plain" : "solid"} download>
    <span class="with-icon"><BrandIcon name="mac" />DOWNLOAD FOR MACOS</span>
    <span>&gt;&gt;&gt;</span>
  </LinkButton>
  <p class="micro muted">
    LATEST RELEASE // FREE //
    <a href="#download">{other ? "ALL SYSTEMS" : "WINDOWS + LINUX SOON"}</a>
  </p>
</div>

<style>
  .download {
    display: grid;
    max-width: 480px;
    border: var(--rule-thick) solid var(--rule);
  }
  p {
    margin: 0;
    padding: 6px var(--gutter);
    border-top: var(--rule-thin) solid var(--rule);
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  p:last-child {
    border-bottom: none;
  }
</style>
