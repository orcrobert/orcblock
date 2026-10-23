<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorMessage, onUpdateProgress } from "../api";
  import { failed, offered, progressed, started, updateLabel, type UpdateState } from "../update";

  let { onerror }: { onerror: (message: string) => void } = $props();

  let update = $state<UpdateState>({ kind: "none" });
  let label = $derived(updateLabel(update));

  onMount(() => {
    // A failed check is not worth an error: the app works as before.
    api.checkUpdate().then(
      (offer) => (update = offered(offer)),
      (error: unknown) => console.warn("update check failed", error),
    );
    const unlisten = onUpdateProgress((percent) => (update = progressed(update, percent)));
    return () => {
      void unlisten.then((stop) => stop());
    };
  });

  async function install() {
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
  <button
    class="micro"
    disabled={update.kind !== "available"}
    title={update.offer.date ? `RELEASED ${update.offer.date}` : undefined}
    onclick={install}
  >
    {label}
  </button>
{/if}

<style>
  button {
    height: 24px;
    padding: 0 8px;
    border: var(--rule-thin) solid var(--ink);
    background: var(--paper);
    color: var(--ink);
    cursor: pointer;
  }
  button:not(:disabled):hover {
    background: var(--ink);
    color: var(--paper);
  }
  button:disabled {
    background: repeating-linear-gradient(-45deg, var(--hatch) 0 2px, transparent 2px 7px);
    color: var(--ink-muted);
    cursor: default;
  }
</style>
