// App-Initialisierung: Service Worker + SSE (sofort, kein Warten auf SW)
import { actions } from '/assets/js/datastar.js';

function now() {
  return performance.now().toFixed(1);
}

console.log('[init] app-init.js loaded at ' + now());

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

  console.log('[init] registering SW /sw2.js at ' + now());

  var regStart = performance.now();
  navigator.serviceWorker.register('/sw2.js', {
    updateViaCache: 'none',
  }).then(function (reg) {
    var regMs = (performance.now() - regStart).toFixed(1);
    var state = reg.active ? 'active' : reg.installing ? 'installing' : 'waiting';
    console.log('[init] SW registered in ' + regMs + 'ms state=' + state +
      ' controller=' + (navigator.serviceWorker.controller ? 'YES' : 'NO') +
      ' waiting=' + (reg.waiting ? 'YES' : 'NO') +
      ' at ' + now());
  }).catch(function (err) {
    console.error('[init] SW registration failed at ' + now() + ':', err && err.message);
  });
} else {
  console.log('[init] ServiceWorker NOT supported');
}

// ── SSE sofort starten ──
// Kein navigator.serviceWorker.ready — das blockiert in Chromium bis zu 15s.
// Der SW aktiviert + claimt im Hintergrund; SSE-Caching greift ab dem
// nächsten Page-Load (Reconnect nach Navigation).
// Der erste Stream läuft direkt zum Server — völlig in Ordnung.

console.log('[init] starting SSE (SW controlled: ' +
  (navigator.serviceWorker && navigator.serviceWorker.controller ? 'YES' : 'NO') + ') at ' + now());

(function initSse() {
  var sseEl = document.createElement('div');
  sseEl.hidden = true;
  document.body.appendChild(sseEl);

  console.log('[init] SSE POST /sse at ' + now());
  var sseStart = performance.now();
  actions.post({ el: sseEl }, '/sse').then(function () {
    console.log('[init] SSE POST resolved at ' + now() + ' (took ' + (performance.now() - sseStart).toFixed(1) + 'ms)');
  }).catch(function (err) {
    console.log('[init] SSE POST rejected at ' + now() + ' (took ' + (performance.now() - sseStart).toFixed(1) + 'ms):', err && err.message);
  });
})();

// ── Page-Lifecycle ──

var lifecycleLog = function (event, data) {
  console.log('[init] lifecycle: ' + event + (data ? ' ' + data : '') + ' at ' + now());
  if (navigator.serviceWorker && navigator.serviceWorker.controller) {
    console.log('[init]  → posting message type=' + (data || event));
    navigator.serviceWorker.controller.postMessage({ type: data || event });
  } else {
    console.log('[init]  → NO controller to postMessage');
  }
};

window.addEventListener('beforeunload', function () {
  lifecycleLog('beforeunload');
});

document.addEventListener('visibilitychange', function () {
  if (document.hidden) {
    lifecycleLog('visibilitychange', 'sse-close');
  }
});
