<script lang="ts">
  import Vial from "$app/mascot/Vial.svelte";
  import BrandIcon from "./BrandIcon.svelte";
  import { LINKS } from "../lib/links";
</script>

<footer class="page">
  <nav class="micro" aria-label="More">
    <a class="with-icon" href={LINKS.repo}><BrandIcon name="github" size={14} />GITHUB</a>
    <a href={LINKS.readme}>README</a>
    <a href={LINKS.releases}>RELEASES</a>
    <a href={LINKS.roadmap}>ROADMAP</a>
    <a href={LINKS.privacy}>PRIVACY</a>
    <a href={LINKS.security}>SECURITY</a>
    <a href={LINKS.license}>LICENSE</a>
    <a href={LINKS.issues}>REPORT A BUG</a>
  </nav>
  <p class="micro muted">
    <span class="vial" aria-hidden="true"><Vial full={true} /></span>
    ORCBLOCK // FREE SOFTWARE UNDER THE AGPL-3.0 // NO ORCS WERE HARMED
  </p>
</footer>

<style>
  footer {
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  nav {
    display: flex;
    flex-wrap: wrap;
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  nav a {
    padding: 12px var(--gutter);
    border-right: var(--rule-thin) solid var(--rule);
    text-decoration: none;
  }
  nav a:hover {
    background: var(--ink);
    color: var(--paper);
  }
  p {
    display: flex;
    align-items: center;
    gap: 10px;
    margin: 0;
    padding: 12px var(--gutter);
  }
  .vial {
    display: block;
    width: 8px;
    height: 12px;
  }
</style>
