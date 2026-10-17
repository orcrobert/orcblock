export const REPO = "https://github.com/orcrobert/orcblock";

/** Always the newest published release: the release workflow uploads the dmg
    under this same name every time (docs/releasing.md). */
export const DOWNLOAD_MAC = `${REPO}/releases/latest/download/OrcBlock.dmg`;

export const LINKS = {
  repo: REPO,
  readme: `${REPO}#readme`,
  releases: `${REPO}/releases`,
  latest: `${REPO}/releases/latest`,
  roadmap: `${REPO}/blob/main/docs/roadmap.md`,
  security: `${REPO}/blob/main/SECURITY.md`,
  license: `${REPO}/blob/main/LICENSE`,
  issues: `${REPO}/issues`,
  privacy: "/privacy/",
} as const;
