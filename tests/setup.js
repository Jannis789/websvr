// Jest setup: mock Service Worker globals and load real sw.js
const fs = require('fs');
const path = require('path');
const { TextDecoder } = require('util');

// Enable test mode before loading sw.js
globalThis.__SW_TEST_MODE = true;

global.self = global;
global.skipWaiting = jest.fn();
global.clients = { claim: jest.fn() };
global.fetch = jest.fn();

const listeners = {};
global.addEventListener = jest.fn((type, fn) => {
  listeners[type] = fn;
});
global.removeEventListener = jest.fn((type) => {
  delete listeners[type];
});
global.TextDecoder = TextDecoder;

// Mock Response constructor (not available in Node)
global.Response = class Response {
  constructor(body, init = {}) {
    this.body = body;
    this.status = init.status || 200;
    this.statusText = init.statusText || '';
    this.headers = init.headers || {};
  }
};

// Mock URL (available in Node 10+, but ensure it's there)
if (!global.URL) {
  global.URL = require('url').URL;
}

/**
 * Load the real sw.js and return the exposed test hooks.
 * Call this in beforeEach() to get a fresh registry state.
 */
global.loadSw = () => {
  // Clear previous state
  const keys = Object.keys(listeners);
  for (const k of keys) delete listeners[k];

  // Load sw.js source and evaluate in this context
  const swPath = path.join(__dirname, '..', 'crates', 'platform-backend', 'assets', 'js', 'sw.js');
  const swCode = fs.readFileSync(swPath, 'utf8');
  const fn = new Function(swCode);
  fn();

  return self.__sw;
};

/**
 * Dispatch a fetch event through the registered listener.
 * Returns the event object with a tracked respondWith call.
 */
global.dispatchFetchEvent = (url, respondWithOverride) => {
  const respondWith = respondWithOverride || jest.fn();
  const event = {
    request: { url },
    respondWith,
  };
  if (listeners['fetch']) {
    listeners['fetch'](event);
    return event;
  }
  throw new Error('No fetch listener registered');
};

/**
 * Dispatch install/activate lifecycle events.
 */
global.dispatchLifecycleEvent = (type) => {
  const event = { waitUntil: jest.fn() };
  if (listeners[type]) {
    listeners[type](event);
  }
  return event;
};
