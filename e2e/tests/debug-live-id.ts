
import { startBackend, stopBackend, generateClientId, authHeaders, collectSseEvents } from './helpers/backend'

async function main() {
  const BASE_URL = await startBackend()
  const cid = generateClientId()
  const headers = await authHeaders(cid)

  // Connect SSE
  const sse = await fetch(`${BASE_URL}/sse`, {
    headers: { ...headers, Accept: 'text/event-stream' },
  })
  
  // Wait for initial
  await new Promise(r => setTimeout(r, 500))

  // Trigger event
  const r = await fetch(`${BASE_URL}/test/1`, { headers })
  console.log('trigger status:', r.status)
  
  // Collect events
  const events = await collectSseEvents(sse, 2000)
  console.log('events count:', events.length)
  for (const e of events) {
    console.log(JSON.stringify({ id: e.id, event: e.event, hasData: !!e.data }))
  }
  
  stopBackend()
}

main().catch(e => { console.error(e); stopBackend(); process.exit(1) })
