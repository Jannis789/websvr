/**
 * SSE Replay Protocol — TDD E2E Tests
 *
 * Every event carries `id:` (its patch_ver) so the client can always
 * track its version for reconnects:
 *   Phase 1: Initial /sse connect → snapshot → assert empty
 *   Phase 2: Click → PatchElement → snapshot → assert HAS id
 *   Phase 3: Reload → id_only for known, live click HAS id
 */

import { describe, test, expect, beforeAll, afterAll } from 'vitest'
import {
  startBackend,
  stopBackend,
  generateClientId,
  authHeaders,
  readSseStream,
  collectSseEvents,
  type SseEvent,
} from './helpers/backend'

// ──── Shared state ────

let BASE_URL: string
let cid: string
let headers: Record<string, string>

// Phase snapshots
let phase1Events: SseEvent[]
let phase2Events: SseEvent[]
let phase3Events: SseEvent[]

// ──── Helpers ────

/** Convert a parsed SseEvent back to raw SSE wire format string. */
function eventToWire(e: SseEvent): string {
  const lines: string[] = []
  if (e.id !== undefined) lines.push(`id: ${e.id}`)
  if (e.event) lines.push(`event: ${e.event}`)
  if (e.data) lines.push(e.data)
  return lines.join('\n')
}

/** Convert events to raw SSE wire format (joined by double newlines). */
function eventsToWire(events: SseEvent[]): string {
  if (events.length === 0) return ''
  return events.map(eventToWire).join('\n\n')
}

/** Filter only patch-elements events. */
function filterPatch(events: SseEvent[]): SseEvent[] {
  return events.filter((e) => e.event === 'datastar-patch-elements')
}

/** Connect to SSE stream with auth headers. */
async function connectSse(): Promise<Response> {
  const resp = await fetch(`${BASE_URL}/sse`, {
    headers: { ...headers, Accept: 'text/event-stream' },
  })
  expect(resp.status).toBe(200)
  return resp
}

/** Parse raw SSE lines to find id-only events (just `id: <ver>`, no event/data). */
function findIdOnlyEvents(events: SseEvent[]): SseEvent[] {
  return events.filter((e) => e.id !== undefined && !e.event && !e.data)
}

/** Parse raw SSE lines to find full events (have id + event/data). */
function findFullEventsWithId(events: SseEvent[]): SseEvent[] {
  return events.filter((e) => e.id !== undefined && !!e.event)
}

// ──── Lifecycle ────

beforeAll(async () => {
  BASE_URL = await startBackend()
  cid = generateClientId()
  headers = await authHeaders(cid)
}, 30_000)

afterAll(() => {
  stopBackend()
})

// ════════════════════════════════════════════════════════════════
// Phase 1: Initial connect — empty cache
// ════════════════════════════════════════════════════════════════

describe('SSE Replay Protocol — Phase 1: Initial connect', () => {
  test('/sse stream is empty initially (no cached events)', async () => {
    const sse = await connectSse()

    // Collect for a short window — nothing should arrive since cache is empty
    phase1Events = await collectSseEvents(sse, 1_500)

    // No patch-elements events in initial stream (empty cache)
    const patches = filterPatch(phase1Events)
    expect(patches.length).toBe(0)

    // Snapshot: empty initial stream
    expect(eventsToWire(phase1Events)).toMatchSnapshot('phase-1-initial-connect')

    console.log(`[phase1] Initial connect: ${phase1Events.length} events, ${patches.length} patches (expected 0)`)
  }, 10_000)
})

// ════════════════════════════════════════════════════════════════
// Phase 2: Click → PatchElement with id
// ════════════════════════════════════════════════════════════════

