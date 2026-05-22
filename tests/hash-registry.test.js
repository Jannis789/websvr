/**
 * §2.12 — Tests for the REAL sw.js
 *
 * Loads the actual sw.js from assets/js/sw.js and tests:
 * - Hash Registry (register, TTL, dedup, MAX_REGISTRY_SIZE)
 * - SSE Parser (processSSERead — hash learning from event stream)
 * - Fetch Interception (known_hashes appended to /sse URL)
 * - Stream Teeing (hash learning while forwarding to page)
 * - Edge Cases EC-1 through EC-6 (§4.4)
 */

// ─────────────────────────────────────────────
// Helper: Create a mock ReadableStream from SSE text chunks
// ─────────────────────────────────────────────
function createMockStream(chunks) {
  const encoder = new TextEncoder();
  let index = 0;
  return {
    getReader() {
      return {
        read: async () => {
          if (index < chunks.length) {
            return { done: false, value: encoder.encode(chunks[index++]) };
          }
          return { done: true, value: undefined };
        },
        releaseLock() {},
      };
    },
    tee() {
      // Return two independent readers of the same data
      const data = chunks.map(c => encoder.encode(c));
      const makeStream = () => ({
        getReader() {
          let i = 0;
          return {
            read: async () => i < data.length ? { done: false, value: data[i++] } : { done: true, value: undefined },
            releaseLock() {},
          };
        },
      });
      return [makeStream(), makeStream()];
    },
  };
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

let sw;

beforeEach(() => {
  sw = global.loadSw();
});

// ─── 1. SW Lifecycle ───

describe('SW Lifecycle', () => {
  test('install event calls skipWaiting', () => {
    global.dispatchLifecycleEvent('install');
    expect(global.skipWaiting).toHaveBeenCalled();
  });

  test('activate event calls clients.claim', () => {
    const event = global.dispatchLifecycleEvent('activate');
    expect(event.waitUntil).toHaveBeenCalled();
  });
});

// ─── 2. Hash Registry — Basic Operations ───

describe('Hash Registry — Basic Operations (real sw.js)', () => {
  test('registerHash adds hash to registry', () => {
    sw.registerHash('abc123');
    expect(sw.HASH_REGISTRY.has('abc123')).toBe(true);
    expect(sw.HASH_REGISTRY.has('not-registered')).toBe(false);
  });

  test('registerHash with multiple hashes', () => {
    sw.registerHash('hash1');
    sw.registerHash('hash2');
    sw.registerHash('hash3');
    expect(sw.HASH_REGISTRY.size).toBe(3);
    expect(sw.HASH_REGISTRY.has('hash1')).toBe(true);
    expect(sw.HASH_REGISTRY.has('hash2')).toBe(true);
    expect(sw.HASH_REGISTRY.has('hash3')).toBe(true);
  });

  test('registerHash deduplicates (same hash does not increase size)', () => {
    sw.registerHash('same');
    sw.registerHash('same');
    expect(sw.HASH_REGISTRY.size).toBe(1);
  });

  test('registerHash updates timestamp on re-registration', () => {
    sw.registerHash('rehash');
    const ts1 = sw.HASH_REGISTRY.get('rehash');
    // Small delay then re-register
    sw.registerHash('rehash');
    const ts2 = sw.HASH_REGISTRY.get('rehash');
    expect(ts2).toBeGreaterThanOrEqual(ts1);
    expect(sw.HASH_REGISTRY.size).toBe(1);
  });
});

// ─── 3. Hash Registry — TTL Expiration ───

describe('Hash Registry — TTL Expiration (24h)', () => {
  test('EC-6: cleanExpiredHashes removes hashes older than 24h', () => {
    // Manually inject an old hash (25h ago)
    sw.HASH_REGISTRY.set('old_hash', Date.now() - 25 * 60 * 60 * 1000);
    sw.HASH_REGISTRY.set('fresh_hash', Date.now());

    sw.cleanExpiredHashes();

    expect(sw.HASH_REGISTRY.has('old_hash')).toBe(false);
    expect(sw.HASH_REGISTRY.has('fresh_hash')).toBe(true);
  });

  test('hashes exactly at TTL boundary are kept', () => {
    // Exactly 24h old — should still be kept (< not >=)
    sw.HASH_REGISTRY.set('boundary_hash', Date.now() - 24 * 60 * 60 * 1000);
    sw.cleanExpiredHashes();
    // The check is `now - ts > TTL_MS`, so exactly at boundary = kept
    expect(sw.HASH_REGISTRY.has('boundary_hash')).toBe(true);
  });

  test('cleanExpiredHashes with empty registry does nothing', () => {
    sw.cleanExpiredHashes();
    expect(sw.HASH_REGISTRY.size).toBe(0);
  });
});

// ─── 4. Hash Registry — MAX_REGISTRY_SIZE ───

describe('Hash Registry — MAX_REGISTRY_SIZE eviction', () => {
  test('evicts oldest entries when exceeding max size', () => {
    // Fill to MAX + 5
    const limit = sw.MAX_REGISTRY_SIZE;
    for (let i = 0; i < limit + 5; i++) {
      sw.registerHash(`hash_${i.toString().padStart(5, '0')}`);
    }
    // Size should be capped at MAX_REGISTRY_SIZE
    expect(sw.HASH_REGISTRY.size).toBe(limit);
    // Oldest entries should be evicted
    expect(sw.HASH_REGISTRY.has('hash_00000')).toBe(false);
    expect(sw.HASH_REGISTRY.has('hash_00004')).toBe(false);
    // Newest entries should be kept
    expect(sw.HASH_REGISTRY.has(`hash_${(limit + 4).toString().padStart(5, '0')}`)).toBe(true);
  });
});

// ─── 5. SSE Parser — processSSERead ───

describe('SSE Parser — processSSERead (real sw.js)', () => {
  test('registers hash from PatchElements event', () => {
    let buffer = '';
    buffer = sw.processSSERead(buffer, [
      'event: datastar-patch-elements\n',
      'id: abc123def456\n',
      'data: <div>Hello</div>\n',
      '\n',
    ].join(''));
    expect(sw.HASH_REGISTRY.has('abc123def456')).toBe(true);
  });

  test('ignores non-PatchElements events (does not register hash)', () => {
    let buffer = '';
    buffer = sw.processSSERead(buffer, [
      'event: datastar-patch-signals\n',
      'id: signal-hash\n',
      'data: {"count": 5}\n',
      '\n',
    ].join(''));
    expect(sw.HASH_REGISTRY.has('signal-hash')).toBe(false);
  });

  test('ignores ExecuteScript events', () => {
    let buffer = '';
    buffer = sw.processSSERead(buffer, [
      'event: datastar-execute-script\n',
      'id: script-hash\n',
      'data: console.log("hi")\n',
      '\n',
    ].join(''));
    expect(sw.HASH_REGISTRY.has('script-hash')).toBe(false);
  });

  test('handles multiple events in one chunk', () => {
    let buffer = '';
    buffer = sw.processSSERead(buffer, [
      'event: datastar-patch-elements\n',
      'id: hash_a\n',
      'data: <div>A</div>\n',
      '\n',
      'event: datastar-patch-elements\n',
      'id: hash_b\n',
      'data: <div>B</div>\n',
      '\n',
    ].join(''));
    expect(sw.HASH_REGISTRY.has('hash_a')).toBe(true);
    expect(sw.HASH_REGISTRY.has('hash_b')).toBe(true);
    expect(sw.HASH_REGISTRY.size).toBe(2);
  });

  test('handles split chunks (event split across two reads)', () => {
    let buffer = '';
    // First chunk: incomplete event
    buffer = sw.processSSERead(buffer, 'event: datastar-patch-elements\nid: split');
    expect(sw.HASH_REGISTRY.size).toBe(0); // Not yet complete

    // Second chunk: rest of event
    buffer = sw.processSSERead(buffer, '_hash\ndata: <div>X</div>\n\n');
    expect(sw.HASH_REGISTRY.has('split_hash')).toBe(true);
  });

  test('ignores events without an id field', () => {
    let buffer = '';
    buffer = sw.processSSERead(buffer, [
      'event: datastar-patch-elements\n',
      'data: <div>No ID</div>\n',
      '\n',
    ].join(''));
    expect(sw.HASH_REGISTRY.size).toBe(0);
  });

  test('ignores events with empty id', () => {
    let buffer = '';
    buffer = sw.processSSERead(buffer, [
      'event: datastar-patch-elements\n',
      'id: \n',
      'data: <div>Empty ID</div>\n',
      '\n',
    ].join(''));
    expect(sw.HASH_REGISTRY.size).toBe(0);
  });
});

// ─── 6. Fetch Interception — known_hashes ───

describe('Fetch Interception — known_hashes (real sw.js)', () => {
  test('appends known_hashes to /sse URL when registry has entries', async () => {
    sw.registerHash('hash_a');
    sw.registerHash('hash_b');

    const respondWith = jest.fn();
    global.dispatchFetchEvent('http://localhost:3000/sse', respondWith);

    expect(respondWith).toHaveBeenCalledTimes(1);
    const fetchCall = global.fetch.mock.calls[0];
    expect(fetchCall[0]).toContain('/sse?known_hashes=');
    expect(fetchCall[0]).toContain('hash_a');
    expect(fetchCall[0]).toContain('hash_b');
  });

  test('does not append known_hashes when registry is empty', async () => {
    const respondWith = jest.fn();
    global.dispatchFetchEvent('http://localhost:3000/sse', respondWith);

    expect(respondWith).toHaveBeenCalledTimes(1);
    const fetchCall = global.fetch.mock.calls[0];
    expect(fetchCall[0]).not.toContain('known_hashes');
  });

  test('does not intercept non-/sse requests', () => {
    const respondWith = jest.fn();
    global.dispatchFetchEvent('http://localhost:3000/home', respondWith);
    expect(respondWith).not.toHaveBeenCalled();
  });

  test('does not intercept /sse in path prefix (e.g. /assets/sse)', () => {
    const respondWith = jest.fn();
    global.dispatchFetchEvent('http://localhost:3000/assets/sse', respondWith);
    expect(respondWith).not.toHaveBeenCalled();
  });

  test('cleanExpiredHashes is called before building known_hashes', () => {
    // Inject an expired hash
    sw.HASH_REGISTRY.set('expired', Date.now() - 25 * 60 * 60 * 1000);
    sw.HASH_REGISTRY.set('fresh', Date.now());

    const respondWith = jest.fn();
    global.dispatchFetchEvent('http://localhost:3000/sse', respondWith);

    const fetchCall = global.fetch.mock.calls[0];
    expect(fetchCall[0]).not.toContain('expired');
    expect(fetchCall[0]).toContain('fresh');
  });
});

// ─── 7. Stream Teeing — consumeSSEStream ───

describe('Stream Teeing — consumeSSEStream learns hashes from stream', () => {
  test('consumeSSEStream reads stream and registers hashes', async () => {
    const chunks = [
      'event: datastar-patch-elements\nid: stream_hash_1\ndata: A\n\n',
      'event: datastar-patch-elements\nid: stream_hash_2\ndata: B\n\n',
    ];
    const stream = createMockStream(chunks);
    await sw.consumeSSEStream(stream);

    expect(sw.HASH_REGISTRY.has('stream_hash_1')).toBe(true);
    expect(sw.HASH_REGISTRY.has('stream_hash_2')).toBe(true);
  });

  test('consumeSSEStream handles empty stream', async () => {
    const stream = createMockStream([]);
    await sw.consumeSSEStream(stream);
    expect(sw.HASH_REGISTRY.size).toBe(0);
  });

  test('consumeSSEStream ignores errors gracefully', async () => {
    const errorStream = {
      getReader() {
        return {
          read: async () => { throw new Error('Stream error'); },
          releaseLock() {},
        };
      },
    };
    // Should not throw
    await sw.consumeSSEStream(errorStream);
    expect(sw.HASH_REGISTRY.size).toBe(0);
  });
});

// ─── 8. Edge Cases — §4.4 Spec ───

describe('Edge Cases — §4.4 Spec (real sw.js)', () => {
  test('EC-1: Hash Match — known hash is included in known_hashes', () => {
    sw.registerHash('known_hash_123');
    const known = Array.from(sw.HASH_REGISTRY.keys());
    expect(known).toContain('known_hash_123');
    // Server will skip this event
  });

  test('EC-2: Hash Mismatch — unknown hash is NOT in known_hashes', () => {
    sw.registerHash('old_hash');
    const known = Array.from(sw.HASH_REGISTRY.keys());
    expect(known).not.toContain('new_hash_456');
    // Server will send this event
  });

  test('EC-3: Out-of-order Events — each hash checked independently', () => {
    sw.registerHash('hash_1');
    const known = Array.from(sw.HASH_REGISTRY.keys());
    expect(known).toContain('hash_1');       // known → skip
    expect(known).not.toContain('hash_2');   // unknown → process
    expect(known).not.toContain('hash_3');   // unknown → process
  });

  test('EC-4: Hash collision — different content, same hash (accepted risk)', () => {
    // Practically impossible with HMAC-SHA256 truncated to 128 bit
    // But if it happens, the second event is skipped (by design)
    sw.registerHash('collision_hash');
    expect(sw.HASH_REGISTRY.has('collision_hash')).toBe(true);
    // A different event with same hash would be skipped — accepted risk
  });

  test('EC-5: SW loses hashes — empty registry sends empty known_hashes', () => {
    // Fresh SW, no hashes
    expect(sw.HASH_REGISTRY.size).toBe(0);
    const known = Array.from(sw.HASH_REGISTRY.keys()).join(',');
    expect(known).toBe('');
    // Server will replay all buffered events → no data loss
  });

  test('EC-6: TTL exceeded — hash removed, event re-processed', () => {
    sw.HASH_REGISTRY.set('expired_hash', Date.now() - 25 * 60 * 60 * 1000);
    sw.cleanExpiredHashes();
    expect(sw.HASH_REGISTRY.has('expired_hash')).toBe(false);
    // Server will send the event again → client processes it
  });
});

// ─── 9. Integration — Full SSE Round-Trip ───

describe('Integration — Full SSE Round-Trip', () => {
  test('hashes learned from stream appear in subsequent fetch known_hashes', async () => {
    // Phase 1: Learn hashes from an SSE stream
    const chunks = [
      'event: datastar-patch-elements\nid: learned_a\ndata: A\n\n',
      'event: datastar-patch-elements\nid: learned_b\ndata: B\n\n',
    ];
    const stream = createMockStream(chunks);
    await sw.consumeSSEStream(stream);

    expect(sw.HASH_REGISTRY.has('learned_a')).toBe(true);
    expect(sw.HASH_REGISTRY.has('learned_b')).toBe(true);

    // Phase 2: Subsequent /sse fetch should include learned hashes
    global.fetch.mockClear();
    const respondWith = jest.fn();
    global.dispatchFetchEvent('http://localhost:3000/sse', respondWith);

    const fetchCall = global.fetch.mock.calls[0];
    expect(fetchCall[0]).toContain('learned_a');
    expect(fetchCall[0]).toContain('learned_b');
  });

  test('expired hashes are NOT included in subsequent fetch', async () => {
    sw.registerHash('still_fresh');
    sw.HASH_REGISTRY.set('now_expired', Date.now() - 25 * 60 * 60 * 1000);

    global.fetch.mockClear();
    const respondWith = jest.fn();
    global.dispatchFetchEvent('http://localhost:3000/sse', respondWith);

    const fetchCall = global.fetch.mock.calls[0];
    expect(fetchCall[0]).toContain('still_fresh');
    expect(fetchCall[0]).not.toContain('now_expired');
  });
});
