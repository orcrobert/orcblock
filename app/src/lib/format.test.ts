import { describe, expect, it } from "vitest";

import {
  describeRemaining,
  formatCountdown,
  formatMinutes,
  monogram,
  parseMinutes,
  potionsLabel,
} from "./format";

describe("formatCountdown", () => {
  it.each([
    [0, "0:00"],
    [65, "1:05"],
    [3600, "1:00:00"],
    [8049, "2:14:09"],
    [-5, "0:00"],
  ])("%d s is %s", (secs, want) => expect(formatCountdown(secs)).toBe(want));
});

describe("formatMinutes", () => {
  it.each([
    [25, "25 min"],
    [60, "1 h"],
    [90, "1 h 30 min"],
    [1440, "1 d"],
    [1440 + 240 + 5, "1 d 4 h"],
  ])("%d min is %s", (minutes, want) => expect(formatMinutes(minutes)).toBe(want));
});

describe("describeRemaining", () => {
  it("rounds up to minutes and pluralises", () => {
    expect(describeRemaining(30)).toBe("Less than a minute left");
    expect(describeRemaining(61)).toBe("2 minutes left");
    expect(describeRemaining(3600)).toBe("1 hour left");
    expect(describeRemaining(8049)).toBe("2 hours 15 minutes left");
  });
});

describe("parseMinutes", () => {
  it.each([
    ["90", 90],
    ["25m", 25],
    ["25 min", 25],
    ["2h", 120],
    ["1h30m", 90],
    ["1d", 1440],
    ["", null],
    ["h", null],
    ["1.5h", null],
    ["soon", null],
  ])("%j is %j", (input, want) => expect(parseMinutes(input)).toBe(want));
});

describe("monogram", () => {
  it("uses the display letter and a stable hue", () => {
    const a = monogram("xn--bcher-kva.de", "bücher.de");
    expect(a.letter).toBe("B");
    expect(monogram("xn--bcher-kva.de", "bücher.de")).toEqual(a);
    expect(a.hue).toBeGreaterThanOrEqual(0);
    expect(a.hue).toBeLessThan(360);
  });
});

describe("potionsLabel", () => {
  it("pluralises", () => {
    expect(potionsLabel(0)).toBe("No sleep potions left this month");
    expect(potionsLabel(1)).toBe("1 sleep potion left this month");
    expect(potionsLabel(5)).toBe("5 sleep potions left this month");
  });
});
