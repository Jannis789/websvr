
import { spawn } from "child_process"
import { setTimeout as sleep } from "timers/promises"

const DB = "/tmp/platform-raw-debug2.db"
const CID = "raw-wire-001"

// Start server
const server = spawn("./target/debug/platform-backend", [], {
  cwd: "/home/jannis/Dokumente/test2",
  env: { ...process.env, DATABASE_URL: `sqlite://${DB}?mode=rwc`, HMAC_SECRET: "***" },
  stdio: ["ignore", "pipe", "pipe"]
})
server.stdout.on("data", d => process.stderr.write("[srv] " + d))
server.stderr.on("data", d => process.stderr.write("[srv-err] " + d))

await sleep(2000)

// Auth
await fetch("http://localhost:3000/test/auth", { headers: { cookie: `platform_cid=${CID}` } })
console.error("auth done")

// Start SSE connection - get RAW body
const sseResp = await fetch("http://localhost:3000/sse", {
  headers: { cookie: `platform_cid=${CID}`, Accept: "text/event-stream" }
})
console.error("sse connected, status:", sseResp.status)

// Read raw bytes for 1 second, then trigger, read more
const reader = sseResp.body!.getReader()
const decoder = new TextDecoder()
let raw = ""

// Read initial (should be empty)
await sleep(500)

// Trigger
await fetch("http://localhost:3000/test/1", { headers: { cookie: `platform_cid=${CID}` } })
console.error("triggered /test/1")

// Read for 2 seconds
const ctrl = new AbortController()
setTimeout(() => ctrl.abort(), 2500)

try {
  while (true) {
    const { done } = await Promise.race([
      reader.read(),
      new Promise(r => setTimeout(() => r({ done: true }), 2500))
    ])
    if (done) break
  }
} catch {}

// Get raw text
reader.cancel().catch(() => {})

// Easier: just use collectSseEvents and print raw
const { collectSseEvents } = await import("./tests/helpers/backend.js")
// Reconnect
const sse2 = await fetch("http://localhost:3000/sse", {
  headers: { cookie: `platform_cid=${CID}`, Accept: "text/event-stream" }
})
await sleep(500)
await fetch("http://localhost:3000/test/1", { headers: { cookie: `platform_cid=${CID}` } })
const events = await collectSseEvents(sse2, 2000)

// Print raw wire format for each event
for (const e of events) {
  const lines = []
  if (e.id !== undefined) lines.push("id: " + e.id)
  if (e.event) lines.push("event: " + e.event)
  if (e.data) lines.push(e.data)
  console.log("---EVENT---")
  console.log(lines.join("\n"))
  console.log("---END---")
}

server.kill()
process.exit(0)
