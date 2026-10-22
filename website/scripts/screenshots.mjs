// Renders the real app UI with sample data and saves the site's screenshots:
//   npm --prefix website run screenshots
// It builds app/ (the same Svelte code the app ships), serves it locally,
// stands in for the Tauri bridge with fixed data and captures the window
// content at 900 x 520 points, 2x, in light and dark, as WebP. Needs a Chromium-based
// browser (Chrome, Brave, Edge, Arc, Chromium); set CHROME to pick one.

import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { extname, join, normalize } from "node:path";

const root = fileURLToPath(new URL("../..", import.meta.url));
const appDist = join(root, "app", "dist");
const outDir = join(root, "website", "src", "screenshots");
const WIDTH = 900;
const HEIGHT = 520;

const BROWSERS = [
  process.env.CHROME,
  "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
  "/Applications/Chromium.app/Contents/MacOS/Chromium",
  "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser",
  "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge",
  "/Applications/Arc.app/Contents/MacOS/Arc",
  "/usr/bin/google-chrome",
  "/usr/bin/chromium",
].filter(Boolean);

const SITES = ["youtube.com", "reddit.com", "x.com", "news.ycombinator.com", "instagram.com"];
const APPS = [
  { bundle_id: "com.tinyspeck.slackmacgap", name: "Slack" },
  { bundle_id: "com.hnc.discord", name: "Discord" },
];

function sampleStatus() {
  const now = new Date();
  const duration = 2 * 3600;
  const remaining = 3600 + 29 * 60 + 42;
  const startedAt = new Date(now.getTime() - (duration - remaining) * 1000);
  const endsAt = new Date(now.getTime() + remaining * 1000);
  const refill = new Date(now.getFullYear(), now.getMonth() + 1, 1);
  const day = (date) =>
    `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, "0")}-${String(date.getDate()).padStart(2, "0")}`;
  return {
    daemon_version: "0.0.0",
    mode: "live",
    lists: [
      {
        id: 1,
        name: "Distractions",
        sites: SITES.map((domain, i) => ({
          domain,
          display: domain,
          added_at: new Date(now.getTime() - (i + 2) * 86_400_000).toISOString(),
        })),
        apps: APPS.map((app, i) => ({
          ...app,
          added_at: new Date(now.getTime() - (i + 1) * 86_400_000).toISOString(),
        })),
      },
      { id: 2, name: "Evenings", sites: [], apps: [] },
    ],
    block: {
      lists: [1],
      domains: SITES,
      apps: APPS.map((app) => app.bundle_id),
      started_at: startedAt.toISOString(),
      duration_secs: duration,
      remaining_secs: remaining,
      ends_at: endsAt.toISOString(),
    },
    blocked_launches: [],
    potions: { available: 3, per_month: 5, refill_on: day(refill) },
  };
}

const ICON_TYPES = new Set([
  "image/png",
  "image/x-icon",
  "image/vnd.microsoft.icon",
  "image/svg+xml",
]);

/** Best effort: the real favicons make the list look like the app's. */
async function favicons() {
  const icons = {};
  for (const domain of SITES) {
    for (const host of [domain, `www.${domain}`]) {
      try {
        const response = await fetch(`https://${host}/favicon.ico`, {
          signal: AbortSignal.timeout(5000),
        });
        const type = (response.headers.get("content-type") ?? "").split(";")[0] ?? "";
        const size = Number(response.headers.get("content-length") ?? 0);
        if (response.ok && ICON_TYPES.has(type) && size <= 256 * 1024) {
          const bytes = Buffer.from(await response.arrayBuffer());
          icons[domain] = `data:${type};base64,${bytes.toString("base64")}`;
          break;
        }
      } catch {
        // No icon: the row shows the app's blank square, as it would offline.
      }
    }
  }
  return icons;
}

/** Runs before the app: a stand-in for the Tauri bridge the app calls into. */
function bridge(status, icons, theme) {
  return `
    localStorage.setItem("orcblock.theme", ${JSON.stringify(theme)});
    const status = ${JSON.stringify(status)};
    const icons = ${JSON.stringify(icons)};
    window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
    window.__TAURI_INTERNALS__ = {
      metadata: { currentWindow: { label: "main" }, currentWebview: { windowLabel: "main", label: "main" } },
      transformCallback: () => 1,
      unregisterCallback() {},
      convertFileSrc: (path) => path,
      async invoke(cmd, args) {
        if (cmd === "status") return status;
        if (cmd === "favicon") return icons[args.domain] ?? null;
        if (cmd === "app_icon") return null;
        if (cmd.startsWith("plugin:event|")) return 1;
        throw { kind: "unsupported", message: cmd };
      },
    };
  `;
}

const TYPES = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".css": "text/css",
  ".png": "image/png",
  ".svg": "image/svg+xml",
};

