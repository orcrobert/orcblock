import { describe, expect, it } from "vitest";

import {
  describeRemaining,
  formatCountdown,
  formatMinutes,
  DURATION_STOPS,
  formatEnds,
  formatHoursMinutes,
  formatIsoDay,
  pad2,
  parseDuration,
  presetLabel,
  stepDuration,
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

describe("stepDuration", () => {
  it.each([
    [60, 1, 75],
    [60, -1, 45],
    [61, -1, 60],
    [15, -1, 15],
    [10080, 1, 10080],
    [5, 1, 15],
  ] as const)("%i stepped %i is %i", (minutes, direction, want) =>
    expect(stepDuration(minutes, direction)).toBe(want),
  );
});

describe("formatHoursMinutes", () => {
  it.each([
    [25, "0:25"],
    [90, "1:30"],
    [10080, "168:00"],
  ])("%i is %s", (minutes, want) => expect(formatHoursMinutes(minutes)).toBe(want));
});

describe("presetLabel", () => {
  it.each([
    [25, "25M"],
    [120, "2H"],
    [1440, "1D"],
  ])("%i is %s", (minutes, want) => expect(presetLabel(minutes)).toBe(want));
});

describe("pad2 and formatIsoDay", () => {
  it("pads and uses the local day", () => {
    expect(pad2(7)).toBe("07");
    expect(formatIsoDay(new Date(2026, 9, 7, 23, 30).toISOString())).toBe("2026-10-07");
  });
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

describe("parseDuration", () => {
  it.each([
    ["clock", "1:30", 90],
    ["clock with spaces", " 2:05 ", 125],
    ["clock past a day", "36:00", 2160],
    ["bare number is minutes", "45", 45],
    ["minutes suffix", "90m", 90],
    ["hours suffix", "2h", 120],
    ["hours and minutes", "1h30", 90],
    ["hours and minutes with suffixes", "1H 30M", 90],
    ["days", "1d", 1440],
    ["days and hours", "2d 4h", 3120],
    ["shortest", "1", 1],
    ["longest", "7d", 10080],
  ])("%s: %s is %i minutes", (_name, text, want) => expect(parseDuration(text)).toBe(want));

  it.each([
    ["empty", ""],
    ["zero", "0:00"],
    ["over seven days", "7d 1m"],
    ["minutes over 59 on a clock", "1:75"],
    ["letters", "soon"],
    ["negative", "-5"],
    ["fraction", "1.5h"],
    ["duplicate unit", "1h 2h"],
  ])("%s: %s is rejected", (_name, text) => expect(parseDuration(text)).toBeUndefined());
});
