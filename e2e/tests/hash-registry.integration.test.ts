/**
 * §2.12 — Integration Tests: Service Worker + MSW
 *
 * Self-contained: creates its own MSW server and SW environment.
 * Verifies the SW's fetch interception, known_hashes appending,
 * and SSE hash learning via MSW-mocked HTTP responses.
 */

import { describe, test, expect, beforeEach, afterEach, beforeAll, afterAll, vi } from 'vitest'
import { setupServer } from 'msw/node'
import { http, HttpResponse } from 'msw'
import fs from 'node:fs'
import path from 'node:path'

// ─────────────────────────────────────────────
// MSW Server (self-contained)
// ─────────────────────────────────────────────

const callLog: { url: string; knownHashes: string }[] = []

let _sseEvents: string[] = []

const server = setupServer(
  http.get('http://localhost:3000/sse', ({ request }) => {
    const url = new URL(request.url)
    const knownHashes = url.searchParams.get('known_hashes') || ''
    callLog.push({ url: request.url, knownHashes })

    const encoder = new TextEncoder()
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        for (const event of _sseEvents) {
          controller.enqueue(encoder.encode(event))
        }
        controller.close()
      },
    })

    return new HttpResponse(stream, {
      status: 200,
      headers: {
        'Content-Type': 'text/event-stream',
        'Cache-Control': 'no-cache',
      },
    })
  }),
)

beforeAll(() => server.listen({ onUnhandledRequest: 'warn' }))
afterEach(() => {
  callLog.length = 0
  _sseEvents = []
  server.resetHandlers()
})
afterAll(() => server.close())

// ─────────────────────────────────────────────
// SW Environment
// ─────────────────────────────────────────────

const listeners: Record<string, (...args: any[]) => void> = {}

;(globalThis as any).self = globalThis
;(globalThis as any).skipWaiting = vi.fn()
;(globalThis as any).clients = { claim: vi.fn() }

;(globalThis as any).addEventListener = vi.fn((type: string, fn: (...args: any[]) => void) => {
  listeners[type] = fn
})
;(globalThis as any).removeEventListener = vi.fn((type: string) => {
  delete listeners[type]
})

// Ensure TextEncoder/TextDecoder
;(globalThis as any).TextEncoder = globalThis.TextEncoder ?? TextEncoder
;(globalThis as any).TextDecoder = globalThis.TextDecoder ?? TextDecoder
;(globalThis as any).URL = globalThis.URL ?? URL

function loadSw(): any {
  for (const k of Object.keys(listeners)) delete listeners[k]
  ;(globalThis as any).__SW_TEST_MODE = true

  const swPath = path.join(__dirname, '..', '..', 'crates', 'platform-backend', 'assets', 'js', 'sw.js')
  const swCode = fs.readFileSync(swPath, 'utf8')
  const fn = new Function(swCode)
  fn()

  return (globalThis as any).self.__sw
}

/**
 * Dispatch a fetch event to the SW and return a promise that resolves
 * after the fetch completes AND background stream processing finishes.
 *
 * The SW's consumeSSEStream runs in the background (not awaited).
 * We resolve on the next macrotask after the fetch completes to ensure
 * all microtask-based stream reads have been processed.
 *
 * If the SW doesn't call event.respondWith() (e.g. non-/sse paths),
 * the promise resolves immediately after the listener returns.
 */
