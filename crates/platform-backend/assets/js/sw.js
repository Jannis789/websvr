// Service Worker — Route-based Hash Dedup & Event Cache
//
// Per browser route (e.g. /home, /login), tracks which SSE event hashes
// have been seen. On SSE reconnect, sends only the hashes for the current
// route. Server skips known events → SW replays them from local cache.
//
// Storage:
//   routeHashes: Map<route, Set<hash>>
//   eventCache:  Map<hash, rawSseText>

self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', (e) => e.waitUntil(self.clients.claim()));

// ── State ──

/** @type {Map<string, Set<string>>} hashes grouped by browser route */
const routeHashes = new Map();

/** @type {Map<string, string>} hash → raw SSE event text */
const eventCache = new Map();

/** Max cached events to prevent unbounded memory */
const MAX_CACHE_SIZE = 200;

// ── Route Detection ──

async function getRouteFromClient() {
  const clients = await self.clients.matchAll({ type: 'window' });
  if (clients.length === 0) return '/';
  const url = new URL(clients[0].url);
  return url.pathname;
}

// ── SSE Parser ──

function parseSseEvents(raw) {
  const events = [];
  let current = { id: '', event: '', data: '' };
  let hasContent = false;

  for (const line of raw.split('\n')) {
    if (line.startsWith('id:')) {
      current.id = line.slice(3).trim();
      hasContent = true;
    } else if (line.startsWith('event:')) {
      current.event = line.slice(6).trim();
      hasContent = true;
    } else if (line.startsWith('data:')) {
      current.data += (current.data ? '\n' : '') + line;
      hasContent = true;
    } else if (line === '' && hasContent) {
      events.push(current);
      current = { id: '', event: '', data: '' };
      hasContent = false;
    }
  }
  if (hasContent) events.push(current);
  return events;
}

function buildRawEvent(evt) {
  let raw = '';
  if (evt.id) raw += `id: ${evt.id}\n`;
  if (evt.event) raw += `event: ${evt.event}\n`;
  if (evt.data) raw += `${evt.data}\n`;
  raw += '\n';
  return raw;
}

// ── Fetch Handler ──

self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);
  if (url.pathname !== '/sse') return;

  event.respondWith(handleSse(event));
});

async function handleSse(event) {
  const route = await getRouteFromClient();

  // Get hashes for current route
  const known = routeHashes.get(route);
  const hashList = known ? Array.from(known) : [];

  // Append known_hashes to URL
  const sseUrl = new URL(event.request.url);
  if (hashList.length > 0) {
    sseUrl.searchParams.set('known_hashes', hashList.join(','));
  }

  const response = await fetch(sseUrl.toString(), {
    method: event.request.method,
    headers: event.request.headers,
    signal: event.request.signal,
  });

  // No events to replay — pass through directly
  if (hashList.length === 0) {
    return new Response(response.body, {
      status: response.status,
      statusText: response.statusText,
      headers: response.headers,
    });
  }

  // Build replay prefix from cache for known hashes
  const replayParts = [];
  for (const hash of hashList) {
    const cached = eventCache.get(hash);
    if (cached) replayParts.push(cached);
  }
  const replayPrefix = replayParts.join('');

  // Intercept the stream: replay prefix + passthrough new events
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  const encoder = new TextEncoder();

  const stream = new ReadableStream({
    async start(controller) {
      // Phase 1: Replay cached events
      if (replayPrefix) {
        controller.enqueue(encoder.encode(replayPrefix));
      }

      // Phase 2: Forward live events, caching each one
      try {
        let buffer = '';
        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          const text = decoder.decode(value, { stream: true });
          buffer += text;

          // Parse complete events from buffer
          const events = parseSseEvents(buffer);
          // Keep incomplete trailing data
          const lastBoundary = buffer.lastIndexOf('\n\n');
          if (lastBoundary !== -1) {
            buffer = buffer.slice(lastBoundary + 2);
          }

          for (const evt of events) {
            if (evt.id) {
              // Cache the event
              const raw = buildRawEvent(evt);
              eventCache.set(evt.id, raw);
              routeHashes.get(route)?.add(evt.id);

              // Evict oldest if over limit
              if (eventCache.size > MAX_CACHE_SIZE) {
                const oldest = eventCache.keys().next().value;
                eventCache.delete(oldest);
              }
            }
          }

          controller.enqueue(value);
        }

        // Flush remaining buffer
        if (buffer.trim()) {
          controller.enqueue(encoder.encode(buffer));
        }
      } catch (e) {
        // Stream aborted — normal on SSE disconnect
      } finally {
        controller.close();
      }
    },
  });

  return new Response(stream, {
    status: response.status,
    statusText: response.statusText,
    headers: response.headers,
  });
}

// ── Test exports ──
if (typeof globalThis.__SW_TEST_MODE !== 'undefined') {
  self.__sw = {
    routeHashes,
    eventCache,
    parseSseEvents,
    buildRawEvent,
    _getRouteFromClient: getRouteFromClient,
    _setRoute: (route) => { /* test helper */ },
  };
}
