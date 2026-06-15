// Service Worker — SSE Cache Proxy
// Server sendet volle Events ODER nur `id: N` (Content-Dedup).
// Volle Events → FIFO-Cache + durchreichen.
// id: N → Replay aus FIFO.
// Connect mit ?s=N (FIFO-Größe). Server pusht id-only bei Match, volle bei Mismatch.

function swLog() {
  console.log('[sw]', ...arguments);
}
function now() {
  return performance.now().toFixed(1);
}
swLog('loaded at ' + now());

// ── Closing-Clients ──
var closingClients = new Set();

self.addEventListener('message', function (event) {
  var type = event.data && event.data.type;
  if (type === 'sse-close' && event.source && event.source.id) {
    var cid = event.source.id;
    closingClients.add(cid);
    setTimeout(function () { closingClients.delete(cid); }, 10000);
  }
});

self.addEventListener('install', function () { swLog('install at ' + now()); self.skipWaiting(); });
self.addEventListener('activate', function () { swLog('activate at ' + now()); self.clients.claim(); });

// ── FIFO-Cache ──
var FIFO_MAX = 1000;
var fifo = new Map();

// ── SSE-Parser ──

function parseBlock(block) {
  var idMatch = block.match(/id:\s*(\d+)/);
  if (!idMatch) return null;
  var id = parseInt(idMatch[1], 10);
  if (isNaN(id)) return null;
  // id-only wenn KEINE event:/data: Zeile
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

      swLog('  STREAM START at ' + now());

      var stream = new ReadableStream({
        pull: function (controller) {
          return (async function () {
            // Wenn sse-close schon kam, Stream sofort beenden
            if (closingClients.has(event.clientId)) {
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
                // Volles Event: cachen + durchreichen
                swLog('  EVENT#' + parsed.id + ' FULL at ' + now());
                var fullBlock = block + '\n\n';
                if (fifo.size >= FIFO_MAX) {
                  var firstKey = fifo.keys().next().value;
                  fifo.delete(firstKey);
                }
                fifo.set(parsed.id, fullBlock);
                controller.enqueue(encoder.encode(fullBlock));
              } else {
                // id-only: Replay aus FIFO
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
