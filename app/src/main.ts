import { mount } from "svelte";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import App from "./App.svelte";
import Shield from "./routes/Shield.svelte";
import "./lib/theme.css";
import { applyTheme, loadTheme } from "./lib/theme";

applyTheme(loadTheme());

const target = document.getElementById("app");
if (!target) throw new Error("index.html has no #app element");

// The shield is a second window on the same page, told apart by its label.
function isShield(): boolean {
  try {
    return getCurrentWebviewWindow().label === "shield";
  } catch {
    // Outside Tauri, as in a plain browser preview.
    return false;
  }
}

const shield = isShield();

export default mount(shield ? Shield : App, { target });
