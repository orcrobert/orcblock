<script lang="ts">
  let { armed, label, mode }: { armed: boolean; label: string; mode?: "live" | "dry_run" } =
    $props();
</script>

<div class="bar micro">
  <span>UNIT / ORC-01</span>
  <span class="status" class:armed>
    <span class="light" aria-hidden="true"></span>STATUS: {label}
  </span>
  {#if mode === "dry_run"}<span class="dev" title="The dev daemon logs instead of blocking"
      >DRY-RUN</span
    >{/if}
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: var(--gutter);
    height: 28px;
    padding: 0 var(--gutter);
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  .status {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-left: auto;
  }
  .light {
    width: 8px;
    height: 8px;
    border: var(--rule-thin) solid var(--ink);
  }
  .armed {
    color: var(--red);
  }
  .armed .light {
    border-color: var(--red);
    background: var(--red);
  }
  .dev {
    padding: 1px 4px;
    background: var(--ink);
    color: var(--paper);
  }
</style>
