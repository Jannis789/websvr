import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    include: [
      'tests/**/dom-snapshots.test.ts',
    ],
    environment: 'happy-dom',
    globals: true,
    testTimeout: 30_000,
    update: false,
  },
})
