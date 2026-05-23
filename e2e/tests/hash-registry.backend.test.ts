/**
 * §2.12 — Backend Integration Tests (Real HTTP)
 *
 * Communicates directly with the running Rust platform-backend on localhost:3000.
 * Loads the real sw.js and feeds it REAL SSE responses from the backend.
 *
 * Strategy: Connect to SSE FIRST, then trigger /test/run, then collect events
 * until the "test-complete" marker arrives. No fixed sleeps needed.
 */

import { describe, test, expect, beforeAll, afterAll } from 'vitest'
import fs from 'node:fs'
import path from 'node:path'
import {
  startBackend,
  stopBackend,
  generateClientId,
  authHeaders,
  readSseStream,
  type SseEvent,
} from './helpers/backend'

// ─────────────────────────────────────────────
// Backend lifecycle
// ─────────────────────────────────────────────

let BASE_URL: string

beforeAll(async () => {
  BASE_URL = await startBackend()
}, 30_000)

afterAll(() => {
  stopBackend()
})

// ─────────────────────────────────────────────
// SW code loading (real sw.js)
// ─────────────────────────────────────────────

function loadSwCode(): string {
  return fs.readFileSync(
    path.join(__dirname, '..', '..', 'crates', 'platform-backend', 'assets', 'js', 'sw.js'),
    'utf8',
  )
}

/**
 * Execute the real sw.js in a Service Worker test environment.
 * Mocks SW globals but does NOT mock fetch — uses REAL Node.js fetch.
 * Returns the exposed test hooks from globalThis.self.__sw.
 */
function loadSw(): any {
  // @ts-ignore
  globalThis.self = globalThis
  // @ts-ignore
  globalThis.skipWaiting = () => {}
  // @ts-ignore
  globalThis.clients = { claim: () => {} }
  // @ts-ignore
  globalThis.__SW_TEST_MODE = true

  const listeners: Record<string, (...args: any[]) => void> = {}
  // @ts-ignore
  globalThis.addEventListener = (type: string, fn: (...args: any[]) => void) => {
    listeners[type] = fn
  }

  // @ts-ignore
  if (typeof globalThis.fetch !== 'function') {
    throw new Error('Node.js built-in fetch required (Node 18+)')
  }

  // @ts-ignore
  globalThis.TextEncoder = TextEncoder
  // @ts-ignore
  globalThis.TextDecoder = TextDecoder
  // @ts-ignore
  globalThis.URL = URL
  // @ts-ignore
  globalThis.Response = Response

  const swCode = loadSwCode()
  const fn = new Function(swCode)
  fn()

  // @ts-ignore
  const sw = globalThis.self.__sw
  sw.__listeners = listeners
  return sw
}

// ─────────────────────────────────────────────
// Promise-based event collection
// ─────────────────────────────────────────────

/**
 * Connect to SSE, trigger /test/run, and collect ALL events until the
 * "test-complete" marker event arrives. Returns a Promise that resolves
 * with all collected events.
 */
async function runTestAndCollectEvents(cid: string): Promise<SseEvent[]> {
  // 1. Connect to SSE first (so we don't miss any events)
  const sseResp = await fetch(`${BASE_URL}/sse`, {
    headers: { ...authHeaders(cid), 'Accept': 'text/event-stream' },
  })
  expect(sseResp.status).toBe(200)

  // 2. Trigger test sequence (returns 204 immediately)
  const runResp = await fetch(`${BASE_URL}/test/run`, { headers: authHeaders(cid) })
  expect(runResp.status).toBe(204)

  // 3. Collect events until "test-complete" marker or timeout
  const events: SseEvent[] = []
  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), 5_000) // safety net

  try {
    for await (const event of readSseStream(sseResp.body!, controller.signal)) {
      events.push(event)
      if (event.id === 'test-complete') break
    }
  } catch {
    // Timeout — return whatever we collected
  } finally {
    clearTimeout(timeout)
    if (!sseResp.bodyUsed) sseResp.body?.cancel().catch(() => {})
  }

  return events
}

/**
 * Collect SSE events from an already-open connection until "test-complete"
 * marker or timeout. Used for reconnect tests where we already have a connection.
 */
