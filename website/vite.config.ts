import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vitest/config";

const path = (relative: string) => decodeURIComponent(new URL(relative, import.meta.url).pathname);
const appLib = path("../app/src/lib");
const appIcons = path("../app/src-tauri/icons");

// The site reuses the app's tokens, orc and vials straight from app/src/lib,
// so it always looks like the app it advertises.
export default defineConfig({
  plugins: [svelte()],
  resolve: {
    alias: { $app: appLib, $icons: appIcons },
    // App components compile to imports of svelte's runtime; one copy only.
    dedupe: ["svelte"],
  },
  server: { fs: { allow: [".", appLib, appIcons] } },
  build: {
    target: "safari16",
    outDir: "dist",
    rollupOptions: {
      input: {
        main: path("index.html"),
        privacy: path("privacy/index.html"),
      },
    },
  },
  test: { include: ["src/**/*.test.ts"] },
});
