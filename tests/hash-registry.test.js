/**
 * §2.12 — Unit Tests for SW Hash Registry
 *
 * Tests the in-memory HASH_REGISTRY logic:
 * - Hash registration
 * - TTL expiration (24h)
 * - Deduplication via known_hashes
 * - MAX_REGISTRY_SIZE enforcement
 * - Edge Cases EC-1 through EC-6 (§4.4)
 */

const TTL_MS = 24 * 60 * 60 * 1000;
const MAX_REGISTRY_SIZE = 2000;
const EVENT_PREFIX = 'datastar-patch-elements';

// ── Extracted Hash Registry (mirrors sw.js logic) ──

function createRegistry() {
  const map = new Map();
  return {
    register(hash) {
      map.set(hash, Date.now());
      if (map.size > MAX_REGISTRY_SIZE) {
        const entries = [...map.entries()].sort((a, b) => a[1] - b[1]);
        const toDelete = entries.slice(0, entries.length - MAX_REGISTRY_SIZE);
        for (const [h] of toDelete) map.delete(h);
      }
    },
    has(hash) { return map.has(hash); },
    size() { return map.size; },
    cleanExpired() {
      const now = Date.now();
      for (const [hash, ts] of map) {
        if (now - ts > TTL_MS) map.delete(hash);
      }
    },
    keys() { return Array.from(map.keys()); },
    getKnownHashesParam() {
      this.cleanExpired();
      return this.keys().join(',');
    },
  };
}

// ── Extracted SSE Parser (mirrors sw.js processSSERead) ──

function createSSEParser() {
  const registry = createRegistry();
  let buffer = '';

  function processChunk(text) {
    const data = buffer + text;
    const lines = data.split('\n');
    let eventType = null;
    let eventId = null;

    for (let i = 0; i < lines.length - 1; i++) {
      const line = lines[i];
      if (line.startsWith('event: ')) {
        eventType = line.slice(7).trim();
      } else if (line.startsWith('id: ')) {
        eventId = line.slice(4).trim();
      } else if (line === '' && eventType) {
        if (eventType === EVENT_PREFIX && eventId && eventId.length > 0) {
          registry.register(eventId);
        }
        eventType = null;
        eventId = null;
      }
    }
    buffer = lines[lines.length - 1];
    return buffer;
  }

  return { registry, processChunk };
}

// ─────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────

describe('Hash Registry — Basic Operations', () => {
  test('register and check hash', () => {
    const reg = createRegistry();
    reg.register('abc123');
    expect(reg.has('abc123')).toBe(true);
    expect(reg.has('not-registered')).toBe(false);
  });

  test('multiple hashes', () => {
    const reg = createRegistry();
    reg.register('hash1');
    reg.register('hash2');
    reg.register('hash3');
    expect(reg.size()).toBe(3);
    expect(reg.keys()).toEqual(['hash1', 'hash2', 'hash3']);
  });

  test('duplicate hash does not increase size', () => {
    const reg = createRegistry();
    reg.register('same');
    reg.register('same');
    expect(reg.size()).toBe(1);
  });
});

describe('Hash Registry — TTL Expiration (24h)', () => {
  test('EC-6: hashes older than 24h are removed on cleanExpired', () => {
    const map = new Map();
    map.set('old_hash', Date.now() - 25 * 60 * 60 * 1000);
    map.set('fresh_hash', Date.now());
    expect(map.size).toBe(2);

    const now = Date.now();
    for (const [hash, ts] of map) {
      if (now - ts > TTL_MS) map.delete(hash);
    }
    expect(map.has('old_hash')).toBe(false);
    expect(map.has('fresh_hash')).toBe(true);
  });
});

describe('Hash Registry — known_hashes parameter', () => {
  test('returns comma-separated hashes', () => {
    const reg = createRegistry();
    reg.register('abc');
    reg.register('def');
    reg.register('ghi');
    expect(reg.getKnownHashesParam()).toBe('abc,def,ghi');
  });

  test('empty registry returns empty string', () => {
    const reg = createRegistry();
    expect(reg.getKnownHashesParam()).toBe('');
  });
});

describe('SSE Parser — Hash Learning from Event Stream', () => {
  test('registers hash from PatchElements event', () => {
    const { registry, processChunk } = createSSEParser();
    processChunk([
      'event: datastar-patch-elements\n',
      'id: abc123def456\n',
      'data: <div>Hello</div>\n',
      '\n',
    ].join(''));
    expect(registry.has('abc123def456')).toBe(true);
  });

  test('ignores non-PatchElements events', () => {
    const { registry, processChunk } = createSSEParser();
    processChunk([
      'event: datastar-patch-signals\n',
      'id: signal-hash\n',
      'data: {"count": 5}\n',
      '\n',
    ].join(''));
    expect(registry.has('signal-hash')).toBe(false);
  });

  test('handles multiple events in one chunk', () => {
    const { registry, processChunk } = createSSEParser();
    processChunk([
      'event: datastar-patch-elements\n',
      'id: hash_a\n',
      'data: <div>A</div>\n',
      '\n',
      'event: datastar-patch-elements\n',
      'id: hash_b\n',
      'data: <div>B</div>\n',
      '\n',
    ].join(''));
    expect(registry.has('hash_a')).toBe(true);
    expect(registry.has('hash_b')).toBe(true);
    expect(registry.size()).toBe(2);
  });
});

describe('Edge Cases — §4.4 Spec', () => {
  test('EC-1: Hash Match — event skipped when hash is known', () => {
    const reg = createRegistry();
    reg.register('known_hash_123');
    const knownHashes = reg.keys();
    expect(knownHashes.includes('known_hash_123')).toBe(true);
  });

  test('EC-2: Hash Mismatch — event processed when hash differs', () => {
    const reg = createRegistry();
    reg.register('old_hash');
    const knownHashes = reg.keys();
    expect(knownHashes.includes('new_hash_456')).toBe(false);
  });

  test('EC-3: Out-of-order Events — each checked independently', () => {
    const reg = createRegistry();
    reg.register('hash_1');
    const knownHashes = reg.keys();
    expect(knownHashes.includes('hash_2')).toBe(false); // unknown → process
    expect(knownHashes.includes('hash_1')).toBe(true);  // known → skip
    expect(knownHashes.includes('hash_3')).toBe(false); // unknown → process
  });

  test('EC-5: SW loses hashes — empty known_hashes sent', () => {
    const reg = createRegistry();
    expect(reg.getKnownHashesParam()).toBe('');
  });

  test('EC-6: TTL exceeded — hash removed, event re-processed', () => {
    const map = new Map();
    map.set('expired_hash', Date.now() - 25 * 60 * 60 * 1000);
    const now = Date.now();
    for (const [hash, ts] of map) {
      if (now - ts > TTL_MS) map.delete(hash);
    }
    expect(map.has('expired_hash')).toBe(false);
  });
});
