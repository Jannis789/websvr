// Service Worker — Hash-Sync Interceptor & Event Cache
//
// Caches complete SSE events (raw text) keyed by their hash.
// On reload, replays cached events to Datastar, sends known_hashes
// to the server so it skips already-seen events, and passes the
// live response through unchanged.
//
// The server buffers only the initial page state. Navigation events
// (should_cache: false) are broadcast but not buffered. On reload the
// server always replays the correct initial state, which overwrites any
// stale navigation events from the SW cache (last patch wins).

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
    const known = Array.from(HASH_REGISTRY.keys()).join(',');
    const sseUrl = known.length > 0
      ? `${url.origin}${url.pathname}?known_hashes=${encodeURIComponent(known)}`
      : `${url.origin}${url.pathname}`;

    event.respondWith(
      fetch(sseUrl).then((response) => {
        if (!response.body) return response;

        const encoder = new TextEncoder();
        const decoder = new TextDecoder();
        const learn = createStreamLearner();

        // Replay prefix: concatenate all cached raw events
        const cached = [];
        for (const entry of HASH_REGISTRY.values()) {
          cached.push(entry.rawEvent);
        }

        const combined = new ReadableStream({
          async start(controller) {
            // 1. Replay cached events
            if (cached.length > 0) {
              controller.enqueue(encoder.encode(cached.join('')));
            }
            // 2. Pipe live response through, learning new events
            const reader = response.body.getReader();
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

        return new Response(combined, {
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
