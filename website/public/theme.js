// Runs before the page paints, so a saved dark choice never flashes light.
// Light is the default; src/lib/theme.ts writes the choice.
try {
  if (localStorage.getItem("orcblock.site.theme") === "dark") {
    document.documentElement.dataset.theme = "dark";
    document.querySelector('meta[name="theme-color"]')?.setAttribute("content", "#121212");
  }
} catch {
  // Storage blocked: stay light.
}
