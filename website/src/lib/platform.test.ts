import { describe, expect, it } from "vitest";
import { detectPlatform, type Platform } from "./platform";

const cases: { name: string; ua: string; hint?: string; want: Platform }[] = [
  {
    name: "safari on macos",
    ua: "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15",
    want: "mac",
  },
  { name: "chrome hint on macos", ua: "Mozilla/5.0", hint: "macOS", want: "mac" },
  {
    name: "edge on windows",
    ua: "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0 Safari/537.36 Edg/130.0",
    want: "windows",
  },
  {
    name: "firefox on linux",
    ua: "Mozilla/5.0 (X11; Linux x86_64; rv:131.0) Gecko/20100101 Firefox/131.0",
    want: "linux",
  },
  { name: "chromebook", ua: "Mozilla/5.0 (X11; CrOS x86_64 14541.0.0)", want: "linux" },
  {
    name: "android is not linux",
    ua: "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 Chrome/130.0 Mobile Safari/537.36",
    want: "mobile",
  },
  {
    name: "iphone is not a mac",
    ua: "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Mobile/15E148",
    want: "mobile",
  },
  { name: "darwin is not windows", ua: "OrcBlock/1 CFNetwork/1568 Darwin/24.0.0", want: "unknown" },
  { name: "nothing to go on", ua: "", want: "unknown" },
];

describe("detectPlatform", () => {
  for (const { name, ua, hint, want } of cases) {
    it(name, () => expect(detectPlatform(ua, hint)).toBe(want));
  }
});
