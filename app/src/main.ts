import { mount } from "svelte";
import App from "./App.svelte";
import "./lib/theme.css";
import { applyTheme, loadTheme } from "./lib/theme";

applyTheme(loadTheme());

const target = document.getElementById("app");
if (!target) throw new Error("index.html has no #app element");

export default mount(App, { target });
