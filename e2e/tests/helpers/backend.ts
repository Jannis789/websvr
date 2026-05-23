/**
 * Backend lifecycle management for integration tests.
 *
 * Starts the Rust backend server as a child process, waits for it
 * to become ready, and provides utilities to stop it after tests.
 */

import { spawn, type ChildProcess } from 'node:child_process'
import path from 'node:path'
import os from 'node:os'
import fs from 'node:fs'

const BACKEND_PORT = 3000
const BASE_URL = `http://localhost:${BACKEND_PORT}`
const READY_TIMEOUT_MS = 15_000
const POLL_INTERVAL_MS = 300

let backendProcess: ChildProcess | null = null
let dbPath: string | null = null

/**
 * Start the backend server and wait until it's ready to accept connections.
 *
 * Uses the pre-built binary from target/debug/platform-backend.
 * Skips if the backend is already running (port 3000 is already in use).
 */
export async function startBackend(): Promise<string> {
  // If already running, skip
  if (await isBackendReady()) {
    console.log('[backend] Already running on port', BACKEND_PORT)
    return BASE_URL
  }

  const projectRoot = path.resolve(__dirname, '..', '..', '..')
  const binary = path.join(projectRoot, 'target', 'debug', 'platform-backend')

  // Use a temp file-based SQLite database (in-memory causes issues with connection pools)
  dbPath = path.join(os.tmpdir(), `platform-test-${Date.now()}.db`)
  console.log('[backend] Starting', binary, 'with DB', dbPath)

  backendProcess = spawn(binary, [], {
    cwd: projectRoot,
    env: {
      ...process.env,
      RUST_LOG: 'info',
      DATABASE_URL: `sqlite://${dbPath}?mode=rwc`,
      HMAC_SECRET: 'test-secret-123',
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  })

  // Pipe stdout/stderr so we can debug failures
  backendProcess.stdout?.on('data', (d: Buffer) => process.stdout.write(`[backend] ${d}`))
  backendProcess.stderr?.on('data', (d: Buffer) => process.stderr.write(`[backend:err] ${d}`))

  // Handle unexpected exit
  backendProcess.on('exit', (code, sig) => {
    console.log(`[backend] Process exited (code=${code}, signal=${sig})`)
    backendProcess = null
  })

  // Wait until ready
  const start = Date.now()
  while (Date.now() - start < READY_TIMEOUT_MS) {
    if (await isBackendReady()) {
      console.log('[backend] Ready!')
      return BASE_URL
    }
    await sleep(POLL_INTERVAL_MS)
  }

  throw new Error('Backend failed to start within timeout')
}

/**
 * Stop the backend process if we started it.
 */
export function stopBackend(): void {
  if (backendProcess) {
    console.log('[backend] Stopping ...')
    backendProcess.kill('SIGTERM')
    backendProcess = null
  }
  // Clean up temp database
  if (dbPath) {
    try {
      fs.unlinkSync(dbPath)
      try { fs.unlinkSync(dbPath + '-wal') } catch {}
      try { fs.unlinkSync(dbPath + '-shm') } catch {}
      try { fs.unlinkSync(dbPath + '-journal') } catch {}
      console.log('[backend] Cleaned up temp DB:', dbPath)
    } catch { /* ignore cleanup errors */ }
    dbPath = null
  }
}

/**
 * Check whether the backend is accepting connections.
 */
async function isBackendReady(): Promise<boolean> {
  try {
    const controller = new AbortController()
    const timer = setTimeout(() => controller.abort(), 500)
    const resp = await fetch(`${BASE_URL}/sw.js`, { signal: controller.signal })
    clearTimeout(timer)
    return resp.status === 200
  } catch {
    return false
  }
}

function sleep(ms: number): Promise<void> {
  return new Promise((r) => setTimeout(r, ms))
}

/**
 * Generate a valid UUID v4 for use as a platform_cid cookie.
 * (Simple v4 UUID generation — sufficient for tests.)
 */
export function generateClientId(): string {
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0
    const v = c === 'x' ? r : (r & 0x3) | 0x8
    return v.toString(16)
  })
}

/**
 * Create standard headers for requests to protected routes.
 */
export function authHeaders(clientId?: string): Record<string, string> {
  const cid = clientId ?? generateClientId()
  return {
    Cookie: `platform_cid=${cid}`,
    'Accept': 'text/event-stream, text/html',
  }
}

/**
 * Read an SSE response body line by line, collecting events.
 * Returns an array of parsed SSE events.
 */
export interface SseEvent {
  event?: string
  id?: string
  data?: string
}

export async function* readSseStream(
  body: ReadableStream<Uint8Array>,
  signal?: AbortSignal,
): AsyncGenerator<SseEvent> {
  const reader = body.getReader()
  const decoder = new TextDecoder()
  let buffer = ''
  let currentEvent: SseEvent = {}

  // When the signal is aborted, cancel the reader to unblock
  // any pending reader.read() call. Without this, reader.read()
  // blocks forever on an open SSE stream.
  const onAbort = () => {
    reader.cancel().catch(() => {})
  }
  if (signal) {
    signal.addEventListener('abort', onAbort, { once: true })
  }

  try {
    while (true) {
      const { done, value } = await reader.read()
      if (done || signal?.aborted) break

      buffer += decoder.decode(value, { stream: true })
      const lines = buffer.split('\n')
      // Last line might be incomplete — keep for next chunk
      buffer = lines.pop() ?? ''

      for (const line of lines) {
        if (line.startsWith('event: ')) {
          currentEvent.event = line.slice(7).trim()
        } else if (line.startsWith('id: ')) {
          currentEvent.id = line.slice(4).trim()
        } else if (line.startsWith('data: ')) {
          currentEvent.data = (currentEvent.data ? currentEvent.data + '\n' : '') + line.slice(6)
        } else if (line === '') {
          // Empty line ends the event
          if (currentEvent.event || currentEvent.data) {
            yield { ...currentEvent }
          }
          currentEvent = {}
        }
      }
    }

    // Flush remaining buffer
    if (buffer.trim()) {
      const line = buffer.trim()
      if (line.startsWith('event: ')) {
        currentEvent.event = line.slice(7).trim()
      } else if (line.startsWith('id: ')) {
        currentEvent.id = line.slice(4).trim()
      } else if (line.startsWith('data: ')) {
        currentEvent.data = (currentEvent.data ?? '') + line.slice(6)
      }
    }
    if (currentEvent.event || currentEvent.data) {
      yield currentEvent
    }
  } finally {
    if (signal) {
      signal.removeEventListener('abort', onAbort)
    }
    reader.releaseLock()
  }
}

/**
 * Collect SSE events from a fetch response for a limited time.
 */
export async function collectSseEvents(
  response: Response,
  durationMs: number,
): Promise<SseEvent[]> {
  const controller = new AbortController()
  // Both the timeout callback and the finally block try to cancel the body.
  // This is safe because cancel() is idempotent — the first call flips
  // bodyUsed to true, so the second check (in finally) becomes a no-op.
  // The duality ensures cleanup regardless of exit path (timeout vs normal end).
  const cancelBody = () => {
    if (!response.bodyUsed) {
      response.body?.cancel().catch(() => {})
    }
  }
  const timer = setTimeout(() => {
    controller.abort()
    cancelBody()
  }, durationMs)

  const events: SseEvent[] = []
  try {
    for await (const event of readSseStream(response.body!, controller.signal)) {
      events.push(event)
    }
  } catch {
    // Timeout or stream end — expected
  } finally {
    clearTimeout(timer)
    cancelBody()
  }

  return events
}
