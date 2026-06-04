// Service Worker — SSE patch_ver tracking + event cache with epoch validation
//
// Intercepts /sse responses, caches full events by server-assigned patch_ver.
// On reconnect, sends ?v=<highest>&e=<epoch>. Server replies with:
//   - Full events WITH id: for content the SW hasn't cached
//   - id-only events (just "id: N\n\n") for content the SW already has
//
// NUR /sse wird abgefangen — alle anderen Requests gehen normal zum Server.
//
// Der SSE-Response-Body wird in einen ReadableStream gewrappt. Jedes Event
// wird beim Durchreichen gecacht. Bei cancel() wird der Reader NICHT gekillt
// — der Stream schliesst sich von selbst (verhindert ERR_INCOMPLETE_CHUNKED).
//
// Der Cache wird bei JEDEM neuen /sse-Intercept geleert, nicht erst beim
// document-request. Das eliminiert Race-Conditions zwischen alten und neuen
// learnFromStream-Instanzen.

function swLog(...args) {
  console.log('[sw]', ...args);
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
  if (eventCache.size > 0 || lastPatchVer > 0) {
    swLog('clear cache');
  }
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
          swLog('from cache', id);
          output.push(cached);
        } else {
          swLog('from server', id);
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

function createPassthroughStream(body) {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let leftover = '';

  return new ReadableStream({
    pull(controller) {
      return reader.read().then(({ done, value }) => {
        if (done) {
          if (leftover.trim()) {
            const result = processSseChunk(leftover);
            if (result) controller.enqueue(new TextEncoder().encode(result));
          }
          controller.close();
          return;
        }

        leftover += decoder.decode(value, { stream: true });
        const lastBoundary = leftover.lastIndexOf('\n\n');
        if (lastBoundary === -1) return;

        const complete = leftover.slice(0, lastBoundary);
        leftover = leftover.slice(lastBoundary + 2);

        const result = processSseChunk(complete);
        if (result) controller.enqueue(new TextEncoder().encode(result));
      });
    },
    cancel() {
      // Reader NICHT killen — der HTTP-Connection bleibt intakt.
      // Der Browser hat den Stream selbst geschlossen (Navigation etc.).
      // controller.close() von aussen nicht noetig — der GC raeumt auf.
    },
  });
}

self.addEventListener('fetch', (event) => {
  try {
    const url = new URL(event.request.url);

    if (url.pathname !== '/sse') return;

    // Cache IMMER vor neuem SSE-Connect leeren — unabhaengig von Navigation.
    // Das eliminiert Race-Conditions zwischen alten learnFromStream und neuen.
    clearCache();

    swLog('intercept /sse');

    // URL modifizieren (v=, e=)
    const cleanUrl = new URL(url.origin + url.pathname);
    if (lastPatchVer > 0) cleanUrl.searchParams.set('v', lastPatchVer);
    if (serverEpoch !== null) cleanUrl.searchParams.set('e', serverEpoch);

    event.respondWith(
      fetch(cleanUrl.toString(), { headers: event.request.headers })
        .then((response) => {
          // Epoch pruefen
          const newEpoch = response.headers.get('x-sse-epoch');
          if (newEpoch !== null) {
            if (serverEpoch !== null && serverEpoch !== newEpoch) {
              clearCache();
            }
            serverEpoch = newEpoch;
          }

          const body = response.body;
          if (!body) return response;

          // Response wrappen: ReadableStream cached Events beim Durchreichen
          return new Response(createPassthroughStream(body), {
            status: response.status,
            statusText: response.statusText,
            headers: response.headers,
          });
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
