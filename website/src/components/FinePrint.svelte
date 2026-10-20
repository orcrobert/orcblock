<script lang="ts">
  import { LINKS } from "../lib/links";
</script>

<section class="band" aria-labelledby="faq-title">
  <div class="page">
    <header class="section-head micro">
      <h2 id="faq-title">[ 006 // FINE PRINT ]</h2>
      <span class="muted">READ BEFORE ARMING</span>
    </header>

    <dl>
      <div>
        <dt>Does it cost anything?</dt>
        <dd>No. OrcBlock is free and open source. No account, no subscription, no ads.</dd>
      </div>
      <div>
        <dt>What do I need?</dt>
        <dd>A Mac with macOS 13 Ventura or newer. Apple silicon and Intel both work.</dd>
      </div>
      <div>
        <dt>Windows? Linux?</dt>
        <dd>
          Not yet. The orc is training. Follow the <a href={LINKS.roadmap}>roadmap</a> to see when.
        </dd>
      </div>
      <div>
        <dt>Why does my Mac warn me the first time?</dt>
        <dd>
          OrcBlock isn't signed with a paid Apple developer certificate yet. The first time you open
          it, go to System Settings → Privacy &amp; Security and click Open Anyway. It then asks for
          your password once, to put the orc on duty.
        </dd>
      </div>
      <div>
        <dt>Does it see what I browse?</dt>
        <dd>
          No. Your list of sites and apps stays on your Mac and nothing is sent anywhere. The
          <a href={LINKS.privacy}>privacy page</a> has the whole story.
        </dd>
      </div>
      <div>
        <dt>How does it actually work?</dt>
        <dd>
          The nuts and bolts are in the <a href={LINKS.readme}>README on GitHub</a>. Short version:
          the orc is good at his job.
        </dd>
      </div>
    </dl>
  </div>
</section>

<style>
  dl {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    margin: 0;
  }
  dl > div {
    display: grid;
    align-content: start;
    gap: 10px;
    padding: var(--gutter) var(--gutter) 28px;
    border-bottom: var(--rule-thin) solid var(--rule);
  }
  dl > div:not(:nth-child(3n + 1)) {
    border-left: var(--rule-thin) solid var(--rule);
  }
  dl > div:nth-last-child(-n + 3) {
    border-bottom: none;
  }
  dt {
    font: 900 20px/1.05 var(--font-macro);
    letter-spacing: -0.02em;
    text-transform: uppercase;
  }
  dd {
    margin: 0;
    font: 400 15px/1.5 var(--font-macro);
  }
  @media (max-width: 900px) {
    dl {
      grid-template-columns: 1fr 1fr;
    }
    dl > div:not(:nth-child(3n + 1)) {
      border-left: none;
    }
    dl > div:nth-child(even) {
      border-left: var(--rule-thin) solid var(--rule);
    }
    dl > div:nth-last-child(-n + 3) {
      border-bottom: var(--rule-thin) solid var(--rule);
    }
    dl > div:nth-last-child(-n + 2) {
      border-bottom: none;
    }
  }
  @media (max-width: 560px) {
    dl {
      grid-template-columns: 1fr;
    }
    dl > div:nth-child(even) {
      border-left: none;
    }
    dl > div:nth-last-child(-n + 2) {
      border-bottom: var(--rule-thin) solid var(--rule);
    }
    dl > div:last-child {
      border-bottom: none;
    }
  }
</style>
