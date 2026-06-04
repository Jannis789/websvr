/**
 * SSE Communication — Snapshot-based E2E Tests
 *
 * TDD approach: Tests define the expected SSE wire format behavior.
 * Live events carry `id:` (patch_ver) so the client can track its version.
 * Replay events (after reload) also carry `id:`.
 * On reconnect, already-known events are sent as id_only (just `id: <ver>`, no data).
 *
 * Flow:
 *   Phase 1: /test/run → live events → snapshot (no id)
 *   Phase 2: Navigate click → live patch → snapshot (no id)
 *   Phase 3: Reload (reconnect) → replay events → snapshot (id set)
 *   Phase 4: DOM verification (happy-dom applies patches)
 *   Phase 5: Post-reload click → live event without id
 *   Memory: 10 reload cycles → cache stays bounded
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

// ──── Shared state across phases ────

let BASE_URL: string
let cid: string
let headers: Record<string, string>

// Phase results
let liveEventsPhase1: SseEvent[]
let liveEventsPhase2: SseEvent[]
let replayEventsPhase3: SseEvent[]
let liveEventsPhase5: SseEvent[]

// ──── Helpers ────

function filterPatch(events: SseEvent[]): SseEvent[] {
  return events.filter((e) => e.event === 'datastar-patch-elements')
}

function parsePatch(event: SseEvent) {
  const lines = (event.data ?? '').split('\n')
  let selector: string | undefined
  let elements: string[] = []
  let mode: string | undefined
  for (const line of lines) {
    if (line.startsWith('selector ')) selector = line.slice(9).trim()
    else if (line.startsWith('elements ')) elements.push(line.slice(9))
    else if (line.startsWith('mode ')) mode = line.slice(5).trim()
  }
  return { selector, elements: elements.join('\n'), mode }
}

/** Build snapshot-friendly structure from raw SSE events. */
function toSnapshot(events: SseEvent[]) {
  return filterPatch(events).map((e) => {
    const { selector, elements, mode } = parsePatch(e)
    return {
      event: e.event,
      id: e.id ?? null,
      selector,
      mode,
      elements_preview: elements.substring(0, 120),
    }
  })
}