describe('SSE Replay Protocol — Phase 2: Click sends live PatchElement (with id)', () => {
  test('GET /test/1 sends PatchElement WITH id in SSE stream', async () => {
    const sse = await connectSse()

    // Wait for any replay to settle
    await new Promise((r) => setTimeout(r, 500))

    // "Click" = GET /test/1 — this should push a PatchElement through the SSE stream
    const clickResp = await fetch(`${BASE_URL}/test/1`, { headers })
    // Handler returns 204 No Content — events flow through SSE stream
    expect(clickResp.status).toBe(204)

    phase2Events = await collectSseEvents(sse, 2_000)
    const patches = filterPatch(phase2Events)
    expect(patches.length).toBeGreaterThan(0)

    // SNAPSHOT — raw SSE wire format
    expect(eventsToWire(phase2Events)).toMatchSnapshot('phase-2-click-live')

    // ASSERTION: Live events must have id (patch_ver) for client version tracking
    for (const e of patches) {
      expect(e.id).toBeDefined()
    }

    // Verify the patch contains the expected marker
    const lastPatch = patches[patches.length - 1]
    expect(lastPatch.data).toContain('test-1')

    console.log(`[phase2] Click: ${patches.length} live patches, all with id`)
  }, 10_000)
})

// ════════════════════════════════════════════════════════════════
// Phase 3: Reload → server sends id_only for known events
// ════════════════════════════════════════════════════════════════

describe('SSE Replay Protocol — Phase 3: Reload (id_only replay)', () => {
  test('Reconnect with ?v=N sends id_only for known events', async () => {
    // Get current server version from /test/stats
    const statsResp = await fetch(`${BASE_URL}/test/stats`, { headers })
    const statsText = await statsResp.text()
    // Parse manually to avoid BigInt precision loss — epoch exceeds Number.MAX_SAFE_INTEGER
    const cachedCount = Number(statsText.match(/"cached_count":\s*(\d+)/)?.[1] ?? 0)
    const currentVer = Number(statsText.match(/"current_ver":\s*(\d+)/)?.[1] ?? 0)
    const epochStr = statsText.match(/"epoch":\s*(\d+)/)?.[1] ?? '0'
    expect(cachedCount).toBeGreaterThan(0)

    // Reconnect WITH client version — server should send id_only for known events
    // Use v = currentVer - 1 so the single cached event is "known" (ver <= client_ver)
    const sse = await fetch(`${BASE_URL}/sse?v=${currentVer - 1}&e=${epochStr}`, {
      headers: { ...headers, Accept: 'text/event-stream' },
    })
    expect(sse.status).toBe(200)

    phase3Events = await collectSseEvents(sse, 2_000)

    // Parse ALL events including id-only ones
    // id-only events: have id but no event/data fields
    const idOnlyEvents = findIdOnlyEvents(phase3Events)
    const fullEventsWithId = findFullEventsWithId(phase3Events)

    // ASSERTION: For events the client already has (ver <= client_ver),
    // server should send ONLY `id: <ver>` — no event type, no data
    expect(idOnlyEvents.length).toBeGreaterThan(0)

    // SNAPSHOT — raw SSE wire format
    expect(eventsToWire(phase3Events)).toMatchSnapshot('phase-3-reload-id-only')

    console.log(`[phase3] Reload: ${idOnlyEvents.length} id-only, ${fullEventsWithId.length} full-with-id`)
  }, 10_000)

  test('After reload, click still sends live event WITH id', async () => {
    // Fresh connect without version — gets full replay
    const sse = await connectSse()
    await new Promise((r) => setTimeout(r, 500))

    // Click again
    const clickResp = await fetch(`${BASE_URL}/test/1`, { headers })
    expect(clickResp.status).toBe(204)

    const postReloadEvents = await collectSseEvents(sse, 2_000)
    const patches = filterPatch(postReloadEvents)

    // Find the NEW live patch (the click response) — it should have id
    const testPatches = patches.filter(
      (e) => (e.data ?? '').includes('test-1')
    )
    expect(testPatches.length).toBeGreaterThan(0)

    // Live click events must have id (patch_ver)
    for (const e of testPatches) {
      expect(e.id).toBeDefined()
    }

    console.log(`[phase3-post] Post-reload click: ${testPatches.length} live patches, all with id`)
  }, 10_000)
})
