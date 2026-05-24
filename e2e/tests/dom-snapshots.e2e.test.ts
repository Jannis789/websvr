/**
 * E2E Tests — Content-body exchange via real SSE + Backend
 *
 * Connects to the running Rust backend, triggers navigation endpoints,
 * collects SSE events, and verifies that the correct PatchElements events
 * with the correct selectors and HTML content are emitted.
 *
 * Also tests page-reload scenario: after collecting events, reconnects
 * with known_hashes to verify deduplication works (content is cached by SW).
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
// Helpers
// ─────────────────────────────────────────────

/**
 * Connect to SSE, trigger a navigation endpoint, and collect events.
 * The SSE connection is opened first, then the navigation request fires.
 * Returns all SSE events collected within the duration window.
 */
async function navigateAndCollect(
  cid: string,
  navPath: string,
  durationMs = 3_000,
): Promise<{ events: SseEvent[]; navStatus: number }> {
  // 1. Connect to SSE first (so we don't miss events)
  const sseResp = await fetch(`${BASE_URL}/sse`, {
    headers: { ...authHeaders(cid), Accept: 'text/event-stream' },
  })
  expect(sseResp.status).toBe(200)

  // 2. Trigger navigation endpoint
  const navResp = await fetch(`${BASE_URL}${navPath}`, {
    headers: authHeaders(cid),
  })

  // 3. Collect events
  const events = await collectSseEvents(sseResp, durationMs)
  return { events, navStatus: navResp.status }
}

/**
 * Extract PatchElements events from a list, optionally filtering by selector.
 */
function filterPatch(events: SseEvent[], selectorContains?: string): SseEvent[] {
  return events.filter((e) => {
    if (e.event !== 'datastar-patch-elements') return false
    if (selectorContains && !(e.data ?? '').includes(selectorContains)) return false
    return true
  })
}

/**
 * Parse data lines from a PatchElements event into structured fields.
 */
function parsePatch(event: SseEvent): {
  selector?: string
  elements: string
  mode?: string
} {
  const lines = (event.data ?? '').split('\n')
  let selector: string | undefined
  let elements: string[] = []
  let mode: string | undefined

  for (const line of lines) {
    if (line.startsWith('selector ')) {
      selector = line.slice('selector '.length).trim()
    } else if (line.startsWith('elements ')) {
      elements.push(line.slice('elements '.length))
    } else if (line.startsWith('mode ')) {
      mode = line.slice('mode '.length).trim()
    }
  }

  return { selector, elements: elements.join('\n'), mode }
}

/**
 * Get the LAST PatchElements event matching a selector — useful when
 * multiple events target the same slot (e.g. sequential navigations
 * on the same SSE connection).
 */
function lastPatchFor(events: SseEvent[], selectorContains: string): SseEvent | undefined {
  const matching = filterPatch(events, selectorContains)
  return matching.length > 0 ? matching[matching.length - 1] : undefined
}

// ─────────────────────────────────────────────
// Tests — Home page shell push
// ─────────────────────────────────────────────

describe('E2E Navigation — Home page initial load', () => {
  test('GET /home pushes all shell components via SSE', async () => {
    const cid = generateClientId()
    // /home returns the shell HTML (200) and pushes components via SSE
    const { events, navStatus } = await navigateAndCollect(cid, '/home')
    expect(navStatus).toBe(200)

    const patchEvents = filterPatch(events)
    expect(patchEvents.length).toBeGreaterThanOrEqual(5)

    // Verify each component slot was patched
    const selectors = patchEvents.map((e) => parsePatch(e).selector)

    expect(selectors).toContain('#sidebar-header')
    expect(selectors).toContain('#sidebar-menu')
    expect(selectors).toContain('#sidebar-footer')
    expect(selectors).toContain('#main-header')
    expect(selectors).toContain('#content-body')

    // Verify content-body has overview content
    const contentPatch = lastPatchFor(events, '#content-body')
    expect(contentPatch).toBeDefined()

    const { elements } = parsePatch(contentPatch!)
    expect(elements).toContain('OVERVIEW')

    console.log(
      `[e2e] /home: ${patchEvents.length} PatchElements events, selectors: ${selectors.join(', ')}`,
    )
  }, 10_000)
})

// ─────────────────────────────────────────────
// Tests — Content-body navigation
// ─────────────────────────────────────────────

