<script lang="ts">
  import Orc from "$app/mascot/Orc.svelte";
  import { reveal } from "../lib/reveal";

  // The drinking animation plays once; replaying it means remounting it.
  let take = $state(0);
  let finished = $state(false);

  $effect(() => {
    if (!finished) return;
    const timer = setTimeout(() => {
      finished = false;
      take += 1;
    }, 1400);
    return () => clearTimeout(timer);
  });
</script>

<section class="band" id="orc" aria-labelledby="orc-title">
  <div class="page">
    <header class="section-head micro">
      <h2 id="orc-title">[ 005 // PERSONNEL FILE ]</h2>
      <span class="muted">UNIT ORC-01</span>
    </header>

    <div class="file">
      <div class="copy">
        <p class="headline">Meet<br />the orc<span class="red">.</span></p>
        <p class="lead">
          ORC-01 lives in the corner of the window and takes the job seriously. Asleep when there is
          nothing to guard, red-eyed while a block runs, out cold after a potion. He doesn't take
          bribes and he doesn't get bored.
        </p>
      </div>

      <ul class="poses">
        <li use:reveal>
          <div class="stage"><Orc pose="sleeping" scale={5} /></div>
          <p class="micro"><span>01</span> ASLEEP <span class="muted">// NOTHING BLOCKED</span></p>
        </li>
        <li use:reveal>
          <div class="stage"><Orc pose="guarding" scale={5} /></div>
          <p class="micro">
            <span class="red">02</span> ON GUARD <span class="muted">// BLOCK RUNNING</span>
          </p>
        </li>
        <li use:reveal>
          <div class="stage">
            {#key take}
              <Orc pose="drinking" scale={5} ondone={() => (finished = true)} />
            {/key}
          </div>
          <p class="micro">
            <span class="potion">03</span> DRINKING <span class="muted">// POTION USED</span>
          </p>
        </li>
      </ul>
    </div>
  </div>
</section>

<style>
  .file {
    display: grid;
    grid-template-columns: 1fr 1.6fr;
  }
  .copy p {
    margin: 0;
  }
  .copy {
    display: grid;
    align-content: start;
    gap: 24px;
    padding: clamp(24px, 5vw, 56px) var(--pad);
    border-right: var(--rule-thin) solid var(--rule);
  }
  .poses {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: grid;
    grid-template-rows: 1fr auto;
  }
  li + li {
    border-left: var(--rule-thin) solid var(--rule);
  }
  .stage {
    display: grid;
    place-items: end center;
    min-height: 220px;
    padding-top: 32px;
    background: repeating-linear-gradient(-45deg, var(--hatch) 0 1px, transparent 1px 10px);
  }
  li p {
    margin: 0;
    padding: 8px var(--gutter);
    border-top: var(--rule-thin) solid var(--rule);
  }
  .potion {
    color: var(--potion);
  }
  @media (max-width: 960px) {
    .file {
      grid-template-columns: 1fr;
    }
    .copy {
      border-right: none;
      border-bottom: var(--rule-thin) solid var(--rule);
    }
  }
  @media (max-width: 560px) {
    .poses {
      grid-template-columns: 1fr;
    }
    li + li {
      border-left: none;
      border-top: var(--rule-thin) solid var(--rule);
    }
  }
</style>
