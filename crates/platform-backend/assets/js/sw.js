// Service Worker — Hash-Sync Interceptor & Payload Cache
// Intercepts fetch('/sse') to append known_hashes for deduplication.
// Caches full event payloads so it can restore them on page reload.
//
// Hash learning: The SSE endpoint embeds the HMAC hash as the event ID.
// This SW tees the response stream, parses SSE event IDs + payloads incrementally,
// and registers them in real time. On subsequent navigations, known_hashes
// is sent with the SSE request so the server can skip already-seen events,
// and the SW replays the cached payloads locally before streaming new ones.

const HASH_REGISTRY = new Map(); // hash → { payload, eventType, timestamp }
const TTL_MS = 24 * 60 * 60 * 1000; // 24 hours
const MAX_REGISTRY_SIZE = 2000;
const EVENT_PREFIX = 'datastar-patch-elements';

self.addEventListener('install', (event) => {
  HASH_REGISTRY.clear();
  self.skipWaiting();
});

self.addEventListener('activate', (event) => {
  event.waitUntil(self.clients.claim());
});

// ── Helper: Parse SSE text incrementally ──
// Processes accumulated SSE text, extracts event IDs + payloads for PatchElements events,
// and registers them. Returns the remaining (incomplete) text.
let pendingEventType = null;
let pendingEventId = null;
let pendingDataLines = [];

function processSSERead(buffer, text) {
  const data = buffer + text;
  const lines = data.split('\n');

  for (let i = 0; i < lines.length - 1; i++) {
    const line = lines[i];
    if (line.startsWith('event: ')) {
      pendingEventType = line.slice(7).trim();
    } else if (line.startsWith('id: ')) {
      pendingEventId = line.slice(4).trim();
    } else if (line.startsWith('data: ')) {
      pendingDataLines.push(line.slice(6));
    } else if (line === '' && pendingEventType) {
      // End of an event
      if (pendingEventType === EVENT_PREFIX && pendingEventId && pendingEventId.length > 0) {
        const payload = pendingDataLines.join('\n');
        registerHash(pendingEventId, payload, pendingEventType);
      }
      pendingEventType = null;
      pendingEventId = null;
      pendingDataLines = [];
    }
  }

  // Keep the potentially incomplete last line
  const lastLine = lines[lines.length - 1];
  if (lastLine.startsWith('data: ')) {
    pendingDataLines.push(lastLine.slice(6));
    return '';
  }
  return lastLine;
}

// ── Helper: Register a hash with payload ──
// Also accepts legacy calls with just a hash (test compatibility)
function registerHash(hash, payload, eventType) {
  if (payload === undefined) payload = '';
  if (eventType === undefined) eventType = EVENT_PREFIX;

  // Never cache navigation-dependent content-body events.
  // The server sends the correct initial content on page load;
  // replaying a stale content-body (e.g. "series" from a previous
  // navigation) would overwrite the correct initial state.
  if (payload.includes('selector #content-body')) return;

  HASH_REGISTRY.set(hash, { payload, eventType, timestamp: Date.now() });

  // Limit registry size (keep latest MAX_REGISTRY_SIZE)
  if (HASH_REGISTRY.size > MAX_REGISTRY_SIZE) {
    const entries = [...HASH_REGISTRY.entries()]
      .sort((a, b) => a[1].timestamp - b[1].timestamp);
    const toDelete = entries.slice(0, entries.length - MAX_REGISTRY_SIZE);
    for (const [h] of toDelete) {
      HASH_REGISTRY.delete(h);
    }
  }
}

// ── Helper: Clean expired hashes ──
function cleanExpiredHashes() {
  const now = Date.now();
  for (const [hash, entry] of HASH_REGISTRY) {
    const ts = typeof entry === 'object' ? entry.timestamp : entry;
    if (now - ts > TTL_MS) {
      HASH_REGISTRY.delete(hash);
    }
  }
}

// ── Helper: Build SSE text for cached events ──
function buildCachedEventsStream() {
  const parts = [];
  for (const [hash, entry] of HASH_REGISTRY) {
    // Skip legacy entries without proper structure
    if (typeof entry !== 'object' || !entry.payload) continue;
    parts.push(`event: ${entry.eventType}`);
    parts.push(`id: ${hash}`);
    // Split payload by newlines into multiple data: lines
    const dataLines = entry.payload.split('\n');
    for (const line of dataLines) {
      parts.push(`data: ${line}`);
    }
    parts.push(''); // empty line = event boundary
  }
  return parts.join('\n') + '\n';
}

// ── Helper: Read SSE stream incrementally ──
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

    // Flush remaining buffered event
    if (pendingEventType === EVENT_PREFIX && pendingEventId && pendingEventId.length > 0) {
      const payload = pendingDataLines.join('\n');
      registerHash(pendingEventId, payload, pendingEventType);
      pendingEventType = null;
      pendingEventId = null;
      pendingDataLines = [];
    }
  } catch (err) {
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
        // Build the cached events prefix
        const cachedStream = buildCachedEventsStream();

        if (response.body) {
          const [pageStream, swStream] = response.body.tee();

          // Parse the SW stream in the background (don't await)
          consumeSSEStream(swStream);

          // Prepend cached events to the page stream
          const encoder = new TextEncoder();
          const cachedBytes = cachedStream.length > 0 ? encoder.encode(cachedStream) : new Uint8Array(0);

          // Combine: cached events + live server events
          const combinedStream = new ReadableStream({
            start(controller) {
              // Push cached events first
              if (cachedBytes.length > 0) {
                controller.enqueue(cachedBytes);
              }
              // Then pipe the live stream
              const reader = pageStream.getReader();
              function pump() {
                reader.read().then(({ done, value }) => {
                  if (done) {
                    try { controller.close(); } catch (_) {}
                    return;
                  }
                  controller.enqueue(value);
                  pump();
                }).catch(() => { try { controller.close(); } catch (_) {} });
              }
              pump();
            }
          });

          return new Response(combinedStream, {
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
    buildCachedEventsStream,
    TTL_MS,
    MAX_REGISTRY_SIZE,
  };
}
