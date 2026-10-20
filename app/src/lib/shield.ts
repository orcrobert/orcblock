// What the orc shield shows.
import type { Status } from "./api";

/** How long the shield stays before it closes itself. */
export const SHIELD_MS = 6_000;

export interface Shield {
  /** The app that was just closed. */
  name: string;
  /** Seconds left in the block. */
  remaining: number;
}

/** The newest closed app and the time left, or `null` when no block runs. */
export function shieldFor(status: Status): Shield | null {
  const launch = status.blocked_launches.at(-1);
  if (!status.block || !launch) return null;
  return { name: launch.name, remaining: status.block.remaining_secs };
}
