// Service Worker — SSE patch_ver tracking + event cache with epoch validation
//
// Intercepts /sse responses, caches full events by server-assigned patch_ver.
// On reconnect, sends ?v=<highest>&e=<epoch>. Server replies with:
//   - Full events WITH id: for content the SW hasn't cached
//   - id-only events (just "id: N\n\n") for content the SW already has
//
// NUR /sse wird abgefangen — alle anderen Requests gehen normal zum Server.
//
// Epoch validation: Server sends X-SSE-Epoch header. If it changed
// (server restart), the SW clears its cache and resets patch_ver.
//
// KEINE ReadableStream-Manipulation — die Server-Response wird 1:1
// an die Page durchgereicht. Nur ein response.clone() läuft parallel
// zum Lernen der Event-IDs.

function swLog(...args) {
  console.log('[sw]', ...args);
  self.clients.matchAll().then(clients => {
    for (const c of clients) c.postMessage({ type: 'sw-log', args });
  });
}

swLog('loaded');

self.addEventListener('install', () => {
  swLog('install');
  self.skipWaiting();
});

self.addEventListener('activate', () => {
  swLog('activate');
  self.clients.claim();
});

let lastPatchVer = 0;
let serverEpoch = null;
const eventCache = new Map();
const MAX_CACHE_SIZE = 200;

function evictIfNeeded() {
  while (eventCache.size > MAX_CACHE_SIZE) {
    const oldest = eventCache.keys().next().value;
    eventCache.delete(oldest);
  }
}

function clearCache() {
  eventCache.clear();
  lastPatchVer = 0;
}

function processSseChunk(text) {
  const output = [];
  const rawEvents = text.split('\n\n');

  for (const raw of rawEvents) {
    if (!raw.trim()) continue;

    let id = null;
    let hasData = false;

    for (const line of raw.split('\n')) {
      if (line.startsWith('id:')) {
        id = parseInt(line.slice(3).trim(), 10);
      }
      if (line.startsWith('data:') || line.startsWith('event:')) {
        hasData = true;
      }
    }

    if (id !== null && !isNaN(id)) {
      if (id > lastPatchVer) lastPatchVer = id;

      const isExecuteScript = raw.includes('event: datastar-execute-script');

      if (hasData && !isExecuteScript) {
        const cached = eventCache.get(id);
        if (cached !== undefined) {
          output.push(cached);
        } else {
          eventCache.set(id, raw + '\n\n');
          evictIfNeeded();
          output.push(raw + '\n\n');
        }
      } else if (hasData && isExecuteScript) {
        output.push(raw + '\n\n');
      } else {
        const cached = eventCache.get(id);
        if (cached) {
          output.push(cached);
        }
      }
    } else if (hasData) {
      output.push(raw + '\n\n');
    }
  }

  return output.join('');
}

// ── Background learner ──
// Konsumiert einen Body-Stream parallel zur Page, lernt Event-IDs
// und aktualisiert lastPatchVer. KEIN ReadableStream-Manipulation.
async function learnFromStream(body) {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let leftover = '';

  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) {
        if (leftover.trim()) {
          processSseChunk(leftover);
        }
        break;
      }
      leftover += decoder.decode(value, { stream: true });
      const lastBoundary = leftover.lastIndexOf('\n\n');
      if (lastBoundary === -1) continue;
      const complete = leftover.slice(0, lastBoundary);
      leftover = leftover.slice(lastBoundary + 2);
      processSseChunk(complete);
    }
  } catch (e) {
    // Stream cancelled by page navigation — expected, ignore
  } finally {
    reader.releaseLock();
  }
}

self.addEventListener('fetch', (event) => {
  try {
    const url = new URL(event.request.url);

    // Clear SSE cache on page navigation (document request)
    if (event.request.destination === 'document') {
      clearCache();
    }

    if (url.pathname !== '/sse') return;

    swLog('intercept /sse');

    // Nur URL modifizieren — Server-Response 1:1 durchreichen
    const cleanUrl = new URL(url.origin + url.pathname);
    if (lastPatchVer > 0) cleanUrl.searchParams.set('v', lastPatchVer);
    if (serverEpoch !== null) cleanUrl.searchParams.set('e', serverEpoch);

    event.respondWith(
      fetch(cleanUrl.toString(), { headers: event.request.headers })
        .then((response) => {
          // Epoch prüfen
          const newEpoch = response.headers.get('x-sse-epoch');
          if (newEpoch !== null) {
            if (serverEpoch !== null && serverEpoch !== newEpoch) {
              clearCache();
            }
            serverEpoch = newEpoch;
          }

          // Parallel lernen aus einem clone — Response geht UNVERÄNDERT an die Page
          if (response.body) {
            learnFromStream(response.clone().body);
          }

          return response; // 1:1, kein ReadableStream, kein Passthrough
        })
        .catch((err) => {
          swLog('fetch error:', err.message);
          return new Response('', { status: 503, statusText: 'Service Unavailable' });
        })
    );
  } catch (e) {
    // Ignore requests that can't be parsed
  }
});
