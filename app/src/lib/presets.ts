// Starter lists the app offers when a list is created. Each one names the
// source it is based on; docs/specs/0005-lists.md says why these sites and
// not others. They ship with the app and are updated by hand.
import type { List } from "./api";

export interface Preset {
  name: string;
  /** Main domains only: the daemon blocks their subdomains too. */
  sites: readonly string[];
  /** Shown under the list name as "BASED ON …". */
  credit: string;
  url: string;
  /** When the sites were last checked against the source. */
  checked: string;
}

export const SOCIAL: Preset = {
  name: "Social media & video",
  sites: [
    "youtube.com",
    "facebook.com",
    "instagram.com",
    "tiktok.com",
    "x.com",
    "twitter.com",
    "reddit.com",
    "pinterest.com",
    "snapchat.com",
    "threads.com",
    "threads.net",
    "bsky.app",
  ],
  credit: "Pew Research Center, Social Media Fact Sheet, 2026",
  url: "https://www.pewresearch.org/internet/fact-sheet/social-media/",
  checked: "2026-10-22",
};

export const PRESETS: readonly Preset[] = [SOCIAL];

/** The preset a list was made from, judged by its name; renaming drops it. */
export function presetFor(list: List): Preset | undefined {
  const name = list.name.toLowerCase();
  return PRESETS.find((preset) => preset.name.toLowerCase() === name);
}
