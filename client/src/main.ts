import { mount } from "svelte";
import App from "./App.svelte";
import { startWs } from "./lib/ws";
import "./app.css";

const app = mount(App, {
  target: document.getElementById("app")!,
});

/* Connects, syncs on open, resyncs on every reconnect. */
startWs();

export default app;
