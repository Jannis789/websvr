
import { describe, test, expect, beforeAll, afterAll } from 'vitest'
import {
  startBackend,
  stopBackend,
  generateClientId,
  authHeaders,
  collectSseEvents,
} from './helpers/backend'

let BASE_URL: string

beforeAll(async () => {
  BASE_URL = await startBackend()
}, 30_000)

afterAll(() => stopBackend())

test('raw SSE wire debug', async () => {
  const cid = generateClientId()
  const headers = await authHeaders(cid)

  const sse = await fetch(`${BASE_URL}/sse`, {
    headers: { ...headers, Accept: 'text/event-stream' },
  })
  await new Promise(r => setTimeout(r, 500))

  await fetch(`${BASE_URL}/test/1`, { headers })
  const events = await collectSseEvents(sse, 2000)

  for (const e of events) {
    const lines: string[] = []
    if (e.id !== undefined) lines.push(`id: ${e.id}`)
    if (e.event) lines.push(`event: ${e.event}`)
    if (e.data) lines.push(e.data)
    console.log("---EVENT---")
    console.log(lines.join("\n"))
    console.log("---END---")
    console.log("parsed:", JSON.stringify(e))
  }
})
