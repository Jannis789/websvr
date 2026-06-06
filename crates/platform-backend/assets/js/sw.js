// Service Worker — SSE Caching via Cache Storage API + ReadableStream
//
// Der SW cached SSE-Events im Cache Storage API (persistent) und in
// einer In-Memory-Map (synchron). Bei id_only-Events (nur `id: N`)
// replayed er aus dem Cache — kein Server-Neutransfer nötig.
//
// ReadableStream-Wrapper: Der SW liest den Response-Body, cached Events
// und leitet sie unverändert an den Browser weiter.
// Kein tee(), kein clone() — ein ReadableStream, ein Konsument.

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

// ── Client-State ──

const clients = new Map();
const MAX_CACHE_SIZE = 5000;
const pendingRestore = new Map();
const CACHE_NAME = 'sse-cache-v1';

// ── Cache Storage API (persistent) ──

async function clearPersistentCache() {
  try {
    const cache = await caches.open(CACHE_NAME);
    const keys = await cache.keys();
    let count = 0;
    for (const req of keys) {
      const path = req.url.split('/').pop();
      if (path === '_ver' || path === '_epoch') continue;
      await cache.delete(req);
      count++;
    }
    if (count > 0) swLog('  persistent cache cleared (', count, 'events)');
  } catch (e) {
    swLog('CACHE CLEAR ERROR:', e.message);
  }
}

async function storeEvent(ver, data) {
  try {
    const cache = await caches.open(CACHE_NAME);
    await cache.put('/' + ver, new Response(data));
  } catch (e) {
    swLog('CACHE PUT ERROR:', e.message);
  }
}

async function storeMeta(ver, epoch) {
  try {
    const cache = await caches.open(CACHE_NAME);
    await cache.put('/_ver', new Response(String(ver)));
    if (epoch !== null) await cache.put('/_epoch', new Response(String(epoch)));
  } catch (e) {
    swLog('META ERROR:', e.message);
  }
}

async function restoreCache(targetMap) {
  try {
    const cache = await caches.open(CACHE_NAME);
    const keys = await cache.keys();
    let count = 0;
    for (const req of keys) {
      const path = req.url.split('/').pop();
      if (path === '_ver' || path === '_epoch') continue;
      const resp = await cache.match(req);
      if (resp) {
        targetMap.set(parseInt(path, 10), await resp.text());
        count++;
      }
    }
    return count;
  } catch (e) {
    swLog('RESTORE ERROR:', e.message);
    return 0;
  }
}

async function restoreMeta() {
  try {
    const cache = await caches.open(CACHE_NAME);
    const [verResp, epochResp] = await Promise.all([
      cache.match('/_ver'),
      cache.match('/_epoch'),
    ]);
    return {
      ver: verResp ? parseInt(await verResp.text(), 10) || 0 : 0,
      epoch: epochResp ? parseInt(await epochResp.text(), 10) || null : null,
    };
  } catch {
    return { ver: 0, epoch: null };
  }
}

function evictIfNeeded(cache) {
  while (cache.size > MAX_CACHE_SIZE) {
    const k = cache.keys().next().value;
    cache.delete(k);
  }
}

async function getClient(id) {
  let c = clients.get(id);
  if (!c) {
    swLog('NEW CLIENT', id.slice(0, 8));
    c = { cache: new Map(), ver: 0, epoch: null, gen: 0 };
    clients.set(id, c);
    pendingRestore.set(id, (async () => {
      const [n, meta] = await Promise.all([restoreCache(c.cache), restoreMeta()]);
      if (n > 0) swLog('  loaded', n, 'events from cache storage');
      if (meta.ver > 0 || meta.epoch !== null) {
        c.ver = meta.ver;
        c.epoch = meta.epoch;
        swLog('  restored meta: ver=', meta.ver, 'epoch=', meta.epoch);
      }
    })());
  }
  const p = pendingRestore.get(id);
  if (p) { await p; pendingRestore.delete(id); }
  return c;
}

// ── ReadableStream-Wrapper ──

function wrapStream(body, clientId, gen) {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  let totalChunks = 0;
  let active = true;

  return new ReadableStream({
    pull(controller) {
      return reader.read().then(function (result) {
        try {
          const client = clients.get(clientId);

          if (!client || client.gen !== gen) {
            if (active) { active = false; try { controller.close(); } catch (e2) {} }
            try { reader.releaseLock(); } catch (e) {}
            return;
          }

          if (result.done) {
            // Stream fertig — verarbeite was noch im buffer ist
            if (buffer.trim()) {
              const norm = buffer.replace(/\r\n/g, '\n');
              const out = processChunk(norm, client);
              if (out) controller.enqueue(new TextEncoder().encode(out));
            }
            storeMeta(client.ver, client.epoch);
            swLog('STREAM DONE client:', clientId.slice(0, 8), 'chunks:', totalChunks, 'cached:', client.cache.size);
            if (active) { active = false; try { controller.close(); } catch (e2) {} }
            try { reader.releaseLock(); } catch (e) {}
            return;
          }

          totalChunks++;
          const decoded = decoder.decode(result.value, { stream: true });
          buffer += decoded;

          // SSE-Events werden durch \n\n getrennt.
          // WICHTIG: Ein Event kann ueber mehrere Chunks verteilt sein!
          // Deshalb sammeln wir Zeilen bis zur Leerzeile (doppeltes \n).
          while (true) {
            // Finde die naechste Leerzeile (\n\n = Event-Trenner)
            const doubleNewlineIdx = buffer.indexOf('\n\n');
            // Oder einfaches \n am Ende (bei stream:true koennen Zeilen unvollstaendig sein)
            // Bei EOF oder Stream-Ende: kein doppeltes \n vorhanden
            
            if (doubleNewlineIdx !== -1) {
              // Vollstaendiges Event gefunden
              const raw = buffer.slice(0, doubleNewlineIdx);
              buffer = buffer.slice(doubleNewlineIdx + 2);
              if (raw.trim()) {
                const norm = raw.replace(/\r\n/g, '\n');
                const out = processChunk(norm, client);
                if (out) controller.enqueue(new TextEncoder().encode(out));
              }
            } else {
              // Kein doppeltes \n gefunden - Event ist noch unvollstaendig
              // Pruefe ob noch mehr Daten kommen werden (result.done = false)
              // Wenn ja, warten wir auf naechsten Chunk
              break;
            }
          }
        } catch (e) {
          swLog('WRAP ERROR:', e.message);
          if (active) { active = false; try { controller.close(); } catch (e2) {} }
          try { reader.releaseLock(); } catch (e2) {}
        }
      }).catch(function (e) {
        if (active) {
          swLog('STREAM ERROR:', e.message);
          storeMeta(client ? client.ver || 0 : 0, client ? client.epoch || null : null);
          active = false;
          try { reader.releaseLock(); } catch (e2) {}
          try { controller.close(); } catch (e2) {}
        }
      });
    },
    cancel: function () {
      if (active) {
        active = false;
        reader.cancel().catch(function () {});
      }
    }
  });
}

