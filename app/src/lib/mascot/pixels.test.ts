import { describe, expect, it } from "vitest";

import { pixelRuns } from "./pixels";

describe("pixelRuns", () => {
  it.each([
    ["empty row", ["...."], []],
    ["one run", [".##."], [{ x: 1, y: 0, width: 2, pixel: "#" }]],
    [
      "colour change splits a run",
      ["#pp#"],
      [
        { x: 0, y: 0, width: 1, pixel: "#" },
        { x: 1, y: 0, width: 2, pixel: "p" },
        { x: 3, y: 0, width: 1, pixel: "#" },
      ],
    ],
    [
      "rows keep their y",
      ["#.", ".#"],
      [
        { x: 0, y: 0, width: 1, pixel: "#" },
        { x: 1, y: 1, width: 1, pixel: "#" },
      ],
    ],
  ])("%s", (_name, rows, want) => expect(pixelRuns(rows)).toEqual(want));
});
