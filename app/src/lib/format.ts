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

/** The durations the picker steps through, in minutes: fine steps for
    short blocks, coarse ones for long blocks, 15 minutes to 7 days. */
export const DURATION_STOPS = [
  15, 20, 25, 30, 45, 60, 75, 90, 120, 150, 180, 240, 300, 360, 480, 600, 720, 1440, 2880, 4320,
  7200, 10080,
] as const;

export const DEFAULT_MINUTES = 60;

/** One-tap durations, in minutes. */
export const PRESETS = [25, 60, 120, 240, 480, 1440] as const;

/** The next stop above (`direction` 1) or below (-1) `minutes`, clamped to
    the ends of `DURATION_STOPS`. */
export function stepDuration(minutes: number, direction: 1 | -1): number {
  const first = DURATION_STOPS[0];
  const last = DURATION_STOPS[DURATION_STOPS.length - 1] ?? first;
  if (direction === 1) return DURATION_STOPS.find((stop) => stop > minutes) ?? last;
  return [...DURATION_STOPS].reverse().find((stop) => stop < minutes) ?? first;
}

/** The bounds of a block the helper accepts, in minutes. */
export const MIN_MINUTES = 1;
export const MAX_MINUTES = 10080;

const UNIT_MINUTES = { d: 1440, h: 60, m: 1 } as const;

/** Reads a typed duration: `1:30`, `90`, `90m`, `2h`, `1h30`, `2d 4h`.
    A bare number is minutes. `undefined` when it does not parse or lies
    outside `MIN_MINUTES`..`MAX_MINUTES`. */
export function parseDuration(text: string): number | undefined {
  const input = text.trim().toLowerCase();
  let total: number | undefined;
  const clock = /^(\d{1,3}):([0-5]\d)$/.exec(input);
  if (clock) {
    total = Number(clock[1]) * 60 + Number(clock[2]);
  } else if (/^\d{1,5}$/.test(input)) {
    total = Number(input);
  } else if (/^(\d{1,5}\s*[dhm]\s*)+$/.test(input) || /^\d{1,3}\s*h\s*\d{1,2}$/.test(input)) {
    const seen = new Set<string>();
    total = 0;
    for (const [, amount, unit = "m"] of input.matchAll(/(\d+)\s*([dhm])?/g)) {
      if (seen.has(unit)) return undefined;
      seen.add(unit);
      total += Number(amount) * UNIT_MINUTES[unit as keyof typeof UNIT_MINUTES];
    }
  }
  if (total === undefined || total < MIN_MINUTES || total > MAX_MINUTES) return undefined;
  return total;
}

/** `90` as `1:30`, `10080` as `168:00`. */
export function formatHoursMinutes(total: number): string {
  const h = Math.floor(total / 60);
  const m = total % 60;
  return `${h}:${m.toString().padStart(2, "0")}`;
}

/** `25M`, `1H`, `1D`: the compact label of a preset. */
export function presetLabel(total: number): string {
  if (total % 1440 === 0) return `${total / 1440}D`;
  if (total % 60 === 0) return `${total / 60}H`;
  return `${total}M`;
}

/** When a block of `minutes` started at `now` ends: `17:42`, or `Tue 17:42`
    when that is not today. */
export function formatEnds(now: Date, minutes: number): string {
  const end = new Date(now.getTime() + minutes * 60_000);
  const time = end.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
  if (end.toDateString() === now.toDateString()) return time;
  const day = end.toLocaleDateString(undefined, { weekday: "short" });
  return `${day} ${time}`;
}

/** `7` as `07`: row and unit numbers. */
export function pad2(n: number): string {
  return n.toString().padStart(2, "0");
}

/** An RFC 3339 timestamp as a local wall-clock time like `14:05`. */
export function formatClock(timestamp: string): string {
  return new Date(timestamp).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

/** An RFC 3339 timestamp as its local calendar day, `2026-10-17`. */
export function formatIsoDay(timestamp: string): string {
  const d = new Date(timestamp);
  return `${d.getFullYear()}-${pad2(d.getMonth() + 1)}-${pad2(d.getDate())}`;
}
