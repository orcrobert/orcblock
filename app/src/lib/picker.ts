// Filtering and ordering for the app picker.
import type { InstalledApp } from "./api";

/** How well an app matches the query; lower is better, `null` is no match. */
export function matchRank(app: InstalledApp, query: string): number | null {
  const q = query.trim().toLowerCase();
  if (q === "") return 0;
  const name = app.name.toLowerCase();
  if (name.startsWith(q)) return 0;
  if (name.split(/[\s.\-_]+/).some((word) => word.startsWith(q))) return 1;
  if (name.includes(q)) return 2;
  if (app.bundle_id.includes(q)) return 3;
  return null;
}

/**
 * The apps to offer: those not listed yet that match `query`, best match
 * first, open apps before the rest within a rank, then by name.
 */
export function pickable(
  apps: InstalledApp[],
  query: string,
  listed: ReadonlySet<string>,
  running: ReadonlySet<string>,
): InstalledApp[] {
  return apps
    .filter((app) => !listed.has(app.bundle_id))
    .map((app) => ({ app, rank: matchRank(app, query) }))
    .filter((entry): entry is { app: InstalledApp; rank: number } => entry.rank !== null)
    .sort(
      (a, b) =>
        a.rank - b.rank ||
        Number(running.has(b.app.bundle_id)) - Number(running.has(a.app.bundle_id)) ||
        a.app.name.localeCompare(b.app.name, undefined, { sensitivity: "base" }),
    )
    .map((entry) => entry.app);
}

/** Two letters for an app without an icon: `Visual Studio Code` is `VS`. */
export function monogram(name: string): string {
  const [first = "?", second] = name
    .trim()
    .split(/[\s.\-_]+/)
    .filter(Boolean);
  const letters = second ? first.charAt(0) + second.charAt(0) : first.slice(0, 2);
  return letters.toUpperCase();
}
