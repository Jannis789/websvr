// Service Worker — SSE Cache Proxy
// Kein autonomes Replay aus dem SW-Cache — der Server entscheidet was replayt wird.
// id_only Events werden nur als nackter Event-Header durchgereicht.
// Ein globaler sseGen killt alte Streams sobald ein neuer startet.

function swLog() {
  console.log('[sw]', ...arguments);
}

function now() {
  return performance.now().toFixed(1);
}

swLog('loaded at ' + now());

// ── Globaler SSE-Gen — jeder neue Stream inkrementiert ihn.
// Alte Streams checken ob ihr gen noch aktuell ist → stoppen.
var globalSseGen = 0;
var activeAbortController = null;

// ── Per-Client-Tracking (nur fuer beforeunload/closing) ──

var closingClients = new Set();

self.addEventListener('message', function (event) {
  var type = event.data && event.data.type;
  var sourceId = (event.source && event.source.id) || 'none';
  swLog('MSG type=' + type + ' source=' + sourceId.slice(0, 8) + ' at ' + now());
  if (type === 'sse-close') {
    if (event.source && event.source.id) {
      var cid = event.source.id;
      swLog('CLOSE ADD client=' + cid.slice(0, 8) + ' at ' + now());
      closingClients.add(cid);
      setTimeout(function () {
        if (closingClients.has(cid)) {
          swLog('CLOSE TIMEOUT remove client=' + cid.slice(0, 8) + ' at ' + now());
          closingClients.delete(cid);
        }
      }, 10000);
    }
  }
});

self.addEventListener('install', () => {
  swLog('install at ' + now());
  self.skipWaiting();
});
self.addEventListener('activate', () => { swLog('activate at ' + now()); self.clients.claim(); });

// ── Cache Storage (Persistenz fuer Reload) ──

var cacheName = 'sse-v4';
var storedBytes = 0;
var maxBytes = null;

async function ensureMaxBytes() {
  if (maxBytes !== null) return;
  var estimate = await navigator.storage.estimate();
  maxBytes = Math.max(1024 * 1024, Math.floor((estimate.quota - estimate.usage) * 0.5));
}

async function putCache(id, raw) {
  try {
    var cache = await caches.open(cacheName);
    await ensureMaxBytes();
    await cache.put('/' + id, new Response(raw));
    storedBytes += raw.length;
    if (storedBytes <= maxBytes) return;
    var keys = await cache.keys();
    for (var i = 0; i < keys.length && storedBytes > maxBytes; i++) {
      var name = keys[i].url.split('/').pop();
      if (name === 'ver') continue;
      var response = await cache.match(keys[i]);
      if (response) storedBytes -= (await response.text()).length;
      await cache.delete(keys[i]);
    }
  } catch (error) {
    swLog('putCache error:', error && error.message);
  }
}

async function getCache(id) {
  try {
    var response = await (await caches.open(cacheName)).match('/' + id);
    return response ? await response.text() : null;
  } catch (error) { swLog('getCache error for', id, ':', error && error.message); return null; }
}

async function saveVer(value) {
  swLog('saveVer value=' + value + ' at ' + now());
  try { await (await caches.open(cacheName)).put('/ver', new Response(String(value))); } catch (error) { swLog('saveVer error:', error && error.message); }
}

async function loadVer() {
  try {
    var response = await (await caches.open(cacheName)).match('/ver');
    var ver = response ? (parseInt(await response.text(), 10) || 0) : 0;
    swLog('loadVer=' + ver + ' at ' + now());
    return ver;
  } catch (error) { swLog('loadVer error:', error && error.message); return 0; }
}

// ── SSE-Parser ──

function parseBlock(block) {
  var idMatch = block.match(/id:\s*(\d+)/);
  if (!idMatch) return null;
  var id = parseInt(idMatch[1], 10);
  if (isNaN(id)) return null;
  return { id: id, hasBody: block.indexOf('\n') !== -1 };
}

// ── Fetch Intercept ──

