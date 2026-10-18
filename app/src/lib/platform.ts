// The desktop system the app runs on, read from the webview's user agent
// (WebView2 on Windows, WKWebView on macOS) and exposed to CSS as
// data-platform on <html>. Only the app sets it; the website, which shares
// these components, gets the macOS look, which is also the default.

export type Platform = "mac" | "windows";

export function detectPlatform(userAgent: string): Platform {
  return userAgent.includes("Windows") ? "windows" : "mac";
}

export const PLATFORM: Platform = detectPlatform(
  typeof navigator === "undefined" ? "" : navigator.userAgent,
);

export function applyPlatform(
  platform: Platform = PLATFORM,
  root: { dataset: DOMStringMap } = document.documentElement,
): void {
  root.dataset.platform = platform;
}
