import { mount } from "svelte";
import "$app/theme.css";
import "./site.css";
import App from "./App.svelte";

const target = document.getElementById("site");
if (!target) throw new Error("index.html has no #site element");

export default mount(App, { target });
