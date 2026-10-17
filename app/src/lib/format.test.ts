import { describe, expect, it } from "vitest";

import {
  describeRemaining,
  formatCountdown,
  formatMinutes,
  DURATION_STOPS,
  durationParts,
  formatEnds,
  initial,
  potionsSummary,
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

describe("DURATION_STOPS", () => {
  it("rises from 15 minutes to 7 days", () => {
    expect(DURATION_STOPS[0]).toBe(15);
    expect(DURATION_STOPS.at(-1)).toBe(7 * 24 * 60);
    expect(DURATION_STOPS.every((m, i) => i === 0 || m > DURATION_STOPS[i - 1]!)).toBe(true);
  });
});

describe("durationParts", () => {
  it.each([
    [25, [{ value: 25, unit: "min" }]],
    [
      90,
      [
        { value: 1, unit: "h" },
        { value: 30, unit: "min" },
      ],
    ],
    [2880, [{ value: 2, unit: "d" }]],
  ])("%j", (minutes, want) => expect(durationParts(minutes)).toEqual(want));
});

describe("formatEnds", () => {
  const now = new Date(2026, 9, 17, 10, 0);
  const time = (d: Date) => d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
  it("shows only the time today", () => {
    expect(formatEnds(now, 90)).toBe(time(new Date(2026, 9, 17, 11, 30)));
  });
  it("adds the weekday on another day", () => {
    const ends = formatEnds(now, 1440);
    expect(ends.endsWith(time(now))).toBe(true);
    expect(ends.length).toBeGreaterThan(time(now).length);
  });
});

describe("initial", () => {
  it("uses the Unicode form", () => expect(initial("bücher.de")).toBe("B"));
});

describe("potionsSummary", () => {
  it("counts down to none", () => {
    expect(potionsSummary(3, 5)).toBe("3 of 5 left");
    expect(potionsSummary(0, 5)).toBe("None left this month");
  });
});
