<script lang="ts">
  const ATTEMPTS = [
    {
      label: "Quit the app",
      verdict: "DENIED",
      reply: "The orc doesn't live in the app. Still blocked.",
    },
    {
      label: "Restart the Mac",
      verdict: "DENIED",
      reply: "He's back on guard before you've logged in.",
    },
    {
      label: "Change the clock",
      verdict: "DENIED",
      reply: "He keeps his own count of the minutes.",
    },
    {
      label: "Delete the site",
      verdict: "DENIED",
      reply: "The list is locked until the timer runs out.",
    },
    { label: "Ask nicely", verdict: "DENIED", reply: "Orcs don't negotiate." },
    { label: "Drink a potion", verdict: "ALLOWED", reply: "That one works. Five times a month." },
  ];

  let tried = $state<number[]>([]);
  let last = $state(-1);

  function attempt(i: number) {
    last = i;
    if (!tried.includes(i)) tried = [...tried, i];
  }

  const message = $derived(last >= 0 ? ATTEMPTS[last] : undefined);
</script>

<section class="band" aria-labelledby="escape-title">
  <div class="page">
    <header class="section-head micro">
      <h2 id="escape-title">[ 003 // ESCAPE ATTEMPTS ]</h2>
      <span class="muted">{String(tried.length).padStart(2, "0")} / 06 TRIED</span>
    </header>

    <div class="grid">
      <div class="intro">
        <p class="headline">Try<br />it<span class="red">.</span></p>
        <p class="lead">
          Every website blocker has a back door. Pick one and see how the orc takes it.
        </p>
      </div>

      <div class="board">
        <ul>
          {#each ATTEMPTS as item, i (item.label)}
            <li>
              <button
                type="button"
                class:tried={tried.includes(i)}
                class:allowed={item.verdict === "ALLOWED"}
                onclick={() => attempt(i)}
              >
                <span>{item.label}</span><span class="micro">&gt;&gt;&gt;</span>
              </button>
            </li>
          {/each}
        </ul>
        <div class="readout" aria-live="polite">
          {#if message}
            {#key last}
              <p class="stamp" class:ok={message.verdict === "ALLOWED"}>{message.verdict}</p>
              <p class="reply">{message.reply}</p>
              {#if message.verdict === "ALLOWED"}
                <a class="micro" href="#potions">SEE THE POTIONS ↓</a>
              {/if}
            {/key}
          {:else}
            <p class="micro muted">AWAITING INPUT_</p>
          {/if}
        </div>
      </div>
    </div>
  </div>
</section>

<style>
  .grid {
    display: grid;
    grid-template-columns: 1fr 1.2fr;
  }
  .intro {
    display: grid;
    align-content: start;
    gap: 24px;
    padding: clamp(24px, 5vw, 56px) var(--pad);
    border-right: var(--rule-thin) solid var(--rule);
  }
  .board {
    display: grid;
    grid-template-rows: auto 1fr;
  }
  ul {
    display: grid;
    grid-template-columns: 1fr 1fr;
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  li:nth-child(odd) {
    border-right: var(--rule-thin) solid var(--rule);
  }
  button {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    height: 64px;
    padding: 0 var(--gutter);
    border: none;
    background: var(--paper);
    font: var(--type-label);
    text-align: left;
    text-transform: uppercase;
  }
  button:hover {
    background: var(--ink);
    color: var(--paper);
  }
  button:active {
    translate: 0 1px;
  }
  .tried:not(.allowed) {
    color: var(--ink-muted);
    text-decoration: line-through;
    text-decoration-color: var(--red);
    text-decoration-thickness: 2px;
  }
  .allowed {
    color: var(--potion);
  }
  .allowed:hover {
    background: var(--potion);
    color: var(--paper);
  }
  .readout {
    display: grid;
    align-content: center;
    justify-items: start;
    gap: 10px;
    min-height: 200px;
    padding: var(--gutter) var(--pad);
    background: repeating-linear-gradient(-45deg, var(--hatch) 0 1px, transparent 1px 8px);
  }
  .readout p {
    margin: 0;
  }
  .stamp {
    padding: 4px 14px 2px;
    border: 4px solid var(--red);
    color: var(--red);
    font: 900 clamp(40px, 6vw, 64px) / 1 var(--font-macro);
    letter-spacing: -0.02em;
    rotate: -4deg;
    animation: stamp 260ms steps(3, end);
  }
  .stamp.ok {
    border-color: var(--potion);
    color: var(--potion);
  }
  .reply {
    font: 500 17px/1.4 var(--font-macro);
  }
  @keyframes stamp {
    from {
      scale: 2.2;
      opacity: 0;
    }
  }
  @media (max-width: 820px) {
    .grid {
      grid-template-columns: 1fr;
    }
    .intro {
      border-right: none;
      border-bottom: var(--rule-thin) solid var(--rule);
    }
  }
  @media (max-width: 420px) {
    ul {
      grid-template-columns: 1fr;
    }
    li:nth-child(odd) {
      border-right: none;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .stamp {
      animation: none;
    }
  }
</style>