// ── Event-Verarbeitung ──

function processChunk(raw, client) {
  var output = [];
  var id = null;
  var hasData = false;
  var isScript = false;

  var lines = raw.split('\n');
  for (var i = 0; i < lines.length; i++) {
    var line = lines[i];
    if (line.indexOf('id:') === 0) id = parseInt(line.slice(3).trim(), 10);
    if (line.indexOf('data:') === 0 || line.indexOf('event:') === 0) hasData = true;
  }

  isScript = raw.indexOf('event: datastar-execute-script') !== -1;

  // Kein id → ignorieren (Keepalive-Kommentare)
  if (id === null || isNaN(id)) {
    return '';
  }

  if (id > client.ver) client.ver = id;

  // id_only: kein data → aus Cache replayen (KEIN early return!)
  if (!hasData) {
    var cached = client.cache.get(id);
    if (cached) {
      swLog('from cache', id);
      output.push(cached);
    }
    // WICHTIG: Hier NICHT return '' - wir müssen output zurückgeben!
    // (Auch wenn cache leer ist, wird das Event trotzdem vom Server replayed)
    return output.join('');
  }

  // ExecuteScript: nicht cachen, nur durchlassen
  if (isScript) {
    output.push(raw + '\n\n');
    return output.join('');
  }

  // Full event: cachen + loggen
  var cached = client.cache.get(id);
  if (cached !== undefined) {
    swLog('from cache', id);
    output.push(cached);
  } else {
    swLog('from server', id);
    var data = raw + '\n\n';
    client.cache.set(id, data);
    storeEvent(id, data);
    evictIfNeeded(client.cache);
    output.push(data);
  }

  return output.join('');
}

// ── Fetch Intercept ──

self.addEventListener('fetch', function (event) {
  try {
    var url = new URL(event.request.url);
    if (url.pathname !== '/sse') return;

    swLog('intercept /sse');

    event.respondWith(
      (async function () {
        var client = await getClient(event.clientId);

        var clientInfo;
        try { clientInfo = await self.clients.get(event.clientId); } catch (e) {}
        var currentUrl = clientInfo ? clientInfo.url : '';

        if (!client.lastUrl) {
          swLog('first connect, url:', currentUrl.slice(0, 60));
        } else {
          swLog('reconnect, cache:', client.cache.size, 'ev, ver:', client.ver);
        }
        client.lastUrl = currentUrl;

        client.gen++;
        swLog('  gen:', client.gen, 'cache:', client.cache.size, 'ver:', client.ver);

        // URL mit resume-Parametern
        var cleanUrl = new URL(url.origin + url.pathname);
        var origParams = new URLSearchParams(url.search);
        for (const [k, v] of origParams) {
          cleanUrl.searchParams.set(k, v);
        }
        if (client.ver > 0) cleanUrl.searchParams.set('v', client.ver);
        if (client.epoch !== null) cleanUrl.searchParams.set('e', client.epoch);
        if (client.pageGen !== undefined && client.pageGen > 0) cleanUrl.searchParams.set('g', client.pageGen);

        var response = await fetch(cleanUrl.toString(), { headers: event.request.headers });

        // Epoch-Check
        var newEpoch = response.headers.get('x-sse-epoch');
        if (newEpoch !== null) {
          var epoch = parseInt(newEpoch, 10);
          if (!isNaN(epoch)) {
            if (client.epoch !== null && client.epoch !== epoch) {
              swLog('epoch mismatch — cache cleared (in-memory + persistent)');
              client.cache.clear();
              client.ver = 0;
              clearPersistentCache();
            }
            client.epoch = epoch;
            storeMeta(client.ver, client.epoch);
          }
        }

        // Page-Gen speichern (vom Server via x-sse-gen Header)
        var newGen = response.headers.get('x-sse-gen');
        if (newGen !== null) {
          var gen = parseInt(newGen, 10);
          if (!isNaN(gen)) {
            client.pageGen = gen;
          }
        }

        if (!response.body) return response;

        return new Response(wrapStream(response.body, event.clientId, client.gen), {
          status: response.status,
          statusText: response.statusText,
          headers: response.headers,
        });
      })().catch(function (err) {
        swLog('fetch error:', err.message);
        return new Response(null, { status: 503 });
      })
    );
  } catch (e) {
    swLog('exception:', e.message);
  }
});
