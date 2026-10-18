import { describe, expect, it } from "vitest";
import { detectPlatform } from "./platform";

describe("detectPlatform", () => {
  it.each([
    [
      "WebView2 on Windows 11",
      "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/140.0.0.0 Safari/537.36 Edg/140.0.0.0",
      "windows",
    ],
    [
      "WKWebView on macOS",
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko)",
      "mac",
    ],
    ["nothing to go on", "", "mac"],
  ])("%s", (_name, userAgent, want) => {
    expect(detectPlatform(userAgent)).toBe(want);
  });
});
