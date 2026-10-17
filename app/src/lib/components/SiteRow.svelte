<script lang="ts">
  import { api, type Site } from "../api";
  import { formatIsoDay, pad2 } from "../format";

  let {
    site,
    index,
    locked,
    onremove,
  }: { site: Site; index: number; locked: boolean; onremove: (site: Site) => void } = $props();

  let icon = $state<string | null>(null);
  let failed = $state(false);

  $effect(() => {
    const domain = site.domain;
    let current = true;
    failed = false;
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
  <span class="index micro muted">{pad2(index)}</span>
  <span class="icon" aria-hidden="true">
    {#if icon && !failed}
      <img src={icon} alt="" width="16" height="16" onerror={() => (failed = true)} />
    {:else}
      <span class="blank"></span>
    {/if}
  </span>
  <span class="name" title={site.display}>{site.display}</span>
  {#if locked}
    <span class="state micro locked">LOCKED</span>
  {:else}
    <span class="state date micro muted">{formatIsoDay(site.added_at)}</span>
    <button
      type="button"
      class="micro"
      aria-label="Remove {site.display}"
      onclick={() => onremove(site)}>[ × ]</button
    >
  {/if}
</li>

<style>
  li {
    display: grid;
    grid-template-columns: 28px 16px 1fr auto auto;
    align-items: center;
    gap: 10px;
    height: var(--row);
    padding-left: var(--gutter);
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  .icon {
    display: grid;
    place-items: center;
    width: 16px;
    height: 16px;
  }
  img {
    filter: grayscale(1) contrast(1.4);
  }
  .blank {
    width: 10px;
    height: 10px;
    border: var(--rule-thin) solid var(--ink-muted);
  }
  .name {
    overflow: hidden;
    font: var(--type-data);
    letter-spacing: 0.02em;
    text-transform: uppercase;
    white-space: nowrap;
    text-overflow: ellipsis;
  }
  .state {
    padding-right: var(--gutter);
  }
  /* At the narrowest window the name needs the room more than the date. */
  @container targets (max-width: 380px) {
    .date {
      display: none;
    }
  }
  .locked {
    color: var(--red);
  }
  button {
    align-self: stretch;
    padding: 0 var(--gutter);
    border: none;
    border-left: var(--rule-thin) solid var(--rule);
    background: transparent;
    cursor: pointer;
  }
  button:hover {
    background: var(--red);
    color: var(--on-red);
  }
</style>
