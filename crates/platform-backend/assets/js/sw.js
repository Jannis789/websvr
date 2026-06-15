// Service Worker — SSE Cache Proxy
// FIFO-Cache persistiert via Cache Storage API.
// SW-Update → FIFO aus Cache restauriert → kein FULL-Resync nötig.
// Server sendet id-only bei Match, volle Events bei Mismatch.

var CACHE_NAME = 'sse-fifo-v1';
var FIFO_MAX = 1000;

function swLog() {
  console.log('[sw]', ...arguments);
}
function now() {
  return performance.now().toFixed(1);
}
swLog('loaded at ' + now());

// ── FIFO-Cache (Map + Cache Storage) ──
var fifo = new Map();
var fifoReady = false;   // true nachdem fifoRestore() fertig ist
var fifoReadyResolve;
var fifoReadyPromise = new Promise(function(r) { fifoReadyResolve = r; });

async function fifoPersist(id, block) {
  try {
    var cache = await caches.open(CACHE_NAME);
    var req = '/sse-cache/' + id;
    cache.put(req, new Response(block));
    var keys = await cache.keys();
    if (keys.length > FIFO_MAX) {
      var oldest = keys.slice(0, keys.length - FIFO_MAX);
      for (var r of oldest) cache.delete(r);
    }
  } catch(e) {
    swLog('  CACHE ERROR:', e && e.message);
  }
}

async function fifoRestore() {
  try {
    var cache = await caches.open(CACHE_NAME);
    var keys = await cache.keys();
    keys.sort(function(a, b) {
      var idA = parseInt(a.url.split('/').pop(), 10);
      var idB = parseInt(b.url.split('/').pop(), 10);
      return idA - idB;
    });
    for (var req of keys) {
      var resp = await cache.match(req);
      if (resp) {
        var block = await resp.text();
        var id = parseInt(req.url.split('/').pop(), 10);
        if (!isNaN(id) && block) fifo.set(id, block);
      }
    }
    swLog('  CACHE RESTORED: ' + fifo.size + ' events at ' + now());
  } catch(e) {
    swLog('  CACHE RESTORE ERROR:', e && e.message);
  }
  fifoReady = true;
  fifoReadyResolve();
}

// ── Closing-Clients ──
var closingClients = new Set();

self.addEventListener('message', function (event) {
  var type = event.data && event.data.type;
  if ((type === 'sse-close' || type === 'beforeunload') && event.source && event.source.id) {
    var cid = event.source.id;
    closingClients.add(cid);
    setTimeout(function () { closingClients.delete(cid); }, 10000);
  }
});

self.addEventListener('install', function (evt) {
  swLog('install at ' + now());
  // Cache aus vorheriger SW-Version laden (bevor skipWaiting aktiv wird)
  evt.waitUntil(fifoRestore());
  self.skipWaiting();
});
self.addEventListener('activate', function (evt) {
  swLog('activate at ' + now());
  evt.waitUntil(self.clients.claim());
});

// ── SSE-Parser ──

function parseBlock(block) {
  var idMatch = block.match(/id:\s*(\d+)/);
  if (!idMatch) return null;
  var id = parseInt(idMatch[1], 10);
  if (isNaN(id)) return null;
  var hasData = block.indexOf('event:') !== -1 || block.indexOf('data:') !== -1;
  return { id: id, full: hasData };
}

// ── Fetch Intercept ──

