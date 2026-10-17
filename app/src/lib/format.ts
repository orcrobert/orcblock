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

/** `90` as `[{ value: 1, unit: "h" }, { value: 30, unit: "min" }]`, for a
    readout that sets numbers larger than units. */
export function durationParts(total: number): { value: number; unit: string }[] {
  return formatMinutes(total)
    .split(/ (?=\d)/)
    .map((part) => {
      const [value = "0", unit = ""] = part.split(" ");
      return { value: Number(value), unit };
    });
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

/** The first letter of a site as people read it. */
export function initial(display: string): string {
  return [...display][0]?.toLocaleUpperCase() ?? "?";
}

/** `3 of 5 left`. */
export function potionsSummary(available: number, perMonth: number): string {
  return available === 0 ? "None left this month" : `${available} of ${perMonth} left`;
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

/** An RFC 3339 timestamp as a short local date like `Oct 17`. */
export function formatDate(timestamp: string): string {
  return new Date(timestamp).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}
