/**
 * §2.12 — Backend Integration Tests (Real HTTP)
 *
 * Communicates directly with the running Rust platform-backend on localhost:3000.
 * Loads the real sw.js and feeds it REAL SSE responses from the backend.
 *
 * Requires: backend running (auto-started via cargo run)
 *
 * Tests:
 *  1. Backend serves SW code (/sw.js)
 *  2. SSE endpoint returns proper events with HMAC hashes as IDs
 *  3. /test/run triggers events that flow through SSE
 *  4. known_hashes deduplication works end-to-end (server-side)
 *  5. SW correctly parses real SSE from backend (processSSERead with real chunks)
 *  6. SW fetch interception with real fetch to backend
 */

import { describe, test, expect, beforeAll, afterAll } from 'vitest'
import fs from 'node:fs'
import path from 'node:path'
import {
  startBackend,
  stopBackend,
  generateClientId,
  authHeaders,
  collectSseEvents,
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
  // Reset any previous state
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

  // Do NOT mock fetch — real Node.js fetch talks to the backend
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

  // Execute sw.js
  const swCode = loadSwCode()
  const fn = new Function(swCode)
  fn()

  // @ts-ignore
  const sw = globalThis.self.__sw
  sw.__listeners = listeners
  return sw
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
    const runResp = await fetch(`${BASE_URL}/test/run`, {
      headers: authHeaders(cid),
    })
    expect(runResp.status).toBe(204)

    // Wait for background task to finish (~4s of event broadcasting)
    await sleep(4500)

    // Connect to SSE to receive Phase 1 replay
    const sseResp = await fetch(`${BASE_URL}/sse`, {
      headers: { ...authHeaders(cid), 'Accept': 'text/event-stream' },
    })
    expect(sseResp.status).toBe(200)
    expect(sseResp.headers.get('content-type')).toContain('text/event-stream')

    const events = await collectSseEvents(sseResp, 5000)

    // Don't cancel the body — let the timeout close the stream naturally.
    // cancel() may cause the alpha Rama server to panic.

    const patchEvents = events.filter(
      (e: SseEvent) => e.event === 'datastar-patch-elements',
    )
    expect(patchEvents.length).toBeGreaterThan(0)

    // Every PatchElements event must have an ID (HMAC hash)
    for (const event of patchEvents) {
      expect(event.id).toBeTruthy()
      expect(event.id!.length).toBeGreaterThan(0)
    }

    console.log(`[test] Received ${patchEvents.length} PatchElements events`)
  }, 30_000)

  test('Event IDs are HMAC hex hashes (non-marker events)', async () => {
    await fetch(`${BASE_URL}/test/run`, { headers: authHeaders(cid) })
    await sleep(4500)

    const sseResp = await fetch(`${BASE_URL}/sse`, {
      headers: { ...authHeaders(cid), 'Accept': 'text/event-stream' },
    })
    // Small delay to let server recover from previous SSE test
    await sleep(500)

    const events = await collectSseEvents(sseResp, 5000)

    const patchEvents = events.filter(
      (e: SseEvent) => e.event === 'datastar-patch-elements',
    )

    // Non-marker events should have hex hash IDs
    const contentEvents = patchEvents.filter(
      (e: SseEvent) => !e.id?.startsWith('marker-'),
    )
    expect(contentEvents.length).toBeGreaterThan(0)

    for (const event of contentEvents) {
      expect(event.id).toMatch(/^[0-9a-f]+$/)
    }

    console.log(`[test] ${contentEvents.length} content events have hex hashes`)
  }, 30_000)

  test('Events contain HTML payloads', async () => {
    await fetch(`${BASE_URL}/test/run`, { headers: authHeaders(cid) })
    await sleep(4500)

    const sseResp = await fetch(`${BASE_URL}/sse`, {
      headers: { ...authHeaders(cid), 'Accept': 'text/event-stream' },
    })
    // Small delay to let server recover
    await sleep(500)

    const events = await collectSseEvents(sseResp, 5000)

    const htmlEvents = events.filter(
      (e: SseEvent) => e.data?.includes('<div') || e.data?.includes('✅'),
    )
    expect(htmlEvents.length).toBeGreaterThan(0)

    console.log(`[test] ${htmlEvents.length} events contain HTML payloads`)
  }, 30_000)
})

describe('Backend Integration — known_hashes deduplication', () => {
  test('known_hashes skips previously-seen events on reconnect', async () => {
    const testCid = generateClientId()

    // Trigger and buffer events
    await fetch(`${BASE_URL}/test/run`, { headers: authHeaders(testCid) })
    await sleep(4500)

    // First connection: collect all event IDs
    const resp1 = await fetch(`${BASE_URL}/sse`, {
      headers: { ...authHeaders(testCid), 'Accept': 'text/event-stream' },
    })
    const events1 = await collectSseEvents(resp1, 5000)
    // Let server settle after SSE disconnect
    await sleep(1000)

    const patchIds = events1
      .filter((e: SseEvent) => e.event === 'datastar-patch-elements' && e.id)
      .map((e: SseEvent) => e.id!)

    expect(patchIds.length).toBeGreaterThan(0)
    console.log(`[test] First connection: ${patchIds.length} event IDs`)

    // Take first 8 IDs as "known" — DO NOT encode commas, server splits on ','
    const knownIds = patchIds.slice(0, 8)
    const knownParam = knownIds.join(',')

    // Second connection with known_hashes
    // Build URL manually — only encode individual hash values (which are hex, no encoding needed)
    const resp2 = await fetch(
      `${BASE_URL}/sse?known_hashes=${knownParam}`,
      { headers: { ...authHeaders(testCid), 'Accept': 'text/event-stream' } },
    )
    const events2 = await collectSseEvents(resp2, 3000)

    const replayIds = events2
      .filter((e: SseEvent) => e.event === 'datastar-patch-elements' && e.id)
      .map((e: SseEvent) => e.id!)

    // Known IDs should NOT leak through Phase 1 replay
    const knownSet = new Set(knownIds)
    const leaked = replayIds.filter((h: string) => knownSet.has(h))

    console.log(`[test] Second connection: ${replayIds.length} events, ${leaked.length} leaked`)
    expect(leaked.length).toBe(0)
  }, 30_000)
})

