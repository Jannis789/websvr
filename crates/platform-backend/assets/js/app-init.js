// App-Initialisierung: Service Worker + SSE (sofort, kein Warten auf SW)
import { actions } from '/assets/js/datastar.js';

// ── Service Worker (asynchron, blockiert nichts) ──
// Scope: NICHT auf /sse einschränken — Firefox interpretiert Scope inkonsistent.
// Der fetch-Handler in sw.js filtert sowieso nur /sse-Pfade (Line 84).
// Root-Scope (Default, kein scope-Option) = SW kontrolliert alle Pages → zuverlässig.

if ('serviceWorker' in navigator) {
  // SW-Logs in die DevTools-Console forwarden
  navigator.serviceWorker.addEventListener('message', function (evt) {
    if (evt.data && evt.data.type === 'sw-log') {
      console.log('[sw]', ...evt.data.args);
    }
  });

  console.log('[init] registering SW /sw2.js ...');

  navigator.serviceWorker.register('/sw2.js', {
    updateViaCache: 'none',
  }).then(function (reg) {
    var state = reg.active ? 'active' : reg.installing ? 'installing' : 'waiting';
    console.log('[init] SW registered, state: ' + state +
      ' | controller: ' + (navigator.serviceWorker.controller ? 'YES' : 'NO') +
      ' | waiting: ' + (reg.waiting ? 'YES' : 'NO'));
  }).catch(function (err) {
    console.error('[init] SW registration failed:', err);
  });
}

// ── SSE sofort starten ──
// Kein navigator.serviceWorker.ready — das blockiert in Chromium bis zu 15s.
// Der SW aktiviert + claimt im Hintergrund; SSE-Caching greift ab dem
// nächsten Page-Load (Reconnect nach Navigation).
// Der erste Stream läuft direkt zum Server — völlig in Ordnung.

console.log('[init] starting SSE (SW controlled: ' +
  (navigator.serviceWorker && navigator.serviceWorker.controller ? 'YES' : 'NO') + ')');

(function initSse() {
  var sseEl = document.createElement('div');
  sseEl.hidden = true;
  document.body.appendChild(sseEl);

  actions.post({ el: sseEl }, '/sse').catch(function (err) {
    console.warn('[init] SSE POST /sse failed:', err);
  });
})();

// ── Page-Lifecycle ──

// beforeunload: SW zwingen, den inneren fetch zum Server abzubrechen.
// Ohne expliziten Abbruch bleibt die alte Server-Verbindung offen
// (tee() propagiert den Tab-Tod nicht zuverlässig durch Chromium).
// Der Server blockiert dann die neue SSE-Verbindung → "pending".
window.addEventListener('beforeunload', function () {
  if (navigator.serviceWorker && navigator.serviceWorker.controller) {
    navigator.serviceWorker.controller.postMessage({ type: 'sse-close' });
  }
});

document.addEventListener('visibilitychange', function () {
  if (document.hidden) {
    navigator.serviceWorker && navigator.serviceWorker.controller &&
      navigator.serviceWorker.controller.postMessage({ type: 'sse-close' });
  }
});
