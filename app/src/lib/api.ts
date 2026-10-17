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

export interface Block {
  domains: string[];
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
  block: Block | null;
  potions: Potions;
}

export interface AppError {
  /** `unavailable` when the helper is not running, else the daemon's error kind. */
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
