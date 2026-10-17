<script lang="ts">
  import { api, type Site } from "../api";
  import IconButton from "./IconButton.svelte";
  import Monogram from "./Monogram.svelte";

  let { site, locked, onremove }: { site: Site; locked: boolean; onremove: (site: Site) => void } =
    $props();

  let icon = $state<string | null>(null);
  let failed = $state(false);

  $effect(() => {
    const domain = site.domain;
    let current = true;
    api
      .favicon(domain)
      .then((url) => {
        if (current) icon = url;
      })
      .catch(() => {
        if (current) icon = null;
      });
    return () => {
      current = false;
    };
  });
</script>

<li>
  {#if icon && !failed}
    <img src={icon} alt="" width="24" height="24" onerror={() => (failed = true)} />
  {:else}
    <Monogram domain={site.domain} display={site.display} />
  {/if}
  <span class="name" title={site.display}>{site.display}</span>
  <IconButton
    icon={locked ? "lock" : "close"}
    label={locked ? `${site.display} is locked until the block ends` : `Remove ${site.display}`}
    disabled={locked}
    onclick={() => onremove(site)}
  />
</li>

<style>
  li {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-height: 48px;
    padding-left: var(--space-1);
  }
  img {
    border-radius: var(--md-sys-shape-corner-small);
    object-fit: contain;
  }
  .name {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font: var(--md-sys-typescale-title-medium);
  }
</style>
