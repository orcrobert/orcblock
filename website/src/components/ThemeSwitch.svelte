<script lang="ts">
  import { applyTheme, currentTheme } from "../lib/theme";

  let theme = $state(currentTheme());

  function toggle() {
    theme = theme === "light" ? "dark" : "light";
    applyTheme(theme);
  }
</script>

<button
  type="button"
  class="micro"
  onclick={toggle}
  aria-label="Switch to {theme === 'light' ? 'dark' : 'light'} theme"
>
  ◐ {theme === "light" ? "LGT" : "DRK"}
</button>

<style>
  /* The app's theme switch, with two states instead of three. */
  button {
    height: 40px;
    padding: 0 var(--gutter);
    border: none;
    border-left: var(--rule-thin) solid var(--rule);
    background: transparent;
    cursor: pointer;
  }
  button:hover {
    background: var(--ink);
    color: var(--paper);
  }
</style>
