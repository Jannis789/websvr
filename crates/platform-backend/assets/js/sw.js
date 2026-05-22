// Service Worker — Hash-Sync Interceptor & Registry
// Intercepts fetch('/sse') to append known_hashes for deduplication.
// Maintains an in-memory Hash Registry (TTL: 24h) of PatchElements hashes.
//
// Hash learning: The SSE endpoint embeds the HMAC hash as the event ID.
// This SW tees the response stream, parses SSE event IDs incrementally,
// and registers them in real time. On subsequent navigations, known_hashes
// is sent with the SSE request so the server can skip already-seen events.

const HASH_REGISTRY = new Map(); // hash → timestamp
const TTL_MS = 24 * 60 * 60 * 1000; // 24 hours
const MAX_REGISTRY_SIZE = 2000;
const EVENT_PREFIX = 'datastar-patch-elements';

self.addEventListener('install', (event) => {
  self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil(self.clients.claim());
});

// ── Helper: Parse SSE text incrementally ──
// Processes accumulated SSE text, extracts event IDs for PatchElements events,
// and registers them. Returns the remaining (incomplete) text.
let sseBuffer = '';
function processSSERead(buffer, text) {
  const data = buffer + text;
  const lines = data.split('\n');

  // The last line might be incomplete; keep it for the next chunk
  let eventType = null;
  let eventId = null;

  for (let i = 0; i < lines.length - 1; i++) {
    const line = lines[i];
    if (line.startsWith('event: ')) {
      eventType = line.slice(7).trim();
    } else if (line.startsWith('id: ')) {
      eventId = line.slice(4).trim();
    } else if (line === '' && eventType) {
      // End of an event — register if it's a PatchElements event with a hash ID
      if (eventType === EVENT_PREFIX && eventId && eventId.length > 0) {
        registerHash(eventId);
      }
      eventType = null;
      eventId = null;
    }
  }

  // Return the potentially incomplete last line
  return lines[lines.length - 1];
}

// ── Helper: Register a single hash ──
function registerHash(hash) {
  HASH_REGISTRY.set(hash, Date.now());

  // Limit registry size (keep latest MAX_REGISTRY_SIZE)
  if (HASH_REGISTRY.size > MAX_REGISTRY_SIZE) {
    const entries = [...HASH_REGISTRY.entries()]
      .sort((a, b) => a[1] - b[1]);
    const toDelete = entries.slice(0, entries.length - MAX_REGISTRY_SIZE);
    for (const [h] of toDelete) {
      HASH_REGISTRY.delete(h);
    }
  }
}

// ── Helper: Clean expired hashes ──
function cleanExpiredHashes() {
  const now = Date.now();
  for (const [hash, ts] of HASH_REGISTRY) {
    if (now - ts > TTL_MS) {
      HASH_REGISTRY.delete(hash);
    }
  }
}

// ── Helper: Read SSE stream incrementally ──
// Reads from the cloned stream in a background loop, never blocking the
// page's stream. Hashes are registered as events arrive.
async function consumeSSEStream(stream) {
  const reader = stream.getReader();
  const decoder = new TextDecoder();
  let buffer = '';

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;

      const chunk = decoder.decode(value, { stream: true });
      buffer = processSSERead(buffer, chunk);
    }
  } catch (err) {
    // Stream closed or errored — silently ignore
    console.debug('SW: SSE stream consume ended:', err.message);
  } finally {
    reader.releaseLock();
  }
}

// ── Fetch Intercept ──
self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);

  // Only intercept /sse requests
  if (url.pathname === '/sse') {
    cleanExpiredHashes();

    // Append known_hashes to URL
    const known = Array.from(HASH_REGISTRY.keys()).join(',');
    const sseUrl = known.length > 0
      ? `${url.origin}${url.pathname}?known_hashes=${known}`
      : `${url.origin}${url.pathname}`;

    event.respondWith(
      fetch(sseUrl).then(async (response) => {
        // Tee the stream: one for the page (Datastar), one for the SW (hash parsing)
        if (response.body) {
          const [pageStream, swStream] = response.body.tee();

          // Parse the SW stream in the background (don't await — let it run)
          consumeSSEStream(swStream);

          // Return a new response with the page's stream
          return new Response(pageStream, {
            status: response.status,
            statusText: response.statusText,
            headers: response.headers,
          });
        }

        return response;
      })
    );
  }
});

// ── Test exports (only in test environment) ──
if (typeof globalThis.__SW_TEST_MODE !== 'undefined') {
  self.__sw = {
    HASH_REGISTRY,
    processSSERead,
    registerHash,
    cleanExpiredHashes,
    consumeSSEStream,
    TTL_MS,
    MAX_REGISTRY_SIZE,
  };
}