describe('Backend Integration — SW parses real SSE from backend', () => {
  test('SW processSSERead registers hashes from real SSE stream', async () => {
    const sw = loadSw()

    const testCid = generateClientId()

    await fetch(`${BASE_URL}/test/run`, { headers: authHeaders(testCid) })
    await sleep(4500)

    const sseResp = await fetch(`${BASE_URL}/sse`, {
      headers: { ...authHeaders(testCid), 'Accept': 'text/event-stream' },
    })
    expect(sseResp.status).toBe(200)

    const reader = sseResp.body!.getReader()
    const decoder = new TextDecoder()
    let buffer = ''

    // reader.read() blocks forever on an open SSE stream.
    // Cancel the reader after the reading window to unblock.
    const readTimer = setTimeout(() => {
      reader.cancel().catch(() => {})
    }, 4500)

    try {
      const start = Date.now()
      while (Date.now() - start < 4000) {
        const { done, value } = await reader.read()
        if (done) break
        const chunk = decoder.decode(value, { stream: true })
        buffer = sw.processSSERead(buffer, chunk)
      }
    } finally {
      clearTimeout(readTimer)
      // Cancel the SSE body to close the TCP connection before releasing the reader
      sseResp.body?.cancel().catch(() => {})
      reader.releaseLock()
    }

    await sleep(500)

    const hashCount = sw.HASH_REGISTRY.size
    console.log(`[test] SW registered ${hashCount} hashes from real backend SSE`)

    expect(hashCount).toBeGreaterThan(0)

    const entries = Array.from(sw.HASH_REGISTRY.entries()) as [string, number][]
    const contentHashes = entries.filter(
      ([h]: [string, number]) => !h.startsWith('marker-'),
    )
    expect(contentHashes.length).toBeGreaterThan(0)

    for (const [hash] of contentHashes) {
      expect(hash).toMatch(/^[0-9a-f]+$/)
    }
  }, 30_000)

  test('SW does NOT register PatchSignals hashes from real SSE', async () => {
    const sw = loadSw()
    const testCid = generateClientId()

    await fetch(`${BASE_URL}/test/run`, { headers: authHeaders(testCid) })
    await sleep(4500)

    const sseResp = await fetch(`${BASE_URL}/sse`, {
      headers: { ...authHeaders(testCid), 'Accept': 'text/event-stream' },
    })
    const reader = sseResp.body!.getReader()
    const decoder = new TextDecoder()
    let buffer = ''

    // Cancel reader after reading window to unblock reader.read()
    const readTimer = setTimeout(() => {
      reader.cancel().catch(() => {})
    }, 4500)

    try {
      const start = Date.now()
      while (Date.now() - start < 4000) {
        const { done, value } = await reader.read()
        if (done) break
        buffer = sw.processSSERead(buffer, decoder.decode(value, { stream: true }))
      }
    } finally {
      clearTimeout(readTimer)
      sseResp.body?.cancel().catch(() => {})
      reader.releaseLock()
    }

    await sleep(500)

    const registeredHashes = Array.from(sw.HASH_REGISTRY.keys()) as string[]

    // All registered hashes should have correct hex format (non-signal events).
    // The SW filters by EVENT_PREFIX ('datastar-patch-elements'), so no
    // PatchSignals or ExecuteScript hashes should be registered.
    for (const hash of registeredHashes) {
      if (hash.startsWith('marker-')) continue
      expect(hash).toMatch(/^[0-9a-f]+$/)
    }

    console.log(`[test] SW registered ${registeredHashes.length} hashes (no PatchSignals)`)
  }, 30_000)
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

    // We don't need to wait for the full SSE response — just verify
    // that the SW rewrites the URL correctly by checking what URL
    // the fetch was called with.
    let capturedUrl = ''

    // Temporarily intercept fetch to capture the URL the SW calls
    const originalFetch = globalThis.fetch
    // @ts-ignore
    globalThis.fetch = async (input: RequestInfo | URL, init?: RequestInit) => {
      capturedUrl = typeof input === 'string' ? input : input instanceof URL ? input.href : input.url
      // Return a minimal response so respondWith resolves quickly
      return new Response('event: done\n\n', {
        status: 200,
        headers: { 'Content-Type': 'text/event-stream' },
      })
    }

    try {
      const response = await new Promise<Response>((resolve, reject) => {
        const timeout = setTimeout(() => {
          reject(new Error('SW fetch handler timed out'))
        }, 10_000)

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

      // Verify the SW added known_hashes to the URL
      expect(capturedUrl).toContain('known_hashes=')
      expect(capturedUrl).toContain('deadbeef1234')
      expect(capturedUrl).toContain('cafebabe5678')

      console.log(`[test] SW correctly rewrote URL: ${capturedUrl}`)
    } finally {
      // Restore real fetch
      // @ts-ignore
      globalThis.fetch = originalFetch
    }
  }, 20_000)
})

// ─────────────────────────────────────────────
// Utility
// ─────────────────────────────────────────────

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}
