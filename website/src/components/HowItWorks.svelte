<script lang="ts">
  import { reveal } from "../lib/reveal";
  import MacWindow from "./MacWindow.svelte";

  const STEPS = [
    {
      title: "List the sites and apps",
      body: "Paste a link or type a name. A YouTube video link becomes youtube.com, and every corner of it is covered. Apps come from a list of what's on your Mac.",
    },
    {
      title: "Set the timer",
      body: "Twenty-five minutes, an afternoon, a whole week. Arm it and the countdown starts.",
    },
    {
      title: "The orc stands guard",
      body: "The sites stop loading in every browser, and the apps close the moment they open. Close OrcBlock, quit it, restart your Mac: the block is still there when you come back.",
    },
    {
      title: "Wait it out",
      body: "When the timer hits zero the orc nods off and your sites and apps come back on their own. Nothing to remember to switch off.",
    },
  ];
</script>

<section class="band" id="how" aria-labelledby="how-title">
  <div class="page">
    <header class="section-head micro">
      <h2 id="how-title">[ 002 // PROCEDURE ]</h2>
      <span class="muted">04 STEPS</span>
    </header>

    <div class="shot">
      <MacWindow />
    </div>

    <ol>
      {#each STEPS as step, i (step.title)}
        <li use:reveal>
          <span class="index micro red">{String(i + 1).padStart(2, "0")}</span>
          <h3>{step.title}</h3>
          <p>{step.body}</p>
        </li>
      {/each}
    </ol>
  </div>
</section>

<style>
  .shot {
    padding: clamp(24px, 5vw, 64px) clamp(16px, 6vw, 88px) clamp(32px, 6vw, 72px);
    border-bottom: var(--rule-thin) solid var(--rule);
    background:
      repeating-linear-gradient(90deg, transparent 0 39px, var(--hatch) 39px 40px),
      repeating-linear-gradient(0deg, transparent 0 39px, var(--hatch) 39px 40px);
  }
  ol {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: grid;
    align-content: start;
    gap: 12px;
    padding: var(--gutter) var(--gutter) 28px;
  }
  li + li {
    border-left: var(--rule-thin) solid var(--rule);
  }
  .index {
    font-size: 13px;
  }
  h3 {
    margin: 0;
    font: 900 26px/0.95 var(--font-macro);
    letter-spacing: -0.03em;
    text-transform: uppercase;
  }
  p {
    margin: 0;
    font: 400 15px/1.5 var(--font-macro);
  }
  @media (max-width: 900px) {
    ol {
      grid-template-columns: 1fr 1fr;
    }
    li:nth-child(3) {
      border-left: none;
    }
    li:nth-child(n + 3) {
      border-top: var(--rule-thin) solid var(--rule);
    }
  }
  @media (max-width: 520px) {
    ol {
      grid-template-columns: 1fr;
    }
    li + li {
      border-left: none;
      border-top: var(--rule-thin) solid var(--rule);
    }
  }
</style>