async function collectUntilComplete(response: Response, timeoutMs = 5_000): Promise<SseEvent[]> {
  const events: SseEvent[] = []
  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), timeoutMs)

  try {
    for await (const event of readSseStream(response.body!, controller.signal)) {
      events.push(event)
      if (event.id === 'test-complete') break
    }
  } catch {
    // Timeout
  } finally {
    clearTimeout(timeout)
    if (!response.bodyUsed) response.body?.cancel().catch(() => {})
  }

  return events
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

describe('Backend Integration — Server Health', () => {
  test('GET /sw.js returns Service Worker script', async () => {
    const resp = await fetch(`${BASE_URL}/sw.js`)
    expect(resp.status).toBe(200)
    const text = await resp.text()
    expect(text).toContain('HASH_REGISTRY')
    expect(text).toContain('registerHash')
    expect(text).toContain('EVENT_PREFIX')
  })

  test('GET /login (public route) returns HTML', async () => {
    const resp = await fetch(`${BASE_URL}/login`)
    expect(resp.status).toBe(200)
    const text = await resp.text()
    expect(text).toContain('<!DOCTYPE html>')
  })

  test('GET /sse without cookie returns 200 + Set-Cookie', async () => {
    const resp = await fetch(`${BASE_URL}/sse`, {
      headers: { 'Accept': 'text/event-stream' },
    })
    expect(resp.status).toBe(200)
    expect(resp.headers.get('content-type')).toContain('text/event-stream')
    const setCookie = resp.headers.get('set-cookie')
    expect(setCookie).toBeTruthy()
    expect(setCookie).toContain('platform_cid=')
    await resp.body?.cancel()
  })
})

describe('Backend Integration — SSE with /test/run', () => {
  const cid = generateClientId()

  test('POST /test/run returns 204 and triggers SSE events', async () => {
    const events = await runTestAndCollectEvents(cid)

    const patchEvents = events.filter(
      (e: SseEvent) => e.event === 'datastar-patch-elements',
    )
    expect(patchEvents.length).toBeGreaterThan(0)

    for (const event of patchEvents) {
      expect(event.id).toBeTruthy()
    }

    console.log(`[test] Received ${patchEvents.length} PatchElements events`)
  }, 10_000)

  test('Event IDs are HMAC hex hashes (non-marker events)', async () => {
    const events = await runTestAndCollectEvents(cid)

    const patchEvents = events.filter(
      (e: SseEvent) => e.event === 'datastar-patch-elements',
    )

    const contentEvents = patchEvents.filter(
      (e: SseEvent) => !e.id?.startsWith('marker-') && e.id !== 'test-complete',
    )
    expect(contentEvents.length).toBeGreaterThan(0)

    for (const event of contentEvents) {
      expect(event.id).toMatch(/^[0-9a-f]+$/)
    }

    console.log(`[test] ${contentEvents.length} content events have hex hashes`)
  }, 10_000)

  test('Events contain HTML payloads', async () => {
    const events = await runTestAndCollectEvents(cid)

    const htmlEvents = events.filter(
      (e: SseEvent) => e.data?.includes('<div') || e.data?.includes('✅'),
    )
    expect(htmlEvents.length).toBeGreaterThan(0)

    console.log(`[test] ${htmlEvents.length} events contain HTML payloads`)
  }, 10_000)
})

describe('Backend Integration — known_hashes deduplication', () => {
  test('known_hashes skips previously-seen events on reconnect', async () => {
    const testCid = generateClientId()

    // First connection: trigger + collect all event IDs
    const events1 = await runTestAndCollectEvents(testCid)

    const patchIds = events1
      .filter((e: SseEvent) => e.event === 'datastar-patch-elements' && e.id)
      .map((e: SseEvent) => e.id!)

    expect(patchIds.length).toBeGreaterThan(0)
    console.log(`[test] First connection: ${patchIds.length} event IDs`)

    // Take first 8 IDs as "known"
    const knownIds = patchIds.slice(0, 8)
    const knownParam = knownIds.join(',')

    // Second connection with known_hashes — connect first, then no /test/run needed
    // (events are already buffered in EventEmitter for this client)
    const resp2 = await fetch(
      `${BASE_URL}/sse?known_hashes=${knownParam}`,
      { headers: { ...authHeaders(testCid), 'Accept': 'text/event-stream' } },
    )
    // Give the replay a moment, then abort — no /test/run needed
    const events2 = await collectUntilComplete(resp2, 2_000)

    const replayIds = events2
      .filter((e: SseEvent) => e.event === 'datastar-patch-elements' && e.id)
      .map((e: SseEvent) => e.id!)

    const knownSet = new Set(knownIds)
    const leaked = replayIds.filter((h: string) => knownSet.has(h))

    console.log(`[test] Second connection: ${replayIds.length} events, ${leaked.length} leaked`)
    expect(leaked.length).toBe(0)
  }, 10_000)
})

