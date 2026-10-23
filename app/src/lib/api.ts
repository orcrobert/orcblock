// Typed wrappers around the Tauri commands; the shapes mirror orcblock-ipc.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type { Offer, Progress } from "./update";

export interface Site {
  /** Cleaned ASCII domain, used for commands and favicons. */
  domain: string;
  /** The domain as people read it. */
  display: string;
  added_at: string;
}

/** An app on the list. */
export interface App {
  /** Lowercase bundle ID, used for commands and icons. */
  bundle_id: string;
  name: string;
  added_at: string;
}

/** An app the orc closed during the block, newest last. */
export interface BlockedLaunch {
  /** Grows with every launch, so the shield shows each one once. */
  id: number;
  bundle_id: string;
  name: string;
  at: string;
}

/** An app found on this Mac for the picker. */
export interface InstalledApp {
  bundle_id: string;
  name: string;
  path: string;
}

/** A named list of sites and apps. */
export interface List {
  /** Assigned by the daemon and never reused. */
  id: number;
  name: string;
  sites: Site[];
  apps: App[];
}

export interface Block {
  /** The lists the block was started over; additions to them join it. */
  lists: number[];
  domains: string[];
  apps: string[];
  started_at: string;
  duration_secs: number;
  remaining_secs: number;
  ends_at: string;
}

export interface Potions {
  available: number;
  per_month: number;
  /** First day of the next refill, `YYYY-MM-DD`. */
  refill_on: string;
}

export interface Status {
  daemon_version: string;
  mode: "live" | "dry_run";
  lists: List[];
  block: Block | null;
  blocked_launches: BlockedLaunch[];
  potions: Potions;
}

export interface AppError {
  /**
   * `unavailable` when the helper is not running, `helper_outdated` when it
   * speaks an older protocol, else the daemon's error kind.
   */
  kind: string;
  message: string;
}

export type StatusEvent = { type: "ok"; status: Status } | { type: "err"; error: AppError };

export const api = {
  status: () => invoke<Status>("status"),
  createList: (name: string, sites: string[] = []) =>
    invoke<Status>("create_list", { name, sites }),
  renameList: (list: number, name: string) => invoke<Status>("rename_list", { list, name }),
  deleteList: (list: number) => invoke<Status>("delete_list", { list }),
  addSite: (list: number, site: string) => invoke<Status>("add_site", { list, site }),
  removeSite: (list: number, site: string) => invoke<Status>("remove_site", { list, site }),
  startBlock: (lists: number[], minutes: number) =>
    invoke<Status>("start_block", { lists, minutes }),
  drinkPotion: () => invoke<Status>("drink_potion"),
  favicon: (domain: string) => invoke<string | null>("favicon", { domain }),
  addApp: (list: number, bundleId: string, name: string) =>
    invoke<Status>("add_app", { list, bundleId, name }),
  removeApp: (list: number, bundleId: string) => invoke<Status>("remove_app", { list, bundleId }),
  installedApps: () => invoke<InstalledApp[]>("installed_apps"),
  runningApps: () => invoke<string[]>("running_apps"),
  appIcon: (bundleId: string) => invoke<string | null>("app_icon", { bundleId }),
  appFromPath: (path: string) => invoke<InstalledApp>("app_from_path", { path }),
  pickAppInFinder: () => invoke<InstalledApp | null>("pick_app_in_finder"),
  installHelper: () => invoke<void>("install_helper"),
  checkUpdate: () => invoke<Offer | null>("check_update"),
  /** Resolves only on failure: on success the app relaunches. */
  installUpdate: () => invoke<void>("install_update"),
};

export function onStatus(handler: (event: StatusEvent) => void): Promise<UnlistenFn> {
  return listen<StatusEvent>("status", (event) => handler(event.payload));
}

/** Progress of an update install. */
export function onUpdateProgress(handler: (progress: Progress) => void): Promise<UnlistenFn> {
  return listen<Progress>("update_progress", (event) => handler(event.payload));
}

export function isAppError(value: unknown): value is AppError {
  return (
    typeof value === "object" &&
    value !== null &&
    typeof (value as AppError).kind === "string" &&
    typeof (value as AppError).message === "string"
  );
}

export function errorMessage(value: unknown): string {
  return isAppError(value) ? value.message : String(value);
}
