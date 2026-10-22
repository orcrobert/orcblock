import { describe, expect, it } from "vitest";

import type { List } from "./api";
import { armLabel, chosenLists, currentList, targetCount } from "./lists";

const site = (domain: string) => ({ domain, display: domain, added_at: "2026-10-22T09:00:00Z" });
const app = (bundle_id: string) => ({
  bundle_id,
  name: bundle_id,
  added_at: "2026-10-22T09:00:00Z",
});
const list = (id: number, name: string, sites: string[] = [], apps: string[] = []): List => ({
  id,
  name,
  sites: sites.map(site),
  apps: apps.map(app),
});

const mine = list(1, "My list", ["youtube.com"], ["com.hnc.discord"]);
const work = list(2, "Work", ["slack.com", "youtube.com"]);
const empty = list(3, "Empty");
const all = [mine, work, empty];

describe("currentList", () => {
  it.each([
    ["remembered", 2, work],
    ["nothing remembered", null, mine],
    ["deleted since", 9, mine],
  ])("%s", (_case, remembered, want) => {
    expect(currentList(all, remembered)).toBe(want);
  });
});

describe("chosenLists", () => {
  it("leaves out skipped lists and keeps new ones", () => {
    expect(chosenLists(all, new Set([2]))).toEqual([mine, empty]);
    expect(chosenLists(all, new Set([9]))).toEqual(all);
  });
});

describe("targetCount", () => {
  it("counts a site on two lists once", () => {
    expect(targetCount([mine, work])).toBe(3);
    expect(targetCount([empty])).toBe(0);
  });
});

describe("armLabel", () => {
  it.each([
    ["one list", [mine], [mine], "ARM // START BLOCK"],
    ["all lists", all, all, "ARM // ALL LISTS"],
    ["some lists", all, [mine, work], "ARM // My list + Work"],
    ["none", all, [], "ARM // TICK A LIST"],
  ])("%s", (_case, lists, chosen, want) => {
    expect(armLabel(lists, chosen)).toBe(want);
  });
});
