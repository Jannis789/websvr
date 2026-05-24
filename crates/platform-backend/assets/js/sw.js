// Service Worker — Hash-Sync Interceptor & Event Cache
//
// Caches SSE events (raw text) keyed by hash for deduplication.
// On reload: clears the registry, lets the server send the full
// initial state fresh — no stale navigation events can leak through.
// For live navigation: passes events through and registers hashes
// so the server can skip already-seen events on reconnect.

const HASH_REGISTRY = new Map(); // hash → { rawEvent, timestamp }
const MAX_REGISTRY_SIZE = 2000;
const EVENT_PREFIX = 'datastar-patch-elements';

self.addEventListener('install', () => {
  HASH_REGISTRY.clear();
  self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil(self.clients.claim());
});

// ── Stream-scoped SSE learner ──
// Closure-scoped state per stream — no module-level mutables.
// Parses SSE text to extract PatchElements event IDs, stores raw event text.
function createStreamLearner() {
  let buffer = '';
  let pendingType = null;
  let pendingId = null;
  let rawLines = [];

  return function feed(text) {
    buffer += text;
    const lines = buffer.split('\n');
    buffer = lines.pop(); // keep incomplete last line

    for (const line of lines) {
      if (line === '') {
        // End of SSE event
        if (pendingType === EVENT_PREFIX && pendingId) {
          HASH_REGISTRY.set(pendingId, {
            rawEvent: rawLines.join('\n') + '\n\n',
            timestamp: Date.now(),
          });
          if (HASH_REGISTRY.size > MAX_REGISTRY_SIZE) {
            HASH_REGISTRY.delete(HASH_REGISTRY.keys().next().value);
          }
        }
        pendingType = null;
        pendingId = null;
        rawLines = [];
      } else {
        rawLines.push(line);
        if (line.startsWith('event: ')) pendingType = line.slice(7).trim();
        else if (line.startsWith('id: ')) pendingId = line.slice(4).trim();
      }
    }
  };
}

// ── Fetch Intercept ──
self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);

  if (url.pathname === '/sse') {
    // Grab known hashes BEFORE clearing — server needs them to skip
    // events that are still in-flight (edge case: fast reconnect).
    // On full page reload the registry is already empty from install,
    // so known will be empty and the server sends everything.
    const known = Array.from(HASH_REGISTRY.keys()).join(',');

    // Clear registry on every SSE reconnect.
    // The server will send the full initial state fresh.
    // Old navigation events must not leak into the replay.
    HASH_REGISTRY.clear();

    const sseUrl = known.length > 0
      ? `${url.origin}${url.pathname}?known_hashes=${encodeURIComponent(known)}`
      : `${url.origin}${url.pathname}`;

    event.respondWith(
      fetch(sseUrl).then((response) => {
        if (!response.body) return response;

        const decoder = new TextDecoder();
        const learn = createStreamLearner();

        // No replay prefix — server sends everything fresh.
        // Just learn new events as they come through.
        const passthrough = new ReadableStream({
          async start(controller) {
            const reader = response.body.getReader();
            const encoder = new TextEncoder();
            try {
              while (true) {
                const { done, value } = await reader.read();
                if (done) { controller.close(); return; }
                learn(decoder.decode(value, { stream: true }));
                controller.enqueue(value);
              }
            } catch (_) {
              try { controller.close(); } catch (__) {}
            }
          }
        });

        return new Response(passthrough, {
          status: response.status,
          statusText: response.statusText,
          headers: response.headers,
        });
      })
    );
  }
});

// ── Test exports ──
if (typeof globalThis.__SW_TEST_MODE !== 'undefined') {
  self.__sw = {
    HASH_REGISTRY,
    createStreamLearner,
    MAX_REGISTRY_SIZE,
    EVENT_PREFIX,
  };
}