describe('E2E Navigation — Content-body swap', () => {
  test('GET /home/movies pushes movies content to #content-body', async () => {
    const cid = generateClientId()
    const { events, navStatus } = await navigateAndCollect(cid, '/home/movies')
    // Navigate handlers return 303 (content pushed via SSE, not in HTTP response)
    expect(navStatus).toBe(303)

    const contentPatch = lastPatchFor(events, '#content-body')
    expect(contentPatch).toBeDefined()

    const { selector, elements } = parsePatch(contentPatch!)
    expect(selector).toBe('#content-body')
    expect(elements).toContain('MOVIES')

    console.log(`[e2e] /home/movies: content-body patched with movies HTML`)
  }, 10_000)

  test('GET /home/series pushes series content to #content-body', async () => {
    const cid = generateClientId()
    const { events, navStatus } = await navigateAndCollect(cid, '/home/series')
    expect(navStatus).toBe(303)

    const contentPatch = lastPatchFor(events, '#content-body')
    expect(contentPatch).toBeDefined()

    const { selector, elements } = parsePatch(contentPatch!)
    expect(selector).toBe('#content-body')
    expect(elements).toContain('SERIES')

    console.log(`[e2e] /home/series: content-body patched with series HTML`)
  }, 10_000)

  test('GET /home/overview pushes overview content to #content-body', async () => {
    const cid = generateClientId()
    const { events, navStatus } = await navigateAndCollect(cid, '/home/overview')
    expect(navStatus).toBe(303)

    const contentPatch = lastPatchFor(events, '#content-body')
    expect(contentPatch).toBeDefined()

    const { selector, elements } = parsePatch(contentPatch!)
    expect(selector).toBe('#content-body')
    expect(elements).toContain('OVERVIEW')

    console.log(`[e2e] /home/overview: content-body patched with overview HTML`)
  }, 10_000)
})

// ─────────────────────────────────────────────
// Tests — Sequential content swaps
// ─────────────────────────────────────────────

describe('E2E Navigation — Sequential content swaps', () => {
  test('navigating overview → movies → series sends correct content each time', async () => {
    const cid = generateClientId()

    // Step 1: overview (uses same SSE connection)
    const { events: ev1 } = await navigateAndCollect(cid, '/home/overview')
    const overviewPatch = lastPatchFor(ev1, '#content-body')
    expect(overviewPatch).toBeDefined()
    expect(parsePatch(overviewPatch!).elements).toContain('OVERVIEW')

    // Step 2: movies — new SSE connection, new events
    const { events: ev2 } = await navigateAndCollect(cid, '/home/movies')
    const moviesPatch = lastPatchFor(ev2, '#content-body')
    expect(moviesPatch).toBeDefined()
    const moviesElements = parsePatch(moviesPatch!).elements
    expect(moviesElements).toContain('MOVIES')
    // Movies page has no card-grid

    // Step 3: series — new SSE connection
    const { events: ev3 } = await navigateAndCollect(cid, '/home/series')
    const seriesPatch = lastPatchFor(ev3, '#content-body')
    expect(seriesPatch).toBeDefined()
    const seriesElements = parsePatch(seriesPatch!).elements
    expect(seriesElements).toContain('SERIES')
    // Series page is distinct from movies
    expect(seriesElements).not.toContain('MOVIES')

    console.log(`[e2e] Sequential navigation: all 3 content swaps verified`)
  }, 15_000)
})

// ─────────────────────────────────────────────
// Tests — Page reload (known_hashes dedup)
// ─────────────────────────────────────────────

