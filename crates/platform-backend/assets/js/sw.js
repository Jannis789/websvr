var CACHE_NAME = 'sse-cache-v1';
var memoryCache = new Map();
var maxId = 0;
var cacheReady = false;
var resolveCacheReady;
var cacheReadyPromise = new Promise(function(r) { resolveCacheReady = r; });

function swLog() {
  console.log('[sw]', ...arguments);
}
function now() {
  return performance.now().toFixed(1);
}

swLog('loaded at ' + now());

async function ensureCacheReady() {
  if (cacheReady) return;
  try {
    var cache = await caches.open(CACHE_NAME);
    var keys = await cache.keys();
    swLog('  INIT: found ' + keys.length + ' items in cache');
    for (var req of keys) {
      if (req.url.includes('/sse/event/')) {
        var idStr = req.url.split('/').pop();
        var id = parseInt(idStr, 10);
        if (!isNaN(id)) {
          if (id > maxId) maxId = id;
          var resp = await cache.match(req);
          if (resp) {
            var text = await resp.text();
            memoryCache.set(id, text);
            swLog('  INIT: restored id=' + id);
          }
        }
      }
    }
    swLog('  INIT: complete, maxId=' + maxId + ', memoryCache.size=' + memoryCache.size);
  } catch(e) {
    swLog('  INIT CACHE ERROR:', e && e.message);
  }
  cacheReady = true;
  resolveCacheReady();
}

// SOFORT beim Aufwachen des SW ausführen (verhindert infinite WAITING bei Wiederverwendung)
ensureCacheReady();

self.addEventListener('install', function(e) {
  swLog('install at ' + now());
  e.waitUntil(ensureCacheReady().then(function() { return self.skipWaiting(); }));
});

self.addEventListener('activate', function(e) {
  swLog('activate at ' + now());
  e.waitUntil(self.clients.claim());
});

self.addEventListener('fetch', function(event) {
  var url = new URL(event.request.url);
  if (url.pathname !== '/sse') return;
  if (event.request.signal.aborted) return;

  var cid = (event.clientId || '?').slice(0, 8);
  swLog('conn start client=' + cid + ' at ' + now());

  event.respondWith((async function() {
    if (!cacheReady) {
      swLog('  WAITING for cache ready...');
      await cacheReadyPromise;
    }
    
    var fetchUrl = url.origin + '/sse?v=' + maxId;
    swLog('  FETCHING: ' + fetchUrl + ' (maxId=' + maxId + ')');

    var response = await fetch(fetchUrl, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({}),
      credentials: 'include'
    });

    if (!response.ok || !response.body) {
      swLog('  FETCH FAILED: status=' + response.status);
      return response.ok ? response : new Response(null, { status: response.status });
    }

    var reader = response.body.getReader();
    var decoder = new TextDecoder();
    var encoder = new TextEncoder();
    var buffer = '';
    var closed = false;

    swLog('  STREAM START at ' + now());

    return new Response(new ReadableStream({
      pull: async function(controller) {
        // AGGRESSIVES CHECKING: Sofort abbrechen, falls cancel bereits gefeuert hat
        if (closed) return;
        
        var result;
        try {
          result = await reader.read();
        } catch (err) {
          swLog('  READ ERROR:', err && err.message);
          closed = true;
          controller.close();
          return;
        }

        if (result.done) {
          swLog('  STREAM DONE at ' + now());
          closed = true;
          controller.close();
          return;
        }
        
        buffer += decoder.decode(result.value, { stream: true });
        
        while (true) {
          // Bei jeder Iteration prüfen, ob wir abgebrochen wurden
          if (closed) return;
          
          var i = buffer.indexOf('\n\n');
          if (i === -1) break;
          
          var block = buffer.slice(0, i);
          buffer = buffer.slice(i + 2);
          if (!block.trim()) continue;

          var idMatch = block.match(/id:\s*(\d+)/);
          if (!idMatch) {
            controller.enqueue(encoder.encode(block + '\n\n'));
            continue;
          }
          
          var id = parseInt(idMatch[1], 10);
          var isFull = block.indexOf('event:') !== -1 || block.indexOf('data:') !== -1;

          try {
            if (isFull) {
              swLog('  EVENT #' + id + ': FULL -> caching and enqueueing');
              if (id > maxId) maxId = id;
              
              var fullBlock = block + '\n\n';
              memoryCache.set(id, fullBlock);
              
              var cache = await caches.open(CACHE_NAME);
              cache.put('/sse/event/' + id, new Response(fullBlock));
              
              controller.enqueue(encoder.encode(fullBlock));
            } else {
              swLog('  EVENT #' + id + ': ID-ONLY -> attempting replay');
              var cached = memoryCache.get(id);
              
              if (cached) {
                swLog('  EVENT #' + id + ': REPLAY from memoryCache SUCCESS');
                controller.enqueue(encoder.encode(cached));
              } else {
                swLog('  EVENT #' + id + ': MISS in memory, trying CacheStorage...');
                var cache = await caches.open(CACHE_NAME);
                var resp = await cache.match('/sse/event/' + id);
                
                if (resp) {
                  var text = await resp.text();
                  swLog('  EVENT #' + id + ': REPLAY from CacheStorage SUCCESS');
                  memoryCache.set(id, text);
                  controller.enqueue(encoder.encode(text));
                } else {
                  swLog('  EVENT #' + id + ': CRITICAL CACHE MISS! Server sent id-only but SW has no record.');
                  controller.enqueue(encoder.encode(block + '\n\n'));
                }
              }
            }
          } catch (e) {
            swLog('  EVENT PROCESS ERROR for id=' + id + ':', e && e.message);
          }
        }
      },
      
      cancel: function() {
        swLog('  STREAM CANCELLED at ' + now());
        closed = true;
        reader.cancel().catch(function(){});
      }
    }), {
      status: response.status,
      statusText: response.statusText,
      headers: response.headers
    });
  })().catch(function(err) {
    swLog('  FETCH HANDLER ERROR:', err && err.message);
    return new Response('', { status: 204 });
  }));
});

self.addEventListener('fetch', function(event) {
  var url = new URL(event.request.url);
  if (url.pathname.startsWith('/sse-cache/') || url.pathname.startsWith('/sse/event/')) {
    event.respondWith(caches.match(event.request));
  }
});
