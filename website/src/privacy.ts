import { mount } from "svelte";
import "$app/theme.css";
import "./site.css";
import Footer from "./components/Footer.svelte";
import SiteBar from "./components/SiteBar.svelte";

// The policy itself is static HTML so it reads without JavaScript; only the
// shared bar and footer are mounted around it.
const bar = document.getElementById("bar");
const footer = document.getElementById("footer");
if (bar) mount(SiteBar, { target: bar });
if (footer) mount(Footer, { target: footer });
