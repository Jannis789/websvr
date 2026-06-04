// Service Worker — SSE caching via ReadableStream (kein response.clone!)
//
// Jeder SSE-Client hat seinen eigenen Cache (Map<clientId, {cache, ver, epoch}>).
// Die Response wird in einen ReadableStream gewrappt — der SW ist der EINZIGE
// Konsument. Kein clone, kein paralleler Stream, kein NS_BINDING_ABORTED.
//
// Bei jedem /sse-Intercept: Cache leeren + ver=0 (Server schickt alles neu).
// Der Cache hilft nur innerhalb EINES SSE-Streams gegen id-only replays.

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

// ── Per-Client State ──
const clients = new Map();

function getClient(id) {
  let c = clients.get(id);
  if (!c) {
    swLog('NEW CLIENT', id.slice(0,8));
    c = { cache: new Map(), ver: 0, epoch: null };
    clients.set(id, c);
  }
  return c;
}

function evictIfNeeded(cache) {
  while (cache.size > 200) {
    const k = cache.keys().next().value;
    cache.delete(k);
  }
}

// ── ReadableStream — wrappt SSE-Response, cached Events beim Durchreichen ──
function wrapStream(body, clientId) {
  const reader = body.getReader();
  const decoder = new TextDecoder();
  let buffer = '';
  let totalChunks = 0;

  return new ReadableStream({
    pull(controller) {
      return reader.read().then(({ done, value }) => {
        const client = clients.get(clientId);
        // Client wurde inzwischen geleert (neuer Cycle) — Events ignorieren
        if (!client) {
          controller.close();
          reader.releaseLock();
          return;
        }

        if (done) {
          // Letzten Rest verarbeiten
          if (buffer.trim()) {
            const result = processChunk(buffer.replace(/\r\n/g, '\n'), client);
            if (result) controller.enqueue(new TextEncoder().encode(result));
          }
          swLog('STREAM DONE', 'client:', clientId.slice(0,8), 'total_chunks:', totalChunks);
          controller.close();
          reader.releaseLock();
          return;
        }

        totalChunks++;
        const decoded = decoder.decode(value, { stream: true });
        buffer += decoded;

        // Events aus dem Buffer extrahieren (getrennt durch \n\n)
        while (true) {
          const norm = buffer.replace(/\r\n/g, '\n');
          const idx = norm.indexOf('\n\n');
          if (idx === -1) break; // warten auf mehr Daten

          const raw = norm.slice(0, idx);
          const origLen = buffer.length;
          const normLen = norm.length;
          const consumed = idx + 2 + (origLen - normLen);
          buffer = buffer.slice(Math.min(consumed, buffer.length));

          if (raw.trim()) {
            const result = processChunk(raw, client);
            if (result) controller.enqueue(new TextEncoder().encode(result));
          }
        }
      });
    },
    cancel() {
      // KEIN reader.cancel() — der HTTP-Connection stirbt von selbst.
      // Sonst: ERR_INCOMPLETE_CHUNKED_ENCODING / NS_BINDING_ABORTED.
      reader.releaseLock();
    },
  });
}

// ── Event-Verarbeitung (lernt + cached) ──
function processChunk(raw, client) {
  const output = [];

  let id = null;
  let hasData = false;
  let isScript = false;

  for (const line of raw.split('\n')) {
    if (line.startsWith('id:')) id = parseInt(line.slice(3).trim(), 10);
    if (line.startsWith('data:') || line.startsWith('event:')) hasData = true;
  }

  isScript = raw.includes('event: datastar-execute-script');

  // Live-Event (keine id) — unverändert durchlassen
  if (id === null || isNaN(id)) {
    if (hasData) output.push(raw + '\n\n');
    return output.join('');
  }

  // lastPatchVer aktualisieren
  if (id > client.ver) client.ver = id;

  // id-only Event — aus Cache bedienen wenn vorhanden
  if (!hasData) {
    const cached = client.cache.get(id);
    if (cached) {
      swLog('from cache', id);
      output.push(cached);
    }
    return output.join('');
  }

  // ExecuteScript — nicht cachen, nur durchlassen
  if (isScript) {
    output.push(raw + '\n\n');
    return output.join('');
  }

  // Full Event — cachen und durchlassen
  const cached = client.cache.get(id);
  if (cached !== undefined) {
    swLog('from cache', id);
    output.push(cached);
  } else {
    swLog('from server', id);
    client.cache.set(id, raw + '\n\n');
    evictIfNeeded(client.cache);
    output.push(raw + '\n\n');
  }

  return output.join('');
}

// ── Fetch Intercept ──
self.addEventListener('fetch', (event) => {
  try {
    const url = new URL(event.request.url);
    if (url.pathname !== '/sse') return;

    swLog('intercept /sse');

    event.respondWith(
      (async () => {
        // Navigation erkennen via Client-URL-Vergleich
        // event.clientId bleibt bei Navigation GLEICH (gleicher Tab).
        // self.clients.get() liefert die aktuelle URL des Clients.
        let clientInfo;
        try { clientInfo = await self.clients.get(event.clientId); } catch {}
        const client = getClient(event.clientId);
        const currentUrl = clientInfo?.url || '';

        if (client.lastUrl && client.lastUrl !== currentUrl) {
          swLog('navigation:', client.lastUrl.slice(0,60), '→', currentUrl.slice(0,60));
          swLog('  cache cleared (was:', client.cache.size, 'ev, ver:', client.ver, ')');
          client.cache.clear();
          client.ver = 0;
        } else if (!client.lastUrl) {
          swLog('first connect, url:', currentUrl.slice(0,60));
        } else {
          swLog('reconnect (same url), cache:', client.cache.size, 'ev, ver:', client.ver);
        }
        client.lastUrl = currentUrl;

        // URL modifizieren — Datastar-Parameter erhalten, Resume-Parameter setzen
        const cleanUrl = new URL(url.origin + url.pathname);
        const origParams = new URLSearchParams(url.search);
        for (const [k, v] of origParams) cleanUrl.searchParams.set(k, v);
        if (client.ver > 0) cleanUrl.searchParams.set('v', client.ver);
        if (client.epoch !== null) cleanUrl.searchParams.set('e', client.epoch);

        const response = await fetch(cleanUrl.toString(), { headers: event.request.headers });

        // Epoch prüfen
        const newEpoch = response.headers.get('x-sse-epoch');
        if (newEpoch !== null) {
          const epoch = parseInt(newEpoch, 10);
          if (!isNaN(epoch)) {
            if (client.epoch !== null && client.epoch !== epoch) {
              swLog('epoch mismatch — cache cleared');
              client.cache.clear();
              client.ver = 0;
            }
            client.epoch = epoch;
          }
        }

        if (!response.body) return response;

        // Response wrappen — KEIN response.clone()!
        // Der ReadableStream ist der EINZIGE Konsument des Bodys.
        return new Response(wrapStream(response.body, event.clientId), {
          status: response.status,
          statusText: response.statusText,
          headers: response.headers,
        });
      })()
        .catch((err) => {
          swLog('fetch error:', err.message);
          return new Response('', { status: 503, statusText: 'Service Unavailable' });
        })
    );
  } catch (e) {
    swLog('exception:', e.message);
  }
});
