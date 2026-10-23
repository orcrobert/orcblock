import { describe, expect, it } from "vitest";

import {
  failed,
  offered,
  progressed,
  started,
  updateLabel,
  updateSpeech,
  type UpdateState,
} from "./update";

const offer = { version: "0.3.1", date: "2026-10-23" };
const downloading = (percent: number | null) => ({ type: "downloading", percent }) as const;
const installing = { type: "installing" } as const;

describe("update", () => {
  it("offers nothing without a newer release", () => {
    expect(offered(null)).toEqual({ kind: "none" });
    expect(updateLabel(offered(null))).toBeNull();
    expect(updateSpeech(offered(null))).toBe("");
  });

  it("walks from offer to install", () => {
    let state: UpdateState = offered(offer);
    expect(updateLabel(state)).toBe("UPDATE → 0.3.1");
    expect(updateSpeech(state)).toBe("Update to 0.3.1, released 2026-10-23");
    state = started(state);
    expect(updateLabel(state)).toBe("DOWNLOADING");
    state = progressed(state, downloading(42));
    expect(updateLabel(state)).toBe("DOWNLOADING 42%");
    expect(updateSpeech(state)).toBe("Downloading OrcBlock 0.3.1");
    state = progressed(state, installing);
    expect(updateLabel(state)).toBe("INSTALLING");
  });

  it("installs after a download of unknown size", () => {
    const state = progressed(progressed(started(offered(offer)), downloading(null)), installing);
    expect(state).toEqual({ kind: "installing", offer });
  });

  it("names the version without a date", () => {
    expect(updateSpeech(offered({ version: "0.3.1", date: null }))).toBe("Update to 0.3.1");
  });

  it.each([
    ["downloading", progressed(started(offered(offer)), downloading(42))],
    ["installing", progressed(started(offered(offer)), installing)],
  ])("offers the update again after failing while %s", (_name, state) => {
    expect(failed(state)).toEqual({ kind: "available", offer });
  });

  it("ignores clicks and progress that do not fit the state", () => {
    expect(started({ kind: "none" })).toEqual({ kind: "none" });
    expect(progressed(offered(offer), downloading(50))).toEqual(offered(offer));
    const done = progressed(started(offered(offer)), installing);
    expect(started(done)).toBe(done);
    expect(progressed(done, downloading(10))).toBe(done);
  });
});