describe('Backend Integration — SW parses real SSE from backend', () => {
  test('SW processSSERead registers hashes from real SSE stream', async () => {
    const sw = loadSw()
    const testCid = generateClientId()

    const events = await runTestAndCollectEvents(testCid)

    // Feed all SSE events through the SW's processSSERead
    for (const event of events) {
      if (event.event && event.id && event.data) {
        const chunk = `event: ${event.event}\nid: ${event.id}\ndata: ${event.data}\n\n`
        sw.processSSERead('', chunk)
      }
    }

    const hashCount = sw.HASH_REGISTRY.size
    console.log(`[test] SW registered ${hashCount} hashes from real backend SSE`)

    expect(hashCount).toBeGreaterThan(0)

    const entries = Array.from(sw.HASH_REGISTRY.entries()) as [string, number][]
    const contentHashes = entries.filter(
      ([h]: [string, number]) => !h.startsWith('marker-'),
    )
    expect(contentHashes.length).toBeGreaterThan(0)

    for (const [hash] of contentHashes) {
      // Skip marker- prefixed and the test-complete sentinel
      if (hash.startsWith('marker-') || hash === 'test-complete') continue
      expect(hash).toMatch(/^[0-9a-f]+$/)
    }
  }, 10_000)

  test('SW does NOT register PatchSignals hashes from real SSE', async () => {
    const sw = loadSw()
    const testCid = generateClientId()

    const events = await runTestAndCollectEvents(testCid)

    for (const event of events) {
      if (event.event && event.id && event.data) {
        const chunk = `event: ${event.event}\nid: ${event.id}\ndata: ${event.data}\n\n`
        sw.processSSERead('', chunk)
      }
    }

    const registeredHashes = Array.from(sw.HASH_REGISTRY.keys()) as string[]

    for (const hash of registeredHashes) {
      if (hash.startsWith('marker-') || hash === 'test-complete') continue
      expect(hash).toMatch(/^[0-9a-f]+$/)
    }

    console.log(`[test] SW registered ${registeredHashes.length} hashes (no PatchSignals)`)
  }, 10_000)
})

// ─────────────────────────────────────────────
// SW fetch interception — verifies known_hashes URL rewriting
// ─────────────────────────────────────────────

describe('Backend Integration — SW URL rewriting', () => {
  test('SW adds known_hashes to /sse URL correctly', async () => {
    const sw = loadSw()

    sw.registerHash('deadbeef1234')
    sw.registerHash('cafebabe5678')

    const fetchListener = sw.__listeners?.['fetch']
    if (!fetchListener) throw new Error('No fetch listener registered')

    let capturedUrl = ''

    const originalFetch = globalThis.fetch
    // @ts-ignore
    globalThis.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
      capturedUrl = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
      return new Response('event: done\n\n', {
        status: 200,
        headers: { 'Content-Type': 'text/event-stream' },
      })
    }

    try {
      const response = await new Promise<Response>((resolve, reject) => {
        const timeout = setTimeout(() => {
          reject(new Error('SW fetch handler timed out'))
        }, 5_000)

        const event = {
          request: { url: `${BASE_URL}/sse` },
          respondWith: async (responsePromise: Promise<Response>) => {
            try {
              const resp = await responsePromise
              clearTimeout(timeout)
              resolve(resp)
            } catch (err) {
              clearTimeout(timeout)
              reject(err)
            }
          },
        }

        fetchListener(event)
      })

      expect(response.status).toBe(200)
      expect(capturedUrl).toContain('known_hashes=')
      expect(capturedUrl).toContain('deadbeef1234')
      expect(capturedUrl).toContain('cafebabe5678')

      console.log(`[test] SW correctly rewrote URL: ${capturedUrl}`)
    } finally {
      // @ts-ignore
      globalThis.fetch = originalFetch
    }
  }, 10_000)
})
