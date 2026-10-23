<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorMessage, onUpdateProgress } from "../api";
  import {
    failed,
    offered,
    progressed,
    started,
    updateLabel,
    updateSpeech,
    type UpdateState,
  } from "../update";

  let { onerror }: { onerror: (message: string) => void } = $props();

  let update = $state<UpdateState>({ kind: "none" });
  let label = $derived(updateLabel(update));
  let speech = $derived(updateSpeech(update));
  // Only the step is announced, never each percent.
  let announced = $derived(
    update.kind === "downloading" || update.kind === "installing" ? speech : "",
  );

  onMount(() => {
    // A failed check is logged by the app and not worth an error here: the
    // app works as before.
    api.checkUpdate().then(
      (offer) => (update = offered(offer)),
      () => {},
    );
    const unlisten = onUpdateProgress((progress) => (update = progressed(update, progress)));
    return () => {
      void unlisten.then((stop) => stop());
    };
  });

  async function install() {
    if (update.kind !== "available") return;
    update = started(update);
    try {
      await api.installUpdate();
    } catch (error) {
      update = failed(update);
      onerror(errorMessage(error));
    }
  }
</script>

{#if update.kind !== "none"}
  <!-- aria-disabled rather than disabled: the button keeps focus while it reports progress. -->
  <button
    class="micro"
    aria-disabled={update.kind !== "available"}
    aria-label={speech}
    title={speech}
    onclick={install}
  >
    {label}
  </button>
{/if}
<span class="visually-hidden" role="status" aria-live="polite">{announced}</span>

<style>
  button {
    flex: none;
    min-width: 20ch;
    height: 24px;
    padding: 0 8px;
    border: var(--rule-thin) solid var(--ink);
    background: var(--paper);
    color: var(--ink);
    white-space: nowrap;
    cursor: pointer;
  }
  button[aria-disabled="false"]:hover {
    background: var(--ink);
    color: var(--paper);
  }
  button[aria-disabled="false"]:active {
    transform: translateY(1px);
  }
  button[aria-disabled="true"] {
    background: repeating-linear-gradient(-45deg, var(--hatch) 0 2px, transparent 2px 7px);
    cursor: default;
  }
</style>
