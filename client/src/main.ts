import { mount } from "svelte";
import App from "./App.svelte";
import { startWs } from "./lib/ws";
import "./app.css";

const app = mount(App, {
  target: document.getElementById("app")!,
});

/* Connects, syncs on open, resyncs on every reconnect. */
startWs();

/* Register a service worker for web page as app */
if ("serviceWorker" in navigator) {
  navigator.serviceWorker.register("/sw.js").catch((e) => {
    console.warn("service worker registration failed:", e);
  });
}

export default app;
