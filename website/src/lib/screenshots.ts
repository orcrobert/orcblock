// The app screenshots on the site. To refresh them after a release, run
// `npm --prefix website run screenshots`, or replace the two files with new
// captures of the window content at the same size; see website/README.md.
// Vite fingerprints them, so a new capture is never hidden by a cache.

import dark from "../screenshots/app-dark.webp";
import light from "../screenshots/app-light.webp";

export const SCREENSHOTS = { light, dark, width: 1800, height: 1040 } as const;