function dispatchFetchEvent(url: string): Promise<void> {
  return new Promise<void>((resolve) => {
    let settled = false

    const event = {
      request: { url },
      respondWith: vi.fn((promise: Promise<any>) => {
        settled = true
        promise.then(
          () => setTimeout(resolve, 0),  // yield to macrotask → background stream done
          () => setTimeout(resolve, 0),
        )
      }),
    }

    const fetchListener = listeners['fetch']
    if (!fetchListener) throw new Error('No fetch listener registered')
    fetchListener(event)

    // respondWith is called synchronously inside the listener for /sse paths.
    // For non-/sse paths it's never called → resolve immediately.
    if (!settled) resolve()
  })
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

let sw: any

beforeEach(() => {
  sw = loadSw()
})

describe('MSW Integration — SW Lifecycle', () => {
  test('install event calls skipWaiting', () => {
    const event = { waitUntil: vi.fn() }
    listeners['install']?.(event)
    expect((globalThis as any).skipWaiting).toHaveBeenCalled()
  })

  test('activate event calls clients.claim', () => {
    const event = { waitUntil: vi.fn() }
    listeners['activate']?.(event)
    expect(event.waitUntil).toHaveBeenCalled()
    expect((globalThis as any).clients.claim).toHaveBeenCalled()
  })
})

describe('MSW Integration — Fetch Interception', () => {
  test('appends known_hashes when registry has entries', async () => {
    sw.registerHash('hash-alpha')
    sw.registerHash('hash-beta')

    await dispatchFetchEvent('http://localhost:3000/sse')

    expect(callLog.length).toBeGreaterThanOrEqual(1)
    const first = callLog[0]
    expect(first.url).toContain('known_hashes=')
    expect(first.url).toContain('hash-alpha')
    expect(first.url).toContain('hash-beta')
  })

  test('does NOT append known_hashes when registry is empty', async () => {
    await dispatchFetchEvent('http://localhost:3000/sse')

    expect(callLog.length).toBeGreaterThanOrEqual(1)
    expect(callLog[0].url).not.toContain('known_hashes')
  })

  test('does NOT intercept non-/sse requests', () => {
    dispatchFetchEvent('http://localhost:3000/home')
    expect(callLog.length).toBe(0)
  })
})

describe('MSW Integration — Hash Learning from SSE Stream', () => {
  test('registers PatchElements hashes from the SSE stream', async () => {
    _sseEvents = [
      'event: datastar-patch-elements\nid: msw-hash-001\ndata: <div>A</div>\n\n',
      'event: datastar-patch-elements\nid: msw-hash-002\ndata: <div>B</div>\n\n',
    ]

    await dispatchFetchEvent('http://localhost:3000/sse')

    expect(sw.HASH_REGISTRY.has('msw-hash-001')).toBe(true)
    expect(sw.HASH_REGISTRY.has('msw-hash-002')).toBe(true)
  })

  test('does NOT register PatchSignals hashes', async () => {
    _sseEvents = [
      'event: datastar-patch-elements\nid: msw-patch-001\ndata: A\n\n',
      'event: datastar-patch-signals\nid: msw-signal-001\ndata: {"x":1}\n\n',
      'event: datastar-patch-elements\nid: msw-patch-002\ndata: B\n\n',
    ]

    await dispatchFetchEvent('http://localhost:3000/sse')

    expect(sw.HASH_REGISTRY.has('msw-patch-001')).toBe(true)
    expect(sw.HASH_REGISTRY.has('msw-signal-001')).toBe(false)
    expect(sw.HASH_REGISTRY.has('msw-patch-002')).toBe(true)
    expect(sw.HASH_REGISTRY.size).toBe(2)
  })
})

describe('MSW Integration — Hash Registry Persistence', () => {
  test('hashes learned from stream appear in next known_hashes', async () => {
    _sseEvents = [
      'event: datastar-patch-elements\nid: first-hash-a\ndata: <div>1A</div>\n\n',
      'event: datastar-patch-elements\nid: first-hash-b\ndata: <div>1B</div>\n\n',
    ]

    await dispatchFetchEvent('http://localhost:3000/sse')

    expect(sw.HASH_REGISTRY.has('first-hash-a')).toBe(true)
    expect(sw.HASH_REGISTRY.has('first-hash-b')).toBe(true)

    // Phase 2: Second request includes learned hashes
    callLog.length = 0
    await dispatchFetchEvent('http://localhost:3000/sse')

    expect(callLog.length).toBeGreaterThanOrEqual(1)
    expect(callLog[0].url).toContain('first-hash-a')
    expect(callLog[0].url).toContain('first-hash-b')
  })

  test('expired hashes are excluded from known_hashes', async () => {
    _sseEvents = [
      'event: datastar-patch-elements\nid: fresh-hash\ndata: A\n\n',
    ]
    await dispatchFetchEvent('http://localhost:3000/sse')

    sw.HASH_REGISTRY.set('expired-hash', Date.now() - 25 * 60 * 60 * 1000)

    callLog.length = 0
    await dispatchFetchEvent('http://localhost:3000/sse')

    expect(callLog.length).toBeGreaterThanOrEqual(1)
    expect(callLog[0].url).toContain('fresh-hash')
    expect(callLog[0].url).not.toContain('expired-hash')
  })
})

describe('MSW Integration — Full Round-Trip', () => {
  test('in-and-out: events in → hashes learned → known_hashes sent out', async () => {
    _sseEvents = [
      'event: datastar-patch-elements\nid: rt-00001\ndata: <div>E1</div>\n\n',
      'event: datastar-patch-elements\nid: rt-00002\ndata: <div>E2</div>\n\n',
      'event: datastar-patch-elements\nid: rt-00003\ndata: <div>E3</div>\n\n',
      'event: datastar-patch-signals\nid: rt-signal\ndata: {}\n\n',
      'event: datastar-patch-elements\nid: rt-00005\ndata: <div>E5</div>\n\n',
    ]

    await dispatchFetchEvent('http://localhost:3000/sse')

    expect(sw.HASH_REGISTRY.has('rt-00001')).toBe(true)
    expect(sw.HASH_REGISTRY.has('rt-00002')).toBe(true)
    expect(sw.HASH_REGISTRY.has('rt-00003')).toBe(true)
    expect(sw.HASH_REGISTRY.has('rt-signal')).toBe(false)
    expect(sw.HASH_REGISTRY.has('rt-00005')).toBe(true)
    expect(sw.HASH_REGISTRY.size).toBe(4)

    callLog.length = 0
    await dispatchFetchEvent('http://localhost:3000/sse')

    expect(callLog.length).toBeGreaterThanOrEqual(1)
    const url = callLog[0].url
    expect(url).toContain('known_hashes=')
    expect(url).toContain('rt-00001')
    expect(url).toContain('rt-00002')
    expect(url).toContain('rt-00003')
    expect(url).not.toContain('rt-signal')
    expect(url).toContain('rt-00005')
  })
})

describe('MSW Integration — Edge Cases', () => {
  test('EC-5: Empty registry → no known_hashes → server replays all', async () => {
    await dispatchFetchEvent('http://localhost:3000/sse')

    expect(callLog.length).toBeGreaterThanOrEqual(1)
    expect(callLog[0].knownHashes).toBe('')
  })

  test('EC-6: TTL exceeded → hash removed, re-fetched', async () => {
    // Phase 1: learn hash
    _sseEvents = [
      'event: datastar-patch-elements\nid: expiring-hash\ndata: A\n\n',
    ]
    await dispatchFetchEvent('http://localhost:3000/sse')
    expect(sw.HASH_REGISTRY.has('expiring-hash')).toBe(true)

    // Phase 2: age out the hash, then trigger a new SSE request
    // with NO events so consumeSSEStream doesn't re-register it
    sw.HASH_REGISTRY.set('expiring-hash', Date.now() - 25 * 60 * 60 * 1000)

    callLog.length = 0
    _sseEvents = []  // empty stream — no re-registration
    await dispatchFetchEvent('http://localhost:3000/sse')

    expect(callLog.length).toBeGreaterThanOrEqual(1)
    expect(callLog[0].knownHashes).toBe('')
    expect(sw.HASH_REGISTRY.has('expiring-hash')).toBe(false)
  })

  test('MAX_REGISTRY_SIZE eviction removes oldest entries', () => {
    const limit = sw.MAX_REGISTRY_SIZE
    for (let i = 0; i < limit + 5; i++) {
      sw.registerHash(`evict_${i.toString().padStart(5, '0')}`)
    }

    expect(sw.HASH_REGISTRY.size).toBe(limit)
    expect(sw.HASH_REGISTRY.has('evict_00000')).toBe(false)
    expect(sw.HASH_REGISTRY.has(`evict_${(limit + 4).toString().padStart(5, '0')}`)).toBe(true)
  })
})


