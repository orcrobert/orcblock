<script lang="ts">
  import { api, type Site } from "../api";
  import { formatDate } from "../format";
  import Avatar from "./Avatar.svelte";
  import Icon from "./Icon.svelte";
  import IconButton from "./IconButton.svelte";

  let { site, locked, onremove }: { site: Site; locked: boolean; onremove: (site: Site) => void } =
    $props();

  let icon = $state<string | null>(null);

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
  <Avatar src={icon} display={site.display} />
  <div class="text">
    <span class="headline" title={site.display}>{site.display}</span>
    <span class="supporting">
      {locked ? "Blocked until the timer ends" : `Added ${formatDate(site.added_at)}`}
    </span>
  </div>
  {#if locked}
    <span class="lock"><Icon name="lock" size={20} /></span>
  {:else}
    <IconButton icon="close" label="Remove {site.display}" onclick={() => onremove(site)} />
  {/if}
</li>

<style>
  li {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    min-height: 72px;
    padding: var(--space-2) var(--space-3) var(--space-2) var(--space-4);
  }
  li + :global(li) {
    border-top: 1px solid var(--md-sys-color-outline-variant);
  }
  .text {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-width: 0;
  }
  .headline,
  .supporting {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .headline {
    color: var(--md-sys-color-on-surface);
    font: var(--md-sys-typescale-body-large);
  }
  .supporting {
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-body-medium);
  }
  .lock {
    display: grid;
    place-items: center;
    width: 40px;
    color: var(--md-sys-color-on-surface-variant);
  }
</style>