self.addEventListener('fetch', function (event) {
  var url = new URL(event.request.url);
  if (url.pathname !== '/sse') return;
  if (event.request.signal.aborted) return;

  var cid = (event.clientId || '?').slice(0, 8);
  swLog('conn start client=' + cid + ' at ' + now());
  if (closingClients.has(event.clientId)) {
    swLog('CLOSING CLIENT — 204 at ' + now());
    closingClients.delete(event.clientId);
    event.respondWith(new Response(null, { status: 204 }));
    return;
  }

  event.respondWith(
    (async function () {
      // Warten bis CACHE RESTORED abgeschlossen ist
      if (!fifoReady) await fifoReadyPromise;
      var sParam = fifo.size > 0 ? '?s=' + fifo.size : '';
      var response = await fetch(url.origin + url.pathname + sParam, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({}),
        credentials: 'include'
      });
      swLog('  server response status=' + response.status + ' at ' + now());

      if (!response.ok || !response.body) {
        return response.ok ? response : new Response(null, { status: response.status });
      }

      var reader = response.body.getReader();
      var decoder = new TextDecoder();
      var encoder = new TextEncoder();
      var buffer = '';
      var closed = false;
      var streamCancelled = false;

      swLog('  STREAM START at ' + now());

      var stream = new ReadableStream({
        pull: function (controller) {
          return (async function () {
            if (closingClients.has(event.clientId) || streamCancelled) {
              if (!closed) { closed = true; try { controller.close(); } catch (_) {} }
              try { reader.cancel(); } catch (_) {}
              return;
            }

            var result;
            try {
              result = await reader.read();
            } catch (err) {
              swLog('  READ ERROR:', err && err.message, 'at', now());
              if (!closed) { closed = true; try { controller.close(); } catch (_) {} }
              return;
            }

            if (result.done) {
              if (!closed) { closed = true; controller.close(); }
              return;
            }

            buffer += decoder.decode(result.value, { stream: true });

            while (true) {
              var i = buffer.indexOf('\n\n');
              if (i === -1) break;
              var block = buffer.slice(0, i);
              buffer = buffer.slice(i + 2);
              if (!block.trim()) continue;

              var parsed = parseBlock(block);
              if (!parsed) {
                controller.enqueue(encoder.encode(block + '\n\n'));
                continue;
              }

              if (parsed.full) {
                if (closingClients.has(event.clientId) || streamCancelled) {
                  if (!closed) { closed = true; try { controller.close(); } catch (_) {} }
                  try { reader.cancel(); } catch (_) {}
                  return;
                }
                swLog('  EVENT#' + parsed.id + ' FULL at ' + now());
                var fullBlock = block + '\n\n';
                if (fifo.size >= FIFO_MAX) {
                  var firstKey = fifo.keys().next().value;
                  fifo.delete(firstKey);
                }
                fifo.set(parsed.id, fullBlock);
                // Asynchron in Cache Storage persistieren
                fifoPersist(parsed.id, fullBlock);
                controller.enqueue(encoder.encode(fullBlock));
              } else {
                if (closingClients.has(event.clientId) || streamCancelled) {
                  if (!closed) { closed = true; try { controller.close(); } catch (_) {} }
                  try { reader.cancel(); } catch (_) {}
                  return;
                }
                var cached = fifo.get(parsed.id);
                swLog('  EVENT#' + parsed.id + (cached ? ' REPLAY' : ' MISS') + ' at ' + now());
                if (cached) {
                  controller.enqueue(encoder.encode(cached));
                }
              }
            }
          })().catch(function (err) {
            swLog('  PULL ERROR:', err && err.message, 'at', now());
            if (!closed) { closed = true; try { controller.close(); } catch (_) {} }
          });
        },

        cancel: function () {
          swLog('  STREAM CANCEL at ' + now());
          closed = true;
          streamCancelled = true;
          reader.cancel().catch(function () {});
        }
      });

      return new Response(stream, {
        status: response.status,
        statusText: response.statusText,
        headers: response.headers
      });
    })().catch(function (err) {
      swLog('  FETCH HANDLER ERROR:', err && err.message, 'at', now());
      return new Response('', { status: 204 });
    })
  );
});

// Cache für SW-internen Gebrauch registrieren (nötig für Cache Storage API)
self.addEventListener('fetch', function (event) {
  var url = new URL(event.request.url);
  if (url.pathname.startsWith('/sse-cache/')) {
    event.respondWith(caches.match(event.request));
  }
});