self.addEventListener('fetch', function (event) {
  var url = new URL(event.request.url);
  if (url.pathname !== '/sse') return;
  if (event.request.signal.aborted) {
    swLog('conn ABORTED client=' + (event.clientId ? event.clientId.slice(0, 8) : '?') + ' at ' + now());
    return;
  }

  var cid = (event.clientId || '?').slice(0, 8);
  swLog('conn start client=' + cid + ' at ' + now());

  // closing client: 204 zurueckgeben, keinen Server-Fetch machen
  if (closingClients.has(event.clientId)) {
    swLog('CLOSING CLIENT — 204 at ' + now());
    closingClients.delete(event.clientId);
    event.respondWith(new Response(null, { status: 204 }));
    return;
  }

  // Globaler SSE-Gen: jeder neue Stream inkrementiert + alter wird gekillt
  globalSseGen++;
  var mySseGen = globalSseGen;
  if (activeAbortController) {
    try { activeAbortController.abort(); } catch (_) {}
  }
  activeAbortController = null;
  swLog('  sseGen=' + mySseGen + ' closingClients size=' + closingClients.size + ' at ' + now());

  event.respondWith(
    (async function () {
      var ver = await loadVer();
      swLog('  POST body seen=' + ver + ' at ' + now());

      var abortController = new AbortController();
      activeAbortController = abortController;
      var headers = new Headers({ 'Content-Type': 'application/json' });
      var fetchStart = performance.now();
      var response = await fetch(url.origin + url.pathname, {
        method: 'POST',
        headers: headers,
        body: JSON.stringify({ seen: ver }),
        credentials: 'include',
        signal: abortController.signal
      });
      var fetchMs = (performance.now() - fetchStart).toFixed(1);
      swLog('  server response status=' + response.status + ' took=' + fetchMs + 'ms at ' + now());

      if (!response.ok) {
        swLog('  server NOT OK — returning ' + response.status + ' at ' + now());
        return new Response(null, { status: response.status });
      }
      if (!response.body) {
        swLog('  server NO BODY at ' + now());
        return response;
      }

      var reader = response.body.getReader();
      var decoder = new TextDecoder();
      var encoder = new TextEncoder();
      var buffer = '';
      var memCache = new Map();
      var lastVer = ver;
      var totalBytes = 0;
      var chunks = 0;
      var eventsProcessed = 0;
      var streamStart = performance.now();
      var closed = false;

      // Prueft ob ein NEUERER Stream gestartet ist (global)
      function isStale() {
        return mySseGen !== globalSseGen;
      }

      function closeIfStale(controller) {
        if (isStale()) {
          if (!closed) { closed = true; try { controller.close(); } catch (_) {} }
          try { reader.cancel(); } catch (_) {}
          try { reader.releaseLock(); } catch (_) {}
          return true;
        }
        return false;
      }

      swLog('  STREAM START sseGen=' + mySseGen + ' at ' + now());

      var stream = new ReadableStream({
        pull: function (controller) {
          return (async function () {
            if (closeIfStale(controller)) return;

            var readStart = performance.now();
            var result;
            try {
              result = await reader.read();
            } catch (err) {
              swLog('  READ ERROR:', err && err.message, 'at', now());
              if (!closed) { closed = true; try { controller.close(); } catch (_) {} }
              return;
            }
            var readMs = (performance.now() - readStart).toFixed(1);

            if (isStale()) {
              swLog('  SSE GEN CHANGED — stopping (was ' + mySseGen + ' now ' + globalSseGen + ') at ' + now());
              if (!closed) { closed = true; try { controller.close(); } catch (_) {} }
              try { reader.cancel(); } catch (_) {}
              try { reader.releaseLock(); } catch (_) {}
              return;
            }

            if (result.done) {
              swLog('  READ DONE at ' + now() + ' (chunks=' + chunks + ' totalBytes=' + totalBytes + ' events=' + eventsProcessed + ' ms=' + (performance.now() - streamStart).toFixed(1) + ' lastVer=' + lastVer + ')');
              try { await saveVer(lastVer); } catch (_) {}
              if (!closed) { closed = true; controller.close(); }
              return;
            }

            chunks++;
            totalBytes += result.value.length;
            swLog('  READ chunk#' + chunks + ' size=' + result.value.length + 'B total=' + totalBytes + 'B took=' + readMs + 'ms at ' + now());

            buffer += decoder.decode(result.value, { stream: true });

            while (true) {
              var i = buffer.indexOf('\n\n');
              if (i === -1) break;
              var block = buffer.slice(0, i);
              buffer = buffer.slice(i + 2);
              if (!block.trim()) continue;

              var info = parseBlock(block);
              if (!info) {
                controller.enqueue(encoder.encode(block + '\n\n'));
                continue;
              }
              if (info.id > lastVer) lastVer = info.id;

              eventsProcessed++;

              if (!info.hasBody) {
                // id_only: NUR den Event-Header weiterleiten, KEIN Cache-Replay.
                // Der Server entscheidet was replayt wird — nicht der SW.
                controller.enqueue(encoder.encode(block + '\n\n'));
                continue;
              }

              var existing = memCache.get(info.id);
              if (existing) {
                swLog('  EVENT#' + info.id + ' dedup HIT (memory) at ' + now());
                controller.enqueue(encoder.encode(existing));
              } else {
                // "from server" — in memory + storage cachen
                swLog('  EVENT#' + info.id + ' from server at ' + now());
                var fullBlock = block + '\n\n';
                memCache.set(info.id, fullBlock);
                putCache(info.id, fullBlock);
                controller.enqueue(encoder.encode(fullBlock));
              }
            }
          })().catch(function (err) {
            swLog('  PULL ERROR:', err && err.message, 'at', now());
            if (!closed) { closed = true; try { controller.close(); } catch (_) {} }
          });
        },
        cancel: function (reason) {
          closed = true;
          swLog('  STREAM CANCEL sseGen=' + mySseGen + ' lastVer=' + lastVer + ' at ' + now());
          if (mySseGen === globalSseGen) {
            saveVer(lastVer);
          }
        }
      });

      return new Response(stream, {
        headers: { 'Content-Type': 'text/event-stream' }
      });
    })()
  );
});
