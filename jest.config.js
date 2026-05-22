/**
 * Jest configuration for Service Worker hash-sync tests.
 * Tests are run in a Node.js environment with mocked fetch/ServiceWorker APIs.
 */
module.exports = {
  testEnvironment: 'node',
  testMatch: ['**/sw.test.js'],
  transform: {},
};