describe('E2E Navigation — Page reload with deduplication', () => {
  test('after reload, known_hashes prevents re-sending already-seen content', async () => {
    const cid = generateClientId()

    // First visit: load home (gets all shell components)
    const { events: ev1, navStatus } = await navigateAndCollect(cid, '/home')
    expect(navStatus).toBe(200)

    const patchIds1 = filterPatch(ev1)
      .map((e) => e.id)
      .filter((id): id is string => !!id)

    expect(patchIds1.length).toBeGreaterThanOrEqual(5)

    // Take all event IDs as "known"
    const knownHashes = patchIds1.join(',')
    console.log(`[e2e] First visit: ${patchIds1.length} events, hashes collected`)

    // Simulate page reload: reconnect with known_hashes
    // The buffered events should be skipped since we already know them
    const sseResp2 = await fetch(
      `${BASE_URL}/sse?known_hashes=${knownHashes}`,
      {
        headers: { ...authHeaders(cid), Accept: 'text/event-stream' },
      },
    )
    expect(sseResp2.status).toBe(200)

    // Collect for a short time — should get NO duplicate patch events
    const events2 = await collectSseEvents(sseResp2, 2_000)

    const patchIds2 = filterPatch(events2)
      .map((e) => e.id)
      .filter((id): id is string => !!id)

    // Verify none of the known hashes leaked through
    const knownSet = new Set(patchIds1)
    const leaked = patchIds2.filter((h) => knownSet.has(h))
    expect(leaked.length).toBe(0)

    console.log(
      `[e2e] Reload: ${patchIds2.length} events (${leaked.length} leaked)`,
    )
  }, 15_000)

  test('after reload + navigation, new content arrives even with known_hashes', async () => {
    const cid = generateClientId()

    // First visit: load overview
    const { events: ev1 } = await navigateAndCollect(cid, '/home/overview')
    const patchIds1 = new Set(
      filterPatch(ev1)
        .map((e) => e.id)
        .filter((id): id is string => !!id),
    )

    // Reconnect with known_hashes, then navigate to movies
    const knownHashes = Array.from(patchIds1).join(',')
    const sseResp2 = await fetch(
      `${BASE_URL}/sse?known_hashes=${knownHashes}`,
      {
        headers: { ...authHeaders(cid), Accept: 'text/event-stream' },
      },
    )

    // Wait for initial replay to settle, then navigate to movies
    await new Promise((r) => setTimeout(r, 500))

    const navResp = await fetch(`${BASE_URL}/home/movies`, {
      headers: authHeaders(cid),
    })
    expect(navResp.status).toBe(303)

    // Collect events from the SSE connection
    const events2 = await collectSseEvents(sseResp2, 3_000)

    // Should have received the movies content
    const moviesPatch = lastPatchFor(events2, '#content-body')
    expect(moviesPatch).toBeDefined()
    expect(parsePatch(moviesPatch!).elements).toContain('MOVIES')

    // The movies patch should have a NEW hash (not one we already knew)
    if (moviesPatch!.id) {
      expect(patchIds1.has(moviesPatch!.id)).toBe(false)
    }

    console.log(`[e2e] Reload+navigate: movies content received, no duplicates`)
  }, 15_000)
})

// ─────────────────────────────────────────────
// Tests — Force reload (reconnect + navigate)
// ─────────────────────────────────────────────

