<script lang="ts" module>
  import { api } from "../api";

  // One request per app for the whole session: the picker grid and the
  // list mount many icons, and every open would otherwise ask again.
  const icons = new Map<string, Promise<string | null>>();

  function iconOf(bundleId: string): Promise<string | null> {
    let icon = icons.get(bundleId);
    if (!icon) {
      icon = api.appIcon(bundleId).catch(() => null);
      icons.set(bundleId, icon);
    }
    return icon;
  }
</script>

<script lang="ts">
  import { monogram } from "../picker";

  let { bundleId, name, size }: { bundleId: string; name: string; size: 16 | 32 } = $props();

  let icon = $state<string | null>(null);
  let failed = $state(false);

  $effect(() => {
    const id = bundleId;
    let current = true;
    icon = null;
    failed = false;
    void iconOf(id).then((url) => {
      if (current) icon = url;
    });
    return () => {
      current = false;
    };
  });
</script>

<span class="icon" style:--size="{size}px" aria-hidden="true">
  {#if icon && !failed}
    <img src={icon} alt="" width={size} height={size} onerror={() => (failed = true)} />
  {:else}
    <span class="mono micro" class:small={size === 16}>{monogram(name)}</span>
  {/if}
</span>

<style>
  .icon {
    display: grid;
    place-items: center;
    width: var(--size);
    height: var(--size);
  }
  img {
    filter: grayscale(1) contrast(1.2);
  }
  .mono {
    display: grid;
    place-items: center;
    width: 100%;
    height: 100%;
    border: var(--rule-thin) solid var(--ink-muted);
    font-weight: 700;
    letter-spacing: 0;
  }
  .mono.small {
    font-size: 8px;
  }
</style>
