// Pure formatting helpers for the UI.

/** `2:14:09` or `4:05`. */
export function formatCountdown(totalSecs: number): string {
  const secs = Math.max(0, Math.floor(totalSecs));
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = secs % 60;
  const pad = (n: number) => n.toString().padStart(2, "0");
  return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${m}:${pad(s)}`;
}

/** `25 min`, `1 h`, `1 h 30 min`, `2 d 4 h`. */
export function formatMinutes(total: number): string {
  const d = Math.floor(total / 1440);
  const h = Math.floor((total % 1440) / 60);
  const m = total % 60;
  const parts: string[] = [];
  if (d > 0) parts.push(`${d} d`);
  if (h > 0) parts.push(`${h} h`);
  if (m > 0 && d === 0) parts.push(`${m} min`);
  return parts.length > 0 ? parts.join(" ") : "0 min";
}

/** A screen-reader friendly, minute-precision description of time left. */
export function describeRemaining(totalSecs: number): string {
  const minutes = Math.ceil(Math.max(0, totalSecs) / 60);
  if (minutes <= 1) return "Less than a minute left";
  const h = Math.floor(minutes / 60);
  const m = minutes % 60;
  const plural = (n: number, word: string) => `${n} ${word}${n === 1 ? "" : "s"}`;
  const parts = [h > 0 ? plural(h, "hour") : "", m > 0 ? plural(m, "minute") : ""].filter(Boolean);
  return `${parts.join(" ")} left`;
}

/** Durations people type: `90`, `25m`, `2h`, `1h30m`, `1d`. Minutes, or null. */
export function parseMinutes(input: string): number | null {
  const text = input.trim().toLowerCase().replace(/\s+/g, "");
  if (text === "") return null;
  if (/^\d+$/.test(text)) return Number(text);
  const match = /^(?:(\d+)d)?(?:(\d+)h)?(?:(\d+)m(?:in)?)?$/.exec(text);
  if (!match || match[0] === "") return null;
  const [, d = "0", h = "0", m = "0"] = match;
  return Number(d) * 1440 + Number(h) * 60 + Number(m);
}

/** The shortest and longest block the daemon accepts, in minutes. */
export const MIN_MINUTES = 1;
export const MAX_MINUTES = 7 * 24 * 60;

export interface Monogram {
  letter: string;
  hue: number;
}

/** The first letter of a site and a stable hue derived from its domain. */
export function monogram(domain: string, display: string = domain): Monogram {
  let hash = 0;
  for (const char of domain) hash = (hash * 31 + char.codePointAt(0)!) >>> 0;
  const letter = [...display][0]?.toLocaleUpperCase() ?? "?";
  return { letter, hue: hash % 360 };
}

export function potionsLabel(available: number): string {
  if (available === 0) return "No sleep potions left this month";
  return `${available} sleep potion${available === 1 ? "" : "s"} left this month`;
}

/** `2026-11-01` as `Nov 1`. */
export function formatDay(isoDate: string): string {
  const [year, month, day] = isoDate.split("-").map(Number);
  if (!year || !month || !day) return isoDate;
  return new Date(year, month - 1, day).toLocaleDateString(undefined, {
    month: "short",
    day: "numeric",
  });
}

/** An RFC 3339 timestamp as a local wall-clock time like `14:05`. */
export function formatClock(timestamp: string): string {
  return new Date(timestamp).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}
