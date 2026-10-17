<script lang="ts">
  import { THEMES, applyTheme, loadTheme, type ThemeChoice } from "../theme";

  const SHORT: Record<ThemeChoice, string> = { system: "SYS", light: "LGT", dark: "DRK" };

  let choice = $state<ThemeChoice>(loadTheme());

  function next() {
    const at = THEMES.findIndex((theme) => theme.value === choice);
    choice = THEMES[(at + 1) % THEMES.length]?.value ?? "system";
    applyTheme(choice);
  }
</script>

<button
  type="button"
  class="micro"
  onclick={next}
  aria-label="Theme: {THEMES.find((t) => t.value === choice)?.label}. Switch theme"
>
  ◐ {SHORT[choice]}
</button>

<style>
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
