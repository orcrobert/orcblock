<script lang="ts">
  import Download from "./components/Download.svelte";
  import Escape from "./components/Escape.svelte";
  import FinePrint from "./components/FinePrint.svelte";
  import Footer from "./components/Footer.svelte";
  import Hero from "./components/Hero.svelte";
  import HowItWorks from "./components/HowItWorks.svelte";
  import MeetOrc from "./components/MeetOrc.svelte";
  import Potions from "./components/Potions.svelte";
  import SiteBar from "./components/SiteBar.svelte";
  import { currentPlatform } from "./lib/platform";

  const platform = currentPlatform();
</script>

<SiteBar />
<main>
  <Hero {platform} />
  <HowItWorks />
  <Escape />
  <Potions />
  <MeetOrc />
  <FinePrint />
  <Download {platform} />
</main>
<Footer />
