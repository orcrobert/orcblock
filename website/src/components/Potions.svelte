<script lang="ts">
  import OrcBox from "$app/components/OrcBox.svelte";
  import ShelfVial from "./ShelfVial.svelte";
  import { reveal } from "../lib/reveal";

  const PER_MONTH = 5;

  let available = $state(PER_MONTH);
  let armed = $state(true);
  let orcBox: OrcBox | undefined = $state();

  const potions = $derived({ available, per_month: PER_MONTH, refill_on: "" });

  function drink() {
    if (!armed || available === 0) return;
    available -= 1;
    armed = false;
    orcBox?.drink(available);
  }

  const readout = $derived(
    armed
      ? available === 0
        ? "NO POTIONS LEFT. WAIT IT OUT."
        : "BLOCK RUNNING. THE ORC IS AWAKE."
      : "BLOCK ENDED. SITES ARE BACK.",
  );
</script>

<section class="band" id="potions" aria-labelledby="potions-title">
  <div class="page">
    <header class="section-head micro">
      <h2 id="potions-title">[ 004 // SLEEP POTIONS ]</h2>
      <span class="muted">{String(available).padStart(2, "0")} / 05 LEFT</span>
    </header>

    <div class="grid">
      <div class="copy">
        <p class="headline">Five<br />a month<span class="potion">.</span></p>
        <p class="lead">
          The one way out early. Drink a sleep potion and the orc nods off on the spot: the block
          ends and your sites come back. You get five every month, and the shelf fills back up on
          the 1st.
        </p>
        <p class="lead muted">Spend them wisely. Or try one here, these are free.</p>
      </div>

      <div class="lab">
        <div class="shelf" role="group" aria-label="{available} of {PER_MONTH} sleep potions left">
          {#each { length: PER_MONTH } as _, i (i)}
            <div class="slot" use:reveal style:--i={i}>
              <ShelfVial full={i < available} index={i} />
            </div>
          {/each}
        </div>

        <div class="demo">
          <div class="box">
            <OrcBox bind:this={orcBox} {potions} guarding={armed} />
          </div>
          <div class="controls">
            <p class="micro" aria-live="polite">{readout}</p>
            {#if armed && available > 0}
              <button type="button" class="alert" onclick={drink}>
                <span>DRINK</span><span>&gt;&gt;&gt;</span>
              </button>
            {:else if armed}
              <button type="button" class="off" disabled>
                <span>SHELF EMPTY</span><span>···</span>
              </button>
              <button type="button" class="plain micro" onclick={() => (available = PER_MONTH)}>
                FAST-FORWARD TO THE 1ST (DEMO ONLY)
              </button>
            {:else}
              <button type="button" class="solid" onclick={() => (armed = true)}>
                <span>ARM // START BLOCK</span><span>&gt;&gt;&gt;</span>
              </button>
            {/if}
          </div>
        </div>
      </div>
    </div>
  </div>
</section>

<style>
  .grid {
    display: grid;
    grid-template-columns: 1fr 1.15fr;
  }
  .copy {
    display: grid;
    align-content: start;
    gap: 20px;
    padding: clamp(24px, 5vw, 56px) var(--pad);
    border-right: var(--rule-thin) solid var(--rule);
  }
  .copy .headline {
    margin-bottom: 8px;
  }
  .potion {
    color: var(--potion);
  }
  .lab {
    display: grid;
    grid-template-rows: auto 1fr;
  }
  .shelf {
    display: flex;
    justify-content: center;
    align-items: end;
    gap: clamp(14px, 4vw, 40px);
    padding: 40px var(--gutter) 0;
    border-bottom: var(--rule-thick) solid var(--rule);
    /* The vials stand on the thick rule like a plank. */
    background: repeating-linear-gradient(-45deg, var(--hatch) 0 1px, transparent 1px 8px);
  }
  .slot {
    transition-delay: calc(var(--i) * 60ms);
  }
  .demo {
    display: grid;
    grid-template-columns: 200px 1fr;
  }
  .box {
    display: grid;
    min-height: 250px;
    border-right: var(--rule-thin) solid var(--rule);
  }
  .box :global(.orc) {
    border-left: none;
  }
  .controls {
    display: grid;
    align-content: end;
  }
  .controls p {
    margin: 0;
    padding: var(--gutter);
  }
  button {
    display: flex;
    justify-content: space-between;
    align-items: center;
    min-height: 64px;
    padding: 0 var(--gutter);
    border: none;
    border-top: var(--rule-thick) solid var(--rule);
    font: var(--type-label);
    text-transform: uppercase;
  }
  button.micro {
    min-height: var(--row);
    border-top-width: var(--rule-thin);
    font: var(--type-micro);
  }
  .alert {
    background: var(--red);
    color: var(--on-red);
  }
  .solid {
    background: var(--ink);
    color: var(--paper);
  }
  .plain {
    background: var(--paper);
  }
  .alert:hover,
  .solid:hover {
    filter: invert(1);
  }
  .plain:hover {
    background: var(--ink);
    color: var(--paper);
  }
  .off {
    background: repeating-linear-gradient(-45deg, var(--hatch) 0 2px, transparent 2px 7px);
    color: var(--ink-muted);
    cursor: not-allowed;
  }
  @media (max-width: 900px) {
    .grid {
      grid-template-columns: 1fr;
    }
    .copy {
      border-right: none;
      border-bottom: var(--rule-thin) solid var(--rule);
    }
  }
  @media (max-width: 460px) {
    .demo {
      grid-template-columns: 1fr;
    }
    .box {
      border-right: none;
      border-bottom: var(--rule-thin) solid var(--rule);
    }
  }
</style>
