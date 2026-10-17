export type Platform = "mac" | "windows" | "linux" | "mobile" | "unknown";

/** The visitor's platform from `navigator.userAgentData.platform` when the
    browser offers it, else from the user agent string. iPads that report
    themselves as Macs count as Macs; they can still send the link on. */
export function detectPlatform(userAgent: string, hint?: string): Platform {
  const source = `${hint ?? ""} ${userAgent}`.toLowerCase();
  if (/android|iphone|ipod|ipad|mobile/.test(source)) return "mobile";
  if (/mac/.test(source)) return "mac";
  if (/\bwin(dows|32|64)\b/.test(source)) return "windows";
  if (/linux|x11|cros/.test(source)) return "linux";
  return "unknown";
}

interface UserAgentData {
  platform?: string;
}

export function currentPlatform(): Platform {
  if (typeof navigator === "undefined") return "unknown";
  const data = (navigator as Navigator & { userAgentData?: UserAgentData }).userAgentData;
  return detectPlatform(navigator.userAgent, data?.platform);
}
