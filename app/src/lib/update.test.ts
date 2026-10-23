import { describe, expect, it } from "vitest";

import { failed, offered, progressed, started, updateLabel, type UpdateState } from "./update";

const offer = { version: "0.3.1", date: "2026-10-23" };

describe("update", () => {
  it("offers nothing without a newer release", () => {
    expect(offered(null)).toEqual({ kind: "none" });
    expect(updateLabel(offered(null))).toBeNull();
  });

  it("walks from offer to install", () => {
    let state: UpdateState = offered(offer);
    expect(updateLabel(state)).toBe("UPDATE → 0.3.1");
    state = started(state);
    expect(updateLabel(state)).toBe("DOWNLOADING");
    state = progressed(state, 42);
    expect(updateLabel(state)).toBe("DOWNLOADING 42%");
    state = progressed(state, 100);
    expect(updateLabel(state)).toBe("INSTALLING");
  });

  it.each([
    ["downloading", progressed(started(offered(offer)), 42)],
    ["installing", progressed(started(offered(offer)), 100)],
  ])("offers the update again after failing while %s", (_name, state) => {
    expect(failed(state)).toEqual({ kind: "available", offer });
  });

  it("ignores clicks and progress that do not fit the state", () => {
    expect(started({ kind: "none" })).toEqual({ kind: "none" });
    expect(progressed(offered(offer), 50)).toEqual(offered(offer));
    const installing = progressed(started(offered(offer)), 100);
    expect(started(installing)).toBe(installing);
    expect(progressed(installing, 10)).toBe(installing);
  });
});
