// Service Worker — Pass-through SSE Interceptor
//
// Intercepts /sse requests so future hash-based dedup can be added.
// Currently a pure pass-through — no caching, no replay, no hash tracking.
// The server-side EventEmitter buffer handles state replay on reconnect.

self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (e) => e.waitUntil(self.clients.claim()));

self.addEventListener('fetch', (event) => {
  if (new URL(event.request.url).pathname === '/sse') {
    event.respondWith(fetch(event.request));
  }
});

// ── Test exports ──
if (typeof globalThis.__SW_TEST_MODE !== 'undefined') {
  self.__sw = {};
}
