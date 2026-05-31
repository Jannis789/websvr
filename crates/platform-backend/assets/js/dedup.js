// Datastar Dedup Plugin — tracks SSE event hashes for deduplication
//
// Listens to `datastar-fetch` custom events dispatched by Datastar's core.
// `evt.detail.argsRaw` is already parsed key-value pairs from the SSE data.
// We hash `argsRaw` values joined the same way as write_data() output.
// Actually — we don't need to reconstruct anything. The SSE `data` field
// IS the write_data() output. But argsRaw loses line ordering.
//
// Better approach: patch into Datastar's fetch to intercept the raw SSE
// data lines before they're dispatched.

(function () {
  // ── HMAC-SHA256 via Web Crypto ──

  async function computeHash(content) {
    const cookieValue = getCookieValue('platform_cid');
    if (!cookieValue) return null;

    const encoder = new TextEncoder();
    const key = await crypto.subtle.importKey(
      'raw',
      encoder.encode(cookieValue),
      { name: 'HMAC', hash: 'SHA-256' },
      false,
      ['sign']
    );
    const sig = await crypto.subtle.sign('HMAC', key, encoder.encode(content));
    const truncated = new Uint8Array(sig, 0, 16);
    return Array.from(truncated).map(b => b.toString(16).padStart(2, '0')).join('');
  }

  function getCookieValue(name) {
    const match = document.cookie.split(';').map(s => s.trim()).find(pair => {
      const [key] = pair.split('=');
      return key === name;
    });
    return match ? match.split('=').slice(1).join('=') : null;
  }

  // ── Hash tracking ──

  const seenHashes = new Set();

  // The `datastar-fetch` event gives us `argsRaw` — parsed key-value pairs.
  // We reconstruct the write_data() payload by joining values in order.
  // This matches server-side write_data() output for PatchElements:
  //   selector X\nmode Y\nelements Z
  // For PatchSignals: signals {...}
  function buildPayload(argsRaw) {
    const keys = ['selector', 'mode', 'useViewTransition', 'elements', 'signals', 'onlyIfMissing'];
    const parts = [];
    for (const k of keys) {
      if (k in argsRaw) parts.push(k + ' ' + argsRaw[k]);
    }
    return parts.join('\n');
  }

  document.addEventListener('datastar-fetch', async (evt) => {
    const { type, argsRaw } = evt.detail;
    if (!type || !type.startsWith('datastar-')) return;
    if (!type.startsWith('datastar-patch-')) return;

    const payload = buildPayload(argsRaw);
    const hash = await computeHash(payload);
    if (hash) {
      seenHashes.add(hash);
      if (navigator.serviceWorker?.controller) {
        navigator.serviceWorker.controller.postMessage({
          type: 'known-hashes',
          hashes: Array.from(seenHashes),
        });
      }
    }
  });
})();
