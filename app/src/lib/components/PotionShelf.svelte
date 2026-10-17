<script lang="ts">
  import type { Potions } from "../api";

  let { potions }: { potions: Potions } = $props();
</script>

<dl class="shelf micro">
  <dt>SLEEP POTIONS</dt>
  <dd><data value={potions.available}>{potions.available}/{potions.per_month} LEFT</data></dd>
  <dd class="muted">REFILL {potions.refill_on}</dd>
</dl>

<style>
  .shelf {
    display: grid;
    align-content: center;
    gap: 4px;
    margin: 0;
    padding: 0 var(--gutter);
  }
  dd {
    margin: 0;
  }
</style>
