<script lang="ts">
  import { initial } from "../format";

  let { src, display }: { src: string | null; display: string } = $props();

  let failed = $state(false);
  $effect(() => {
    void src;
    failed = false;
  });
</script>

<span class="avatar" aria-hidden="true">
  {#if src && !failed}
    <img {src} alt="" width="20" height="20" onerror={() => (failed = true)} />
  {:else}
    {initial(display)}
  {/if}
</span>

<style>
  .avatar {
    display: grid;
    flex: none;
    place-items: center;
    width: 40px;
    height: 40px;
    border-radius: var(--md-sys-shape-corner-full);
    background: var(--md-sys-color-surface-container-highest);
    color: var(--md-sys-color-on-surface-variant);
    font: var(--md-sys-typescale-title-medium);
  }
  img {
    border-radius: var(--md-sys-shape-corner-extra-small);
    object-fit: contain;
  }
</style>
