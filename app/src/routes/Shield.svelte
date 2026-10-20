<script lang="ts">
  import { onMount } from "svelte";
  import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
  import { api, onStatus } from "../lib/api";
  import Button from "../lib/components/Button.svelte";
  import { describeRemaining, formatCountdown } from "../lib/format";
  import Orc from "../lib/mascot/Orc.svelte";
  import { prefersReducedMotion } from "../lib/motion";
  import { SHIELD_MS, shieldFor, type Shield } from "../lib/shield";

  const win = getCurrentWebviewWindow();

  let shield = $state<Shield | null>(null);
  let round = $state(0);
  /**
   * The pointer is over the shield, or a key was pressed in it: it stays
   * until the pointer leaves, or until OK for a keyboard or VoiceOver user.
   */
  let paused = $state(false);
  let ok: HTMLElement | undefined = $state();
  const still = prefersReducedMotion();

  function close() {
    void win.close();
  }

  // Each new closed app, and the pointer leaving, restarts the timer; the
  // strip restarts with it.
  $effect(() => {
    void round;
    if (paused) return;
    const timer = setTimeout(close, SHIELD_MS);
    return () => clearTimeout(timer);
  });

  function resume() {
    paused = false;
    round += 1;
  }

  onMount(() => {
    api.status().then(
      (status) => (shield = shieldFor(status) ?? shield),
      () => {},
    );
    const stopStatus = onStatus((event) => {
      if (event.type === "ok") shield = shieldFor(event.status) ?? shield;
    });
    const stopShield = win.listen("shield", () => {
      round += 1;
      void api.status().then(
        (status) => (shield = shieldFor(status) ?? shield),
        () => {},
      );
    });
    ok?.querySelector("button")?.focus();
    return () => {
      void stopStatus.then((stop) => stop());
      void stopShield.then((stop) => stop());
    };
  });
</script>

<svelte:window
  onkeydown={(event) => {
    if (event.key === "Escape") close();
    else paused = true;
  }}
/>

<div
  class="shield"
  role="alertdialog"
  tabindex="-1"
  aria-labelledby="shield-title"
  aria-describedby="shield-time"
  onpointerenter={() => (paused = true)}
  onpointerleave={resume}
>
  <p class="tag micro" data-tauri-drag-region>[ BLOCKED // ORC-01 ]</p>
  <div class="body">
    <figure>
      <Orc pose="guarding" scale={4} />
    </figure>
    <div class="text">
      <h1 id="shield-title" aria-live="assertive" aria-atomic="true">
        {#if shield}
          <span class="app">{shield.name}</span> is blocked
        {:else}
          Blocked
        {/if}
      </h1>
      {#if shield}
        <p id="shield-time" class="micro">
          <span class="muted">TIME LEFT</span>
          <span class="tabular" aria-hidden="true">{formatCountdown(shield.remaining)}</span>
          <span class="visually-hidden">{describeRemaining(shield.remaining)}</span>
        </p>
      {:else}
        <p id="shield-time" class="micro muted">THE ORC CLOSED IT</p>
      {/if}
    </div>
  </div>
  {#key round}
    <div
      class="timer"
      class:still
      class:paused
      style:--shield-ms="{SHIELD_MS}ms"
      aria-hidden="true"
    ></div>
  {/key}
  <footer bind:this={ok}>
    <Button onclick={close}><span>OK</span><span>&gt;&gt;&gt;</span></Button>
  </footer>
</div>

<style>
  .shield {
    display: grid;
    grid-template-rows: auto 1fr 4px 56px;
    height: 100vh;
    border: var(--rule-thick) solid var(--ink);
    background: var(--paper);
    color: var(--ink);
  }
  .tag {
    cursor: grab;
    user-select: none;
    margin: 0;
    padding: 8px var(--gutter);
    background: var(--red);
    color: var(--on-red);
  }
  .body {
    display: grid;
    grid-template-columns: auto 1fr;
    align-items: center;
    gap: var(--gutter);
    padding: 0 var(--gutter);
  }
  figure {
    margin: 0;
  }
  h1 {
    margin: 0 0 10px;
    font: var(--type-headline);
    font-size: 30px;
    letter-spacing: var(--tracking-macro);
    text-transform: uppercase;
    overflow-wrap: anywhere;
  }
  .app {
    display: -webkit-box;
    overflow: hidden;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 2;
    line-clamp: 2;
    color: var(--red);
  }
  p {
    margin: 0;
  }
  .tabular {
    font: var(--type-data);
  }
  .timer {
    border-top: var(--rule-thin) solid var(--rule);
    background: var(--red);
    transform-origin: left;
    animation: drain var(--shield-ms) linear forwards;
  }
  .timer.still {
    animation: none;
  }
  .timer.paused {
    animation-play-state: paused;
  }
  @keyframes drain {
    to {
      transform: scaleX(0);
    }
  }
  footer {
    display: grid;
    border-top: var(--rule-thick) solid var(--rule);
  }
</style>
