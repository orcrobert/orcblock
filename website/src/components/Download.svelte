<script lang="ts">
  import icon from "$icons/128x128@2x.png";
  import { DOWNLOAD_MAC, LINKS } from "../lib/links";
  import type { Platform } from "../lib/platform";
  import BrandIcon from "./BrandIcon.svelte";
  import LinkButton from "./LinkButton.svelte";

  let { platform }: { platform: Platform } = $props();

  const SYSTEMS = [
    {
      id: "mac",
      name: "macOS",
      detail: "13 VENTURA OR NEWER // APPLE SILICON + INTEL",
      ready: true,
    },
    { id: "windows", name: "Windows", detail: "THE ORC IS TRAINING", ready: false },
    { id: "linux", name: "Linux", detail: "THE ORC IS TRAINING", ready: false },
  ] as const;
</script>

<section class="band" id="download" aria-labelledby="download-title">
  <div class="page">
    <header class="section-head micro">
      <h2 id="download-title">[ 007 // ACQUIRE ]</h2>
      <a class="muted" href={LINKS.releases}>ALL RELEASES ↗</a>
    </header>

    <div class="top">
      <img src={icon} alt="The OrcBlock app icon" width="128" height="128" />
      <div>
        <p class="headline">Get the<br />orc<span class="red">.</span></p>
        <p class="lead">Free. Always the latest version. Set up in a minute.</p>
      </div>
    </div>

    <ul>
      {#each SYSTEMS as system (system.id)}
        <li class:yours={platform === system.id}>
          <div class="head">
            <BrandIcon name={system.id} size={36} />
            {#if platform === system.id}<span class="tag micro">YOUR SYSTEM</span>{/if}
          </div>
          <h3>{system.name}</h3>
          <p class="micro muted">{system.detail}</p>
          {#if system.ready}
            <LinkButton href={DOWNLOAD_MAC} download>
              <span class="with-icon"><BrandIcon name="mac" size={18} />DOWNLOAD .DMG</span>
              <span>&gt;&gt;&gt;</span>
            </LinkButton>
          {:else}
            <LinkButton>
              <span class="with-icon"><BrandIcon name={system.id} size={18} />COMING SOON</span>
              <span>···</span>
            </LinkButton>
          {/if}
        </li>
      {/each}
    </ul>
  </div>
</section>

<style>
  .top {
    display: flex;
    align-items: center;
    gap: clamp(20px, 4vw, 48px);
    padding: clamp(24px, 5vw, 56px) var(--pad);
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  .top > div {
    display: grid;
    gap: 16px;
  }
  .top img {
    width: clamp(88px, 14vw, 160px);
    height: auto;
    image-rendering: pixelated;
    animation: idle 3s steps(2, end) infinite;
  }
  ul {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    margin: 0;
    padding: 0;
    list-style: none;
  }
  li {
    display: grid;
    grid-template-rows: auto auto 1fr auto;
    gap: 10px;
  }
  li + li {
    border-left: var(--rule-thin) solid var(--rule);
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: start;
    padding: var(--gutter) var(--gutter) 0;
  }
  li:not(:first-child) .head {
    color: var(--ink-muted);
  }
  .tag {
    padding: 2px 6px;
    background: var(--red);
    color: var(--on-red);
  }
  h3 {
    margin: 0;
    padding: 0 var(--gutter);
    font: 900 32px/1 var(--font-macro);
    letter-spacing: -0.03em;
  }
  p {
    margin: 0;
    padding: 0 var(--gutter) 8px;
  }
  li :global(.button) {
    border-top: var(--rule-thin) solid var(--rule);
  }
  .yours {
    box-shadow: inset 0 0 0 var(--rule-thick) var(--red);
  }
  @keyframes idle {
    50% {
      translate: 0 -3px;
    }
  }
  @media (max-width: 760px) {
    ul {
      grid-template-columns: 1fr;
    }
    li + li {
      border-left: none;
      border-top: var(--rule-thin) solid var(--rule);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .top img {
      animation: none;
    }
  }
</style>