describe('E2E Navigation — Force reload scenario', () => {
  test('full page reload cycle: initial load → abort SSE → reconnect → navigate works', async () => {
    const cid = generateClientId()

    // === Phase 1: Initial page load ===
    const sseResp1 = await fetch(`${BASE_URL}/sse`, {
      headers: { ...authHeaders(cid), Accept: 'text/event-stream' },
    })
    expect(sseResp1.status).toBe(200)

    // Trigger /home to populate shell
    const homeResp = await fetch(`${BASE_URL}/home`, {
      headers: authHeaders(cid),
    })
    expect(homeResp.status).toBe(200)

    // Collect initial events
    const events1 = await collectSseEvents(sseResp1, 2_000)
    const patches1 = filterPatch(events1)
    expect(patches1.length).toBeGreaterThanOrEqual(5)

    // Extract content-body hash from initial load
    const initialContentPatch = lastPatchFor(events1, '#content-body')
    expect(initialContentPatch).toBeDefined()
    expect(initialContentPatch!.id).toBeTruthy()

    console.log(`[e2e] Phase 1: ${patches1.length} events, content-body hash=${initialContentPatch!.id}`)

    // Collect ALL known hashes (simulates what the SW would store)
    const knownHashes = patches1
      .map((e) => e.id)
      .filter((id): id is string => !!id)
      .join(',')

    // === Phase 2+3+4: Reconnect with known_hashes, then navigate ===
    // We open a new SSE connection and collect events throughout:
    //   - First: buffered replay (should be empty due to known_hashes)
    //   - Then: navigate to movies → collect movies event
    //   - Then: navigate to series → collect series event
    const sseResp2 = await fetch(
      `${BASE_URL}/sse?known_hashes=${encodeURIComponent(knownHashes)}`,
      {
        headers: { ...authHeaders(cid), Accept: 'text/event-stream' },
      },
    )
    expect(sseResp2.status).toBe(200)

    // Start collecting from the new SSE stream with a generous timeout
    // We trigger navigations DURING collection using concurrent promises
    const collectAll = async (): Promise<SseEvent[]> => {
      const controller = new AbortController()
      const events: SseEvent[] = []

      const collect = async () => {
        try {
          for await (const event of readSseStream(sseResp2.body!, controller.signal)) {
            events.push(event)
          }
        } catch {
          // Stream aborted — expected
        }
      }

      const navigations = async () => {
        // Wait for replay to settle
        await new Promise((r) => setTimeout(r, 1_500))

        // Phase 3: Navigate to movies
        const navResp1 = await fetch(`${BASE_URL}/home/movies`, {
          headers: authHeaders(cid),
        })
        expect(navResp1.status).toBe(303)

        // Wait for movies event to arrive
        await new Promise((r) => setTimeout(r, 2_000))

        // Phase 4: Navigate to series
        const navResp2 = await fetch(`${BASE_URL}/home/series`, {
          headers: authHeaders(cid),
        })
        expect(navResp2.status).toBe(303)

        // Wait for series event to arrive
        await new Promise((r) => setTimeout(r, 2_000))

        // Done — abort the SSE stream
        controller.abort()
        if (!sseResp2.bodyUsed) {
          sseResp2.body?.cancel().catch(() => {})
        }
      }

      await Promise.race([
        Promise.all([collect(), navigations()]),
        new Promise((_, reject) => setTimeout(() => reject(new Error('timeout')), 20_000)),
      ])

      return events
    }

    const allEvents = await collectAll()

    // Check that no known hashes leaked through during replay
    const replayEvents = allEvents.slice(0, patches1.length) // early events are replay
    const replayLeaks = filterPatch(replayEvents)
      .map((e) => e.id)
      .filter((id): id is string => !!id)
      .filter((h) => new Set(knownHashes.split(',')).has(h))
    expect(replayLeaks.length).toBe(0)

    // Must have received movies content
    const moviesOnly = allEvents.filter(
      (e) =>
        e.event === 'datastar-patch-elements' &&
        (e.data ?? '').includes('#content-body') &&
        (e.data ?? '').includes('MOVIES'),
    )
    expect(moviesOnly.length).toBeGreaterThanOrEqual(1)

    const moviesParsed = parsePatch(moviesOnly[0])
    expect(moviesParsed.selector).toBe('#content-body')
    expect(moviesParsed.mode).toBe('inner')
    expect(moviesParsed.elements).toContain('MOVIES')

    // Must have received series content (proves #content-body is still targetable)
    const seriesOnly = allEvents.filter(
      (e) =>
        e.event === 'datastar-patch-elements' &&
        (e.data ?? '').includes('#content-body') &&
        (e.data ?? '').includes('SERIES'),
    )
    expect(seriesOnly.length).toBeGreaterThanOrEqual(1)

    const seriesParsed = parsePatch(seriesOnly[0])
    expect(seriesParsed.selector).toBe('#content-body')
    expect(seriesParsed.mode).toBe('inner')
    expect(seriesParsed.elements).toContain('SERIES')

    console.log(`[e2e] Phase 4 (second navigate): series patch received, content-body still targetable`)
  }, 25_000)
})

describe('E2E Navigation — SSE event structure', () => {
  test('PatchElements events have valid selector, elements, mode=inner, and HMAC hash ID', async () => {
    const cid = generateClientId()
    const { events } = await navigateAndCollect(cid, '/home/movies')

    const patches = filterPatch(events)
    expect(patches.length).toBeGreaterThan(0)

    for (const event of patches) {
      // Must have an event ID (HMAC hash)
      expect(event.id).toBeTruthy()
      expect(event.id).toMatch(/^[0-9a-f]+$/)

      const { selector, elements, mode } = parsePatch(event)

      // Must have a selector
      expect(selector).toBeTruthy()
      expect(selector).toMatch(/^#/)

      // Must have elements content
      expect(elements).toBeTruthy()
      expect(elements.length).toBeGreaterThan(0)

      // Mode must be "inner" — outer mode replaces the target element,
      // which breaks subsequent patches because the #id disappears from the DOM
      expect(mode).toBe('inner')
    }

    console.log(`[e2e] ${patches.length} events validated: all have selector, elements, mode=inner, hash ID`)
  }, 10_000)
})
