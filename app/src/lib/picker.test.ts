import { describe, expect, it } from "vitest";

import type { InstalledApp } from "./api";
import { matchRank, monogram, pickable } from "./picker";

const app = (name: string, bundle_id: string): InstalledApp => ({
  name,
  bundle_id,
  path: `/Applications/${name}.app`,
});

const slack = app("Slack", "com.tinyspeck.slackmacgap");
const steam = app("Steam", "com.valvesoftware.steam");
const vscode = app("Visual Studio Code", "com.microsoft.vscode");
const discord = app("Discord", "com.hnc.discord");
const messages = app("Messages", "com.apple.mobilesms");
const all = [vscode, steam, discord, slack, messages];

const names = (apps: InstalledApp[]) => apps.map((a) => a.name);

describe("matchRank", () => {
  it.each([
    ["empty query", slack, "  ", 0],
    ["name prefix", slack, "sl", 0],
    ["any case", slack, "SL", 0],
    ["word prefix", vscode, "stu", 1],
    ["inside the name", discord, "cor", 2],
    ["bundle id only", messages, "mobilesms", 3],
    ["no match", slack, "zoom", null],
  ])("%s", (_case, entry, query, want) => {
    expect(matchRank(entry, query)).toBe(want);
  });
});

describe("pickable", () => {
  const none = new Set<string>();

  it("sorts by name without a query", () => {
    expect(names(pickable(all, "", none, none))).toEqual([
      "Discord",
      "Messages",
      "Slack",
      "Steam",
      "Visual Studio Code",
    ]);
  });

  it("puts open apps first within a rank", () => {
    const running = new Set([steam.bundle_id, vscode.bundle_id]);
    expect(names(pickable(all, "", none, running))).toEqual([
      "Steam",
      "Visual Studio Code",
      "Discord",
      "Messages",
      "Slack",
    ]);
  });

  it("ranks prefix over word over substring over bundle id", () => {
    const apps = [
      app("Toast", "com.example.toast"),
      app("Hot Stuff", "com.example.hot"),
      app("Stack", "com.example.stack"),
      app("Mail", "com.example.st"),
    ];
    const running = new Set(["com.example.toast"]);
    expect(names(pickable(apps, "st", none, running))).toEqual([
      "Stack",
      "Hot Stuff",
      "Toast",
      "Mail",
    ]);
  });

  it("hides apps already listed", () => {
    const listed = new Set([slack.bundle_id]);
    expect(names(pickable(all, "s", listed, none))).toEqual([
      "Steam",
      "Visual Studio Code",
      "Discord",
      "Messages",
    ]);
  });
});

describe("monogram", () => {
  it.each([
    ["Visual Studio Code", "VS"],
    ["Slack", "SL"],
    ["zoom.us", "ZU"],
    ["X", "X"],
    ["  ", "?"],
  ])("%s", (name, want) => {
    expect(monogram(name)).toBe(want);
  });
});
