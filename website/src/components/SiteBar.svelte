<script lang="ts">
  import ThemeSwitch from "./ThemeSwitch.svelte";
  import icon from "$icons/128x128.png";
  import { LINKS } from "../lib/links";
</script>

<header>
  <a class="brand" href="/" aria-label="OrcBlock home">
    <img src={icon} alt="" width="24" height="24" />
    <span class="name">[ ORCBLOCK ]</span>
  </a>
  <nav class="micro" aria-label="Sections">
    <a href="/#how">HOW</a>
    <a href="/#potions">POTIONS</a>
    <a href="/#orc">ORC</a>
    <a href="/#download">DOWNLOAD</a>
  </nav>
  <a class="micro git" href={LINKS.repo} rel="noopener">GITHUB ↗</a>
  <ThemeSwitch />
</header>

<style>
  header {
    position: sticky;
    top: 0;
    z-index: 10;
    display: grid;
    grid-template-columns: auto 1fr auto auto;
    align-items: center;
    height: 40px;
    border-bottom: var(--rule-thick) solid var(--rule);
    background: var(--paper);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 100%;
    padding: 0 var(--gutter);
    text-decoration: none;
  }
  .brand img {
    display: block;
    image-rendering: pixelated;
  }
  .name {
    font: var(--type-label);
    letter-spacing: 0.04em;
  }
  .brand:hover {
    color: inherit;
  }
  .brand:hover img {
    animation: hop 320ms steps(2, end);
  }
  nav {
    display: flex;
    justify-content: flex-end;
    height: 100%;
  }
  nav a,
  .git {
    display: grid;
    place-items: center;
    height: 100%;
    padding: 0 var(--gutter);
    border-left: var(--rule-thin) solid var(--rule);
    text-decoration: none;
  }
  nav a:hover,
  .git:hover {
    background: var(--ink);
    color: var(--paper);
  }
  @keyframes hop {
    50% {
      translate: 0 -4px;
    }
  }
  @media (max-width: 720px) {
    nav {
      visibility: hidden;
      width: 0;
      padding: 0;
      overflow: hidden;
    }
  }
  @media (max-width: 400px) {
    .name {
      display: none;
    }
  }
</style>