function serve(dir) {
  const server = createServer((request, response) => {
    // Only the headless browser should get here; a rebinding page would
    // arrive under another Host.
    if (request.headers.host !== `127.0.0.1:${server.address().port}`) {
      response.writeHead(403).end();
      return;
    }
    try {
      const path = normalize(decodeURIComponent(new URL(request.url, "http://x").pathname));
      const file = join(dir, path === "/" ? "index.html" : path);
      if (!file.startsWith(`${dir}/`) || !existsSync(file) || !statSync(file).isFile()) {
        response.writeHead(404).end();
        return;
      }
      const type = TYPES[extname(file)] ?? "application/octet-stream";
      response.writeHead(200, { "content-type": type });
      response.end(readFileSync(file));
    } catch {
      response.writeHead(400).end();
    }
  });
  return new Promise((resolve) =>
    server.listen(0, "127.0.0.1", () => resolve({ server, port: server.address().port })),
  );
}

function launch(browser, profile) {
  const child = spawn(
    browser,
    [
      "--headless=new",
      "--remote-debugging-port=0",
      `--user-data-dir=${profile}`,
      "--no-first-run",
      "--no-default-browser-check",
      "--hide-scrollbars",
      "about:blank",
    ],
    { stdio: ["ignore", "ignore", "pipe"] },
  );
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      child.kill();
      reject(new Error(`${browser} did not open a DevTools port in 20 s`));
    }, 20_000);
    let log = "";
    child.stderr.on("data", (chunk) => {
      log += chunk;
      const match = log.match(/DevTools listening on (ws:\/\/\S+)/);
      if (match) {
        clearTimeout(timer);
        resolve({ child, endpoint: match[1] });
      }
    });
    child.on("exit", (code) => reject(new Error(`${browser} exited (${code}):\n${log}`)));
  });
}

/** A minimal Chrome DevTools Protocol client over the built-in WebSocket. */
function connect(endpoint) {
  const socket = new WebSocket(endpoint);
  const pending = new Map();
  const waiters = [];
  let next = 1;
  socket.addEventListener("message", (event) => {
    const message = JSON.parse(event.data);
    if (message.id && pending.has(message.id)) {
      const { resolve, reject } = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) reject(new Error(message.error.message));
      else resolve(message.result);
    } else if (message.method) {
      for (const waiter of waiters.splice(0)) {
        if (waiter.method === message.method) waiter.resolve(message.params);
        else waiters.push(waiter);
      }
    }
  });
  const send = (method, params = {}, sessionId) =>
    new Promise((resolve, reject) => {
      const id = next++;
      pending.set(id, { resolve, reject });
      socket.send(JSON.stringify({ id, method, params, sessionId }));
    });
  const once = (method) => new Promise((resolve) => waiters.push({ method, resolve }));
  return new Promise((resolve) =>
    socket.addEventListener("open", () => resolve({ send, once, close: () => socket.close() })),
  );
}

async function capture(cdp, url, script, theme) {
  const { targetId } = await cdp.send("Target.createTarget", { url: "about:blank" });
  const { sessionId } = await cdp.send("Target.attachToTarget", { targetId, flatten: true });
  const send = (method, params) => cdp.send(method, params, sessionId);
  await send("Page.enable");
  await send("Emulation.setDeviceMetricsOverride", {
    width: WIDTH,
    height: HEIGHT,
    deviceScaleFactor: 2,
    mobile: false,
  });
  // Reduced motion holds the orc and the progress bar still for the photo.
  await send("Emulation.setEmulatedMedia", {
    features: [
      { name: "prefers-color-scheme", value: theme },
      { name: "prefers-reduced-motion", value: "reduce" },
    ],
  });
  await send("Page.addScriptToEvaluateOnNewDocument", { source: script });
  const loaded = cdp.once("Page.loadEventFired");
  await send("Page.navigate", { url });
  await loaded;
  await new Promise((resolve) => setTimeout(resolve, 1200));
  // Never overwrite a good screenshot with an error page.
  const { result } = await send("Runtime.evaluate", {
    expression: "document.querySelector('.screen') !== null",
  });
  if (result.value !== true) throw new Error(`the app did not render at ${url}`);
  const { data } = await send("Page.captureScreenshot", { format: "webp", quality: 92 });
  await cdp.send("Target.closeTarget", { targetId });
  return Buffer.from(data, "base64");
}

const browser = BROWSERS.find((path) => existsSync(path));
if (!browser) {
  console.error("No Chromium-based browser found; set CHROME=/path/to/chrome.");
  process.exit(1);
}

const build = spawnSync("npm", ["--prefix", join(root, "app"), "run", "build"], {
  stdio: "inherit",
});
if (build.status !== 0) process.exit(build.status ?? 1);

const status = sampleStatus();
const icons = await favicons();
const { server, port } = await serve(appDist);
const profile = mkdtempSync(join(tmpdir(), "orcblock-shots-"));
const { child, endpoint } = await launch(browser, profile);

try {
  const cdp = await connect(endpoint);
  for (const theme of ["light", "dark"]) {
    const url = `http://127.0.0.1:${port}/`;
    const image = await capture(cdp, url, bridge(status, icons, theme), theme);
    const file = join(outDir, `app-${theme}.webp`);
    writeFileSync(file, image);
    console.log(`wrote ${file}`);
  }
  cdp.close();
} finally {
  const exited = new Promise((resolve) => child.once("exit", resolve));
  child.kill();
  await exited;
  server.close();
  rmSync(profile, { recursive: true, force: true });
}
