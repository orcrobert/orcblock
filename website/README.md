# OrcBlock website

The download page: a static site built with Vite and Svelte 5, deployed to Cloudflare Pages by `.github/workflows/website.yml`. It reuses the app's design tokens (`app/src/lib/theme.css`), the orc, the vials and the icons straight from `app/`, so it changes with the app.

The site talks about what OrcBlock does, not how. Technical detail belongs in the repository README; link to it.

## Run it

```sh
npm ci --prefix app        # the site imports the app's components
npm ci --prefix website
make site-dev              # http://localhost:5173
make site-check            # types, tests, formatting
make site-build            # website/dist
```

## Where things are

| What                                        | Where                                                   |
| ------------------------------------------- | ------------------------------------------------------- |
| Sections of the page                        | `src/components/*.svelte`, in order in `src/App.svelte` |
| Every outbound link, including the download | `src/lib/links.ts`                                      |
| Privacy page (static HTML)                  | `privacy/index.html`                                    |
| App screenshots                             | `src/screenshots/app-light.webp`, `app-dark.webp`       |
| Cloudflare headers, robots, 404             | `public/`                                               |

The macOS button always downloads the newest published release: it links to `releases/latest/download/OrcBlock.dmg`, the stable asset name the release workflow uploads (docs/releasing.md). Windows and Linux show "coming soon"; when a release ships one, add its stable asset link to `links.ts` and make its card a download in `Download.svelte` and `HeroDownload.svelte`.

## Refresh the screenshots

After a release that changes how the app looks:

```sh
make site-screenshots
```

It builds the app's UI from `app/`, renders it in a headless Chromium-based browser (Chrome, Brave, Edge, Arc or Chromium; set `CHROME=/path/to/browser` to pick one) with sample data and writes both theme variants to `src/screenshots/`. Commit them; the deploy picks them up.

To use your own captures instead, replace the two files with WebP images of the window _content_ (no title bar or shadow) at 1800 × 1040 px, a 900 × 520 pt window at 2x. The site draws the macOS frame and traffic lights itself. Different size or format: update `src/lib/screenshots.ts`. File names get a content hash at build time, so a new screenshot is never hidden by a cache.

## Deploy

Every push to `main` that touches `website/`, `app/src/lib/`, `app/src-tauri/icons/`, `app/package.json` or the workflow builds the site and uploads `website/dist` to Cloudflare Pages. Pull requests touching those paths only build and check. Run it by hand from the Actions tab (**website** → **Run workflow**) after changing the Cloudflare settings.

### One-time Cloudflare setup

Do this after the first release made by the release workflow is published: until then the download link has nothing to point at.

1. **Create the Pages project** (Direct Upload, not Git: GitHub Actions does the deploying).
   Cloudflare dashboard → **Workers & Pages** → **Create** → **Pages** → **Upload assets** (Direct Upload). Name it `orcblock`, upload any placeholder file (for example `website/public/404.html`) and deploy. Or from a terminal: `npx wrangler pages project create orcblock --production-branch main`.
   Another name works too: set it as the repository variable `CLOUDFLARE_PAGES_PROJECT` (below).
2. **Create an API token.** My Profile → **API Tokens** → **Create Token** → **Custom token**. Permission: **Account** · **Cloudflare Pages** · **Edit**. Account resources: your account only. No zone permissions needed.
3. **Find your account ID.** Workers & Pages overview, right-hand column (**Account ID**), or the URL `dash.cloudflare.com/<account id>/...`.
4. **Add them to GitHub.** Repository → Settings → **Environments** → **New environment** named `website`. Under Deployment branches and tags choose **Selected branches and tags** and add `main`. Add `CLOUDFLARE_API_TOKEN` and `CLOUDFLARE_ACCOUNT_ID` as secrets _of this environment_, never as repository secrets: environment secrets only reach jobs that deploy from `main`. Optional: repository variable `CLOUDFLARE_PAGES_PROJECT` if the project isn't called `orcblock`.
5. **Deploy.** Merge to `main`, or run the workflow by hand. The site appears at `https://orcblock.pages.dev`.
6. **Your domain.** Pages project → **Custom domains** → **Set up a custom domain** → enter it (for example `orcblock.app` and `www.orcblock.app`). If the domain's DNS is on Cloudflare, it adds the records itself; otherwise add the `CNAME` it shows at your registrar. HTTPS is automatic. To send `www` to the bare domain (or back), add a **Redirect Rule** under the zone's Rules.

Nothing in the build depends on the domain, so changing it needs no code change.