/** Connect to SSE stream with auth headers. */
async function connectSse(): Promise<Response> {
  const resp = await fetch(`${BASE_URL}/sse`, {
    headers: { ...headers, Accept: 'text/event-stream' },
  })
  expect(resp.status).toBe(200)
  return resp
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
// Phase 1: Initial load — live events have NO id
// ════════════════════════════════════════════════════════════════

describe('SSE Communication — Phase 1: Live events (with id)', () => {
  test('/test/run pushes PatchElements WITH id field', async () => {
    const sse = await connectSse()

    // Trigger backend test action
    const runResp = await fetch(`${BASE_URL}/test/run`, { headers })
    expect(runResp.status).toBe(204)

    liveEventsPhase1 = await collectSseEvents(sse, 3_000)
    const patches = filterPatch(liveEventsPhase1)
    expect(patches.length).toBeGreaterThan(0)

    // Snapshot the event structure
    expect(toSnapshot(liveEventsPhase1)).toMatchSnapshot('phase-1-live-events')

    // Every live event must have id (patch_ver for version tracking)
    for (const e of patches) {
      expect(e.id).toBeDefined()
    }

    console.log(`[snapshot] Phase 1: ${patches.length} live events, all with id`)
  }, 10_000)
})

// ════════════════════════════════════════════════════════════════
// Phase 2: Click (navigate) — still live, no id
// ════════════════════════════════════════════════════════════════

describe('SSE Communication — Phase 2: Click (live, with id)', () => {
  test('Navigate to /home/movies pushes patch WITH id', async () => {
    const sse = await connectSse()

    // Wait for replay to settle first (Phase 1 events are cached)
    await new Promise((r) => setTimeout(r, 500))

    // "Click" = navigate
    const navResp = await fetch(`${BASE_URL}/home/movies`, { headers })
    expect(navResp.status).toBe(200)

    liveEventsPhase2 = await collectSseEvents(sse, 3_000)
    const patches = filterPatch(liveEventsPhase2)
    expect(patches.length).toBeGreaterThan(0)

    // Snapshot
    expect(toSnapshot(liveEventsPhase2)).toMatchSnapshot('phase-2-click-live')

    // Only check the LIVE click events (movies content) — replay events have id
    const moviesPatches = patches.filter(
      (e) => (e.data ?? '').includes('content_movies'),
    )
    expect(moviesPatches.length).toBeGreaterThan(0)

    // Live click events must have id (patch_ver for version tracking)
    for (const e of moviesPatches) {
      expect(e.id).toBeDefined()
    }

    // Verify content: movies content-slot patch
    const contentPatch = moviesPatches[0]
    expect(parsePatch(contentPatch).elements).toContain('content_movies')

    console.log(`[snapshot] Phase 2: ${moviesPatches.length} live movies events, all with id`)
  }, 10_000)
})

// ════════════════════════════════════════════════════════════════
// Phase 3: Reload → replay events WITH id
// ════════════════════════════════════════════════════════════════

describe('SSE Communication — Phase 3: Reload (replay with id)', () => {
  test('Reconnect replays cached events WITH id field', async () => {
    // Reconnect without ?v= or ?e= → full replay
    const sse = await connectSse()

    replayEventsPhase3 = await collectSseEvents(sse, 3_000)
    const patches = filterPatch(replayEventsPhase3)
    expect(patches.length).toBeGreaterThan(0)

    // Snapshot
    expect(toSnapshot(replayEventsPhase3)).toMatchSnapshot('phase-3-replay-events')

    // Every replay event must have an id (numeric)
    for (const e of patches) {
      expect(e.id).toBeDefined()
      expect(e.id).toMatch(/^\d+$/)
    }

    // Content comparison: replay selectors cover live selectors
    const liveSelectors = [
      ...filterPatch(liveEventsPhase1),
      ...filterPatch(liveEventsPhase2),
    ]
      .map((e) => parsePatch(e).selector)
      .filter(Boolean)

    const replaySelectors = patches
      .map((e) => parsePatch(e).selector)
      .filter(Boolean)

    for (const sel of new Set(liveSelectors)) {
      expect(replaySelectors).toContain(sel)
    }

    console.log(
      `[snapshot] Phase 3: ${patches.length} replay events, all with id`,
    )
  }, 10_000)

  test('Replay content matches live content (data matches)', async () => {
    // Take the last content-slot patch from live Phase 2
    const liveContentPatches = filterPatch(liveEventsPhase2).filter(
      (e) => (e.data ?? '').includes('#content-slot'),
    )
    const replayContentPatches = filterPatch(replayEventsPhase3).filter(
      (e) => (e.data ?? '').includes('#content-slot'),
    )

    // The LAST replay content-slot should contain the same HTML as the live one
    if (liveContentPatches.length > 0 && replayContentPatches.length > 0) {
      const liveElements = parsePatch(
        liveContentPatches[liveContentPatches.length - 1],
      ).elements
      const replayElements = parsePatch(
        replayContentPatches[replayContentPatches.length - 1],
      ).elements
      expect(replayElements).toBe(liveElements)
    }
  })
})

// ════════════════════════════════════════════════════════════════
// Phase 4: DOM verification (happy-dom)
// ════════════════════════════════════════════════════════════════

describe('SSE Communication — Phase 4: DOM content', () => {
  test('Replay patches produce correct DOM state', async () => {
    // Dynamic import of happy-dom (devDependency)
    const { Window } = await import('happy-dom')

    const window = new Window({ url: 'http://localhost:3000/test' })
    const document = window.document

    // Set up shell structure (matches shell.html)
    document.body.innerHTML = `
      <div class="app-layout">
        <div id="sidebar-slot"></div>
        <main class="main-container">
          <div id="header-slot"></div>
          <div id="content-slot"></div>
        </main>
      </div>
    `

    // Apply replay patches (simulate Datastar PatchElements)
    const patches = filterPatch(replayEventsPhase3)
    for (const event of patches) {
      const { selector, elements, mode } = parsePatch(event)
      if (!selector) continue

      const target = document.querySelector(selector)
      if (!target) continue

      if (mode === 'inner') {
        target.innerHTML = elements
      } else {
        // outer: replace element entirely
        const temp = document.createElement('div')
        temp.innerHTML = elements
        const replacement = temp.firstElementChild
        if (replacement) target.replaceWith(replacement)
      }
    }

    // Verify content-slot still exists (inner mode preserves the container)
    const contentSlot = document.querySelector('#content-slot')
    expect(contentSlot).not.toBeNull()

    // Content-slot should be populated with the last patch's content
    const innerHtml = contentSlot!.innerHTML.trim()
    expect(innerHtml.length).toBeGreaterThan(0)

    // Snapshot DOM state
    const domState = {
      content_slot_html: innerHtml,
    }
    expect(domState).toMatchSnapshot('phase-4-dom-state')

    console.log(`[snapshot] Phase 4: DOM content-slot = "${innerHtml}"`)
  }, 10_000)
})

// ════════════════════════════════════════════════════════════════
// Phase 5: Post-reload click → still no id (live)
// ════════════════════════════════════════════════════════════════

describe('SSE Communication — Phase 5: Post-reload click', () => {
  test('After reload, new live clicks still have id', async () => {
    const sse = await connectSse()

    // Wait for replay to settle
    await new Promise((r) => setTimeout(r, 500))

    // Click → navigate to series
    const navResp = await fetch(`${BASE_URL}/home/series`, { headers })
    expect(navResp.status).toBe(200)

    liveEventsPhase5 = await collectSseEvents(sse, 3_000)

    // Filter: series content is the live click result
    const seriesPatches = filterPatch(liveEventsPhase5).filter(
      (e) => (e.data ?? '').includes('content_series'),
    )
    expect(seriesPatches.length).toBeGreaterThan(0)

    // Live click events must have id (patch_ver)
    for (const e of seriesPatches) {
      expect(e.id).toBeDefined()
    }

    // Content is correct
    expect(parsePatch(seriesPatches[0]).elements).toContain('content_series')

    console.log(
      `[snapshot] Phase 5: ${seriesPatches.length} live series events, all with id`,
    )
  }, 10_000)
})

// ════════════════════════════════════════════════════════════════
// Memory Leak: Cache bounded after 10 reload cycles
// ════════════════════════════════════════════════════════════════

describe('Memory Leak — Reload cycles', () => {
  test('Cache stays bounded after 10 reload cycles', async () => {
    const leakCid = generateClientId()
    const leakHeaders = await authHeaders(leakCid)

    // Initial load: populate cache
    let sse = await fetch(`${BASE_URL}/sse`, {
      headers: { ...leakHeaders, Accept: 'text/event-stream' },
    })
    await fetch(`${BASE_URL}/home/movies`, { headers: leakHeaders })
    await collectSseEvents(sse, 2_000)

    // Check initial stats
    const statsResp0 = await fetch(`${BASE_URL}/test/stats`, {
      headers: leakHeaders,
    })
    const stats0 = (await statsResp0.json()) as {
      cached_count: number
      current_ver: number
    }
    console.log(`[memory] Initial: cached=${stats0.cached_count}, ver=${stats0.current_ver}`)

    // 10 reload cycles
    const cachedCounts: number[] = [stats0.cached_count]
    for (let i = 0; i < 10; i++) {
      sse = await fetch(`${BASE_URL}/sse`, {
        headers: { ...leakHeaders, Accept: 'text/event-stream' },
      })
      await collectSseEvents(sse, 2_000)

      const resp = await fetch(`${BASE_URL}/test/stats`, { headers: leakHeaders })
      const stats = (await resp.json()) as {
        cached_count: number
        current_ver: number
      }
      cachedCounts.push(stats.cached_count)
    }

    // Cache is bounded (MAX_CACHE = 64)
    const maxCached = Math.max(...cachedCounts)
    expect(maxCached).toBeLessThanOrEqual(64)

    // Counts stabilize (no growth after initial populate)
    const last5 = cachedCounts.slice(-5)
    const stable = last5.every((c) => c === last5[0])
    expect(stable).toBe(true)

    console.log(`[memory] Cached counts across 11 loads: [${cachedCounts.join(', ')}]`)
  }, 40_000)
})
