/* Minimal service worker: network-only, exists for PWA installability.
 * No caching — every request passes straight through. */
self.addEventListener("fetch", (event) => {
  event.respondWith(fetch(event.request));
});
