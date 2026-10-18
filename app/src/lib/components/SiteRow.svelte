<script lang="ts">
  import { api, type Site } from "../api";
  import { formatIsoDay, pad2 } from "../format";

  let {
    site,
    index,
    locked,
    onremove,
  }: { site: Site; index: number; locked: boolean; onremove: (site: Site) => void } = $props();

  // A site that cannot be reached now (blocked, offline) gets its icon on a
  // later try. Unreachable sites are not cached, so each try goes to the
  // network: back off from a minute to an hour. Sites known to have no icon
  // also return null, but those tries are answered from the cache.
  const RETRY_MS = 60_000;
  const MAX_RETRY_MS = 3_600_000;

  let icon = $state<string | null>(null);
  let failed = $state(false);

  $effect(() => {
    const domain = site.domain;
    let current = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    let delay = RETRY_MS;
    icon = null;
    failed = false;
    const load = () => {
      api
        .favicon(domain)
        // A failed call is treated like no icon yet and tried again.
        .catch(() => null)
        .then((url) => {
          if (!current) return;
          icon = url;
          if (url) return;
          timer = setTimeout(load, delay);
          delay = Math.min(delay * 2, MAX_RETRY_MS);
        });
    };
    load();
    return () => {
      current = false;
      clearTimeout(timer);
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
