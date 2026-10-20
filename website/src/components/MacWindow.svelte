<script lang="ts">
  import { SCREENSHOTS } from "../lib/screenshots";
</script>

<figure>
  <div class="window">
    <img
      class="shot light"
      src={SCREENSHOTS.light}
      width={SCREENSHOTS.width}
      height={SCREENSHOTS.height}
      alt="The OrcBlock window during a block: time remaining, the orc on guard and the list of blocked sites and apps."
      loading="lazy"
      decoding="async"
    />
    <img
      class="shot dark"
      src={SCREENSHOTS.dark}
      width={SCREENSHOTS.width}
      height={SCREENSHOTS.height}
      alt="The OrcBlock window during a block, in dark mode."
      loading="lazy"
      decoding="async"
    />
    <!-- The window uses an overlay title bar: the traffic lights sit on the
         app's own top bar, where the screenshot leaves room for them. -->
    <span class="lights" aria-hidden="true"><i></i><i></i><i></i></span>
  </div>
  <figcaption class="micro">
    <span>FIG. 01 // ORCBLOCK.APP, ARMED</span>
    <span class="muted">ACTUAL SCREENSHOT</span>
  </figcaption>
</figure>

<style>
  figure {
    margin: 0;
  }
  .window {
    container-type: inline-size;
    position: relative;
    overflow: hidden;
    /* macOS window chrome is the one place with round corners. */
    border-radius: 12px;
    border: var(--rule-thin) solid var(--rule);
    box-shadow:
      10px 10px 0 var(--ink),
      0 30px 60px -20px rgb(0 0 0 / 0.35);
    background: var(--paper);
  }
  .shot {
    display: block;
    width: 100%;
    height: auto;
  }
  .lights {
    position: absolute;
    /* Positions in the window's own points (900 wide), scaled to its width. */
    top: calc(14 / 900 * 100cqw);
    left: calc(20 / 900 * 100cqw);
    display: flex;
    gap: calc(8 / 900 * 100cqw);
  }
  .lights i {
    width: calc(12 / 900 * 100cqw);
    height: calc(12 / 900 * 100cqw);
    border-radius: 50%;
    box-shadow: inset 0 0 0 0.5px rgb(0 0 0 / 0.2);
  }
  /* Apple's own traffic-light colours, not part of the theme. */
  .lights i:nth-child(1) {
    background: #ff5f57;
  }
  .lights i:nth-child(2) {
    background: #febc2e;
  }
  .lights i:nth-child(3) {
    background: #28c840;
  }
  figcaption {
    display: flex;
    justify-content: space-between;
    gap: var(--gutter);
    margin-top: 22px;
  }

  .dark {
    display: none;
  }
  :global(:root[data-theme="dark"]) .light {
    display: none;
  }
  :global(:root[data-theme="dark"]) .dark {
    display: block;
  }
</style>
