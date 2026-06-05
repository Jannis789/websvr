// ds-cache — page-side event cache for instant restore on refresh
//
// Stores the last PatchElements per selector and last PatchSignals
// in sessionStorage. On page load, replays them via datastar-fetch
// for instant UI render before the SSE stream reconnects.

const CACHE_KEY = 'ds-cache';

const CACHE_SLOTS = new Map();
let CACHE_SIGNALS = null;

(function restoreCache() {
  try {
    const raw = sessionStorage.getItem(CACHE_KEY);
    if (!raw) return;
    const { slots, signals } = JSON.parse(raw);
    if (!slots || !slots.length) return;

    for (const [sel, entry] of slots) {
      document.dispatchEvent(new CustomEvent('datastar-fetch', {
        detail: { type: entry.type, el: document.documentElement, argsRaw: entry.argsRaw },
      }));
    }
    if (signals) {
      document.dispatchEvent(new CustomEvent('datastar-fetch', {
        detail: { type: signals.type, el: document.documentElement, argsRaw: signals.argsRaw },
      }));
    }

  } catch (e) {
    // restore failed — will get fresh state from server
  }
})();

document.addEventListener('datastar-fetch', (evt) => {
  const { type, argsRaw } = evt.detail;
  if (!type) return;

  const sel = argsRaw?.selector || '';

  if (type === 'datastar-patch-elements') {
    if (sel && sel !== 'body') CACHE_SLOTS.set(sel, { type, argsRaw });
  } else if (type === 'datastar-patch-signals') {
    CACHE_SIGNALS = { type, argsRaw };
  }
});

window.addEventListener('beforeunload', () => {
  const state = { slots: [...CACHE_SLOTS], signals: CACHE_SIGNALS };
  try { sessionStorage.setItem(CACHE_KEY, JSON.stringify(state)); } catch {}
});
