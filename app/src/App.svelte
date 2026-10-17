<script lang="ts">
  import { onMount } from "svelte";
  import { api, isAppError, onStatus, type Status } from "./lib/api";
  import Snackbar from "./lib/components/Snackbar.svelte";
  import StatusBar from "./lib/components/StatusBar.svelte";
  import TopBar from "./lib/components/TopBar.svelte";
  import Home from "./routes/Home.svelte";
  import Setup from "./routes/Setup.svelte";

  type View = { kind: "loading" } | { kind: "setup" } | { kind: "ready"; status: Status };

  let view = $state<View>({ kind: "loading" });
  let message = $state("");

  function showError(error: unknown) {
    if (isAppError(error) && error.kind === "unavailable") {
      view = { kind: "setup" };
    } else if (isAppError(error)) {
      message = error.message;
    } else {
      message = String(error);
    }
  }

  onMount(() => {
    api.status().then((status) => (view = { kind: "ready", status }), showError);
    const unlisten = onStatus((event) => {
      if (event.type === "ok") view = { kind: "ready", status: event.status };
      else showError(event.error);
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  });
</script>

<TopBar version={__APP_VERSION__} />

<main>
  {#if view.kind === "ready"}
    <Home
      status={view.status}
      onstatus={(status) => (view = { kind: "ready", status })}
      onerror={(text) => (message = text)}
    />
  {:else if view.kind === "setup"}
    <Setup onerror={(text) => (message = text)} />
  {:else}
    <StatusBar armed={false} label="CONNECTING" />
  {/if}
</main>

<Snackbar bind:message />

<style>
  main {
    min-height: 0;
    overflow: hidden;
  }
</style>
