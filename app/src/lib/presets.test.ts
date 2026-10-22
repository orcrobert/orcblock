import { describe, expect, it } from "vitest";

import { PRESETS, SOCIAL, presetFor } from "./presets";

// What the daemon keeps of a domain it is given: lowercase ASCII labels,
// no scheme, path or www.
const CLEAN = /^(?!www\.)([a-z0-9]([a-z0-9-]*[a-z0-9])?\.)+[a-z]{2,}$/;

describe.each(PRESETS.map((preset) => [preset.name, preset] as const))("%s", (_name, preset) => {
  it("lists clean main domains", () => {
    for (const site of preset.sites) expect(site).toMatch(CLEAN);
  });

  it("never lists a domain twice or one another covers", () => {
    for (const site of preset.sites) {
      const covering = preset.sites.filter((other) => site === other || site.endsWith(`.${other}`));
      expect(covering, site).toEqual([site]);
    }
  });

  it("fits a list name and says where it comes from", () => {
    expect(preset.name.length).toBeLessThanOrEqual(40);
    expect(preset.credit).not.toBe("");
    expect(preset.url).toMatch(/^https:\/\//);
    expect(preset.checked).toMatch(/^\d{4}-\d{2}-\d{2}$/);
  });
});

describe("presetFor", () => {
  const list = (name: string) => ({ id: 1, name, sites: [], apps: [] });

  it.each([
    ["same name", "Social media & video", SOCIAL],
    ["other case", "SOCIAL MEDIA & VIDEO", SOCIAL],
    ["renamed", "Evenings", undefined],
  ])("%s", (_case, name, want) => {
    expect(presetFor(list(name))).toBe(want);
  });
});
