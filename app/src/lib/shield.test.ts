import { describe, expect, it } from "vitest";

import type { BlockedLaunch, Status } from "./api";
import { shieldFor } from "./shield";

const launch = (id: number, name: string): BlockedLaunch => ({
  id,
  bundle_id: `com.example.${name.toLowerCase()}`,
  name,
  at: "2026-10-20T12:41:00Z",
});

const status = (launches: BlockedLaunch[], remaining: number | null): Status => ({
  daemon_version: "0.1.0",
  mode: "live",
  sites: [],
  apps: [],
  block:
    remaining === null
      ? null
      : {
          domains: [],
          apps: ["com.example.slack"],
          started_at: "2026-10-20T12:00:00Z",
          duration_secs: 3600,
          remaining_secs: remaining,
          ends_at: "2026-10-20T13:00:00Z",
        },
  blocked_launches: launches,
  potions: { available: 5, per_month: 5, refill_on: "2026-11-01" },
});

describe("shieldFor", () => {
  it("shows the newest closed app and the time left", () => {
    expect(shieldFor(status([launch(1, "Discord"), launch(2, "Slack")], 1140))).toEqual({
      name: "Slack",
      remaining: 1140,
    });
  });

  it.each([
    ["no launches", status([], 600)],
    ["block over", status([launch(1, "Slack")], null)],
  ])("is empty with %s", (_case, input) => {
    expect(shieldFor(input)).toBeNull();
  });
});
