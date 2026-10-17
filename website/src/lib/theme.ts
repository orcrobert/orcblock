// The site's theme: light by default, dark on request. Unlike the app it
// does not follow the system. public/theme.js applies the saved choice
// before the first paint.

export type Theme = "light" | "dark";

const KEY = "orcblock.site.theme";

export function currentTheme(): Theme {
  return document.documentElement.dataset.theme === "dark" ? "dark" : "light";
}

export function applyTheme(theme: Theme): void {
  document.documentElement.dataset.theme = theme;
  const paper = getComputedStyle(document.documentElement).getPropertyValue("--paper").trim();
  document.querySelector('meta[name="theme-color"]')?.setAttribute("content", paper);
  try {
    localStorage.setItem(KEY, theme);
  } catch {
    // Not saved; the choice still applies until the page is left.
  }
}
