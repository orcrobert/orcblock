// Which list the targets panel shows and which lists the next block covers.
// Both are remembered per Mac in localStorage; the daemon only learns the
// chosen lists when a block starts.
import type { List } from "./api";

const CURRENT_KEY = "orcblock.list";
const SKIPPED_KEY = "orcblock.skipped-lists";

/**
 * The list to show: the remembered one while it exists, else the first.
 * The daemon always reports at least one list.
 */
export function currentList(lists: List[], remembered: number | null): List {
  return lists.find((list) => list.id === remembered) ?? lists[0]!;
}

/**
 * The lists the next block covers. The app remembers the lists left out
 * rather than the ones chosen, so a new list is in the next block until
 * unticked. A single list is always chosen: it has no tick to undo a skip.
 */
export function chosenLists(lists: List[], skipped: ReadonlySet<number>): List[] {
  if (lists.length === 1) return lists;
  return lists.filter((list) => !skipped.has(list.id));
}

/** How many distinct sites and apps the lists hold together. */
export function targetCount(lists: List[]): number {
  const domains = new Set(lists.flatMap((list) => list.sites.map((site) => site.domain)));
  const apps = new Set(lists.flatMap((list) => list.apps.map((app) => app.bundle_id)));
  return domains.size + apps.size;
}

/** The Start button's label: which lists a block would cover. */
export function armLabel(lists: List[], chosen: List[]): string {
  if (lists.length === 1) return "ARM // START BLOCK";
  if (targetCount(chosen) === 0) return "ARM // TICK A LIST WITH TARGETS";
  if (chosen.length === lists.length) return "ARM // ALL LISTS";
  return `ARM // ${chosen.map((list) => list.name).join(" + ")}`;
}

export function loadCurrent(): number | null {
  try {
    const stored = Number(localStorage.getItem(CURRENT_KEY));
    return Number.isInteger(stored) && stored > 0 ? stored : null;
  } catch {
    return null;
  }
}

export function saveCurrent(id: number): void {
  try {
    localStorage.setItem(CURRENT_KEY, String(id));
  } catch {
    // Not remembered; the first list shows next time.
  }
}

export function loadSkipped(): Set<number> {
  try {
    const stored: unknown = JSON.parse(localStorage.getItem(SKIPPED_KEY) ?? "[]");
    return new Set(Array.isArray(stored) ? stored.filter(Number.isInteger) : []);
  } catch {
    return new Set();
  }
}

/** Remembers the unticked lists, dropping IDs of lists that are gone. */
export function saveSkipped(skipped: ReadonlySet<number>, lists: List[]): void {
  const kept = lists.filter((list) => skipped.has(list.id)).map((list) => list.id);
  try {
    localStorage.setItem(SKIPPED_KEY, JSON.stringify(kept));
  } catch {
    // Not remembered; every list is ticked next time.
  }
}
