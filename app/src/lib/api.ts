// Typed wrappers around the Tauri commands; the shapes mirror orcblock-ipc.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

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

export interface Block {
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
  sites: Site[];
  apps: App[];
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
  addSite: (site: string) => invoke<Status>("add_site", { site }),
  removeSite: (site: string) => invoke<Status>("remove_site", { site }),
  startBlock: (minutes: number) => invoke<Status>("start_block", { minutes }),
  drinkPotion: () => invoke<Status>("drink_potion"),
  favicon: (domain: string) => invoke<string | null>("favicon", { domain }),
  addApp: (bundleId: string, name: string) => invoke<Status>("add_app", { bundleId, name }),
  removeApp: (bundleId: string) => invoke<Status>("remove_app", { bundleId }),
  installedApps: () => invoke<InstalledApp[]>("installed_apps"),
  runningApps: () => invoke<string[]>("running_apps"),
  appIcon: (bundleId: string) => invoke<string | null>("app_icon", { bundleId }),
  appFromPath: (path: string) => invoke<InstalledApp>("app_from_path", { path }),
  pickAppInFinder: () => invoke<InstalledApp | null>("pick_app_in_finder"),
  installHelper: () => invoke<void>("install_helper"),
};

export function onStatus(handler: (event: StatusEvent) => void): Promise<UnlistenFn> {
  return listen<StatusEvent>("status", (event) => handler(event.payload));
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
