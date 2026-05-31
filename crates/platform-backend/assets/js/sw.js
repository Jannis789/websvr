// Service Worker — minimal pass-through
// Event caching happens page-side via Datastar's 'datastar-fetch' lifecycle event.

self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (e) => e.waitUntil(self.clients.claim()));

self.addEventListener('fetch', (event) => {
  event.respondWith(fetch(event.request));
});
