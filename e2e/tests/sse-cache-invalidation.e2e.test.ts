/**
 * SSE Cache Invalidation — TDD E2E Tests
 *
 * Bug: After login, stale login-form events remain in the SSE replay cache.
 * On page reload, these events are replayed, causing the login form to flash
 * before the home page renders.
 *
 * Fix: Clear the EventEmitter cache on state transitions (login/logout).
 */

import { describe, test, expect, beforeAll, afterAll } from 'vitest'
import {
  startBackend,
  stopBackend,
  generateClientId,
  authHeaders,
  collectSseEvents,
  type SseEvent,
} from './helpers/backend'

let BASE_URL: string
let cid: string
let headers: Record<string, string>

beforeAll(async () => {
  BASE_URL = await startBackend()
  cid = generateClientId()
  headers = await authHeaders(cid)
}, 30_000)

afterAll(() => {
  stopBackend()
})

/** Parse stats response text — avoid BigInt precision loss. */
function parseStats(text: string) {
  return {
    cached_count: Number(text.match(/"cached_count":\s*(\d+)/)?.[1] ?? 0),
    current_ver: Number(text.match(/"current_ver":\s*(\d+)/)?.[1] ?? 0),
    epoch: text.match(/"epoch":\s*(\d+)/)?.[1] ?? '0',
  }
}

/** Get current stats from /test/stats. */
async function getStats() {
  const resp = await fetch(`${BASE_URL}/test/stats`, { headers })
  return parseStats(await resp.text())
}

describe('SSE Cache Invalidation — Login clears stale events', () => {
  test('After clear, stale pre-login events are NOT in cache', async () => {
    // ── Step 1: Emit pre-login events via /test/run ──
    // Connect SSE first so events have a subscriber
    const sse1 = await fetch(`${BASE_URL}/sse`, {
      headers: { ...headers, Accept: 'text/event-stream' },
    })
    // Drain initial
    await collectSseEvents(sse1, 500)

    // /test/run emits 6 Phase A + B events into the cache
    await fetch(`${BASE_URL}/test/run`, { headers })
    await new Promise(r => setTimeout(r, 500))

    const statsBefore = await getStats()
    console.log(`[step1] After /test/run: cached=${statsBefore.cached_count}`)
    expect(statsBefore.cached_count).toBeGreaterThan(0)

    // ── Step 2: Full reload (ver=0) → should replay ALL cached events ──
    const sse2 = await fetch(`${BASE_URL}/sse`, {
      headers: { ...headers, Accept: 'text/event-stream' },
    })
    const fullReplay = await collectSseEvents(sse2, 2_000)

    // Count Phase A/B events (stale login-like events)
    const phaseEventsBefore = fullReplay.filter(e =>
      e.data?.includes('data-phase') || e.data?.includes("data-marker='phase-")
    )
    console.log(`[step2] Full replay before clear: ${phaseEventsBefore.length} stale phase events`)
    expect(phaseEventsBefore.length).toBeGreaterThan(0) // Bug exists: stale events present

    // ── Step 3: Trigger "login" — clear cache, then emit post-login event ──
    // We need /test/clear endpoint to simulate cache clear
    // (In real flow, login success calls ctx.event_emitter.clear())
    await fetch(`${BASE_URL}/test/clear`, { headers })
    await new Promise(r => setTimeout(r, 200))

    // Emit post-login navigation event
    await fetch(`${BASE_URL}/test/1`, { headers })
    await new Promise(r => setTimeout(r, 200))

    const statsAfter = await getStats()
    console.log(`[step3] After clear + /test/1: cached=${statsAfter.cached_count}`)

    // ── Step 4: Full reload again → stale events should be GONE ──
    const sse3 = await fetch(`${BASE_URL}/sse`, {
      headers: { ...headers, Accept: 'text/event-stream' },
    })
    const replayAfter = await collectSseEvents(sse3, 2_000)

    const phaseEventsAfter = replayAfter.filter(e =>
      e.data?.includes('data-phase') || e.data?.includes("data-marker='phase-")
    )
    console.log(`[step4] Full replay after clear: ${phaseEventsAfter.length} stale phase events`)

    // THE KEY ASSERTION: After cache clear, no stale Phase A/B events should be replayed
    expect(phaseEventsAfter.length).toBe(0)

    // Only the post-login event should be in cache
    expect(statsAfter.cached_count).toBe(1)
  })
})
