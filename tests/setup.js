// Jest setup: mock Service Worker globals
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

global.dispatchFetchEvent = (url, respondWith) => {
  const event = {
    request: { url },
    respondWith: respondWith || jest.fn(),
  };
  if (listeners['fetch']) {
    listeners['fetch'](event);
    return event;
  }
  throw new Error('No fetch listener registered');
};

global.dispatchEvent = (type) => {
  const event = { waitUntil: jest.fn() };
  if (listeners[type]) {
    listeners[type](event);
  }
  return event;
};

const { TextDecoder } = require('util');
global.TextDecoder = TextDecoder;
