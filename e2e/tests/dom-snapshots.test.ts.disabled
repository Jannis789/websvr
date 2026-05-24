import { beforeAll, beforeEach, afterEach, describe, expect, it } from 'vitest'
import { Window } from 'happy-dom'
import { readFileSync } from 'fs'
import { join } from 'path'

// ── Paths ──────────────────────────────────────────────
const ASSETS = join(__dirname, '../../crates/platform-backend/assets')
const TEMPLATES = join(ASSETS, 'templates')

// ── Helpers ────────────────────────────────────────────

/**
 * Set up a happy-dom Window with the given HTML document.
 * Returns the window, document, and a cleanup function.
 */
function setupDom(html: string) {
  const win = new Window({
    url: 'http://localhost:3000',
    width: 1024,
    height: 768,
  })
  const doc = win.document
  doc.open()
  doc.write(html)
  doc.close()
  return { win, doc, cleanup: () => win.close() }
}

/**
 * Read a template file from the assets directory.
 */
function loadTemplate(name: string): string {
  return readFileSync(join(TEMPLATES, name), 'utf-8')
}

/**
 * Simulate a Datastar PatchElements event by directly
 * dispatching it through the DOM. Datastar's fetch plugin
 * dispatches 'datastar-fetch' CustomEvents with SSE data.
 *
 * For snapshot tests we bypass the SSE transport and call
 * Datastar's patchElements watcher directly by dispatching
 * a DOMContentLoaded-style event that Datastar processes.
 *
 * Since datastar-core.js registers as an ESM module, we
 * simulate the patching by directly manipulating the DOM
 * in the same way Datastar's patchElements watcher does:
 * parse the HTML, find the selector, and apply the mode.
 */
function applyPatchElements(
  doc: Document,
  elements: string,
  opts: { selector?: string; mode?: string } = {},
) {
  const mode = opts.mode || 'inner'
  const selector = opts.selector || ''

  // Parse the HTML content using DOMParser
  const parser = new DOMParser()
  const parsed = parser.parseFromString(
    `<body><template>${elements}</template></body>`,
    'text/html',
  )
  const fragment = parsed.querySelector('template')!.content

  if (selector) {
    const targets = doc.querySelectorAll(selector)
    for (const target of targets) {
      applyMode(doc, target as Element, fragment.cloneNode(true) as DocumentFragment, mode)
    }
  } else if (mode === 'outer' || mode === 'replace') {
    // Match by id — Datastar's default behavior
    for (const child of Array.from(fragment.children)) {
      if (child.id) {
        const target = doc.getElementById(child.id)
        if (target) {
          if (mode === 'outer' || mode === 'replace') {
            target.replaceWith(child.cloneNode(true))
          }
        }
      }
    }
  }
}

function applyMode(
  _doc: Document,
  target: Element,
  content: DocumentFragment,
  mode: string,
) {
  switch (mode) {
    case 'inner':
      target.innerHTML = ''
      for (const child of Array.from(content.children)) {
        target.appendChild(child.cloneNode(true))
      }
      break
    case 'outer':
    case 'replace':
      target.replaceWith(content)
      break
    case 'append':
      for (const child of Array.from(content.children)) {
        target.appendChild(child.cloneNode(true))
      }
      break
    case 'prepend':
      for (const child of Array.from(content.children).reverse()) {
        target.insertBefore(child.cloneNode(true), target.firstChild)
      }
      break
    case 'remove':
      target.remove()
      break
    case 'before':
      target.parentElement?.insertBefore(content, target)
      break
    case 'after': {
      const next = target.nextSibling
      if (next) {
        target.parentElement?.insertBefore(content, next)
      } else {
        target.parentElement?.appendChild(content)
      }
      break
    }
  }
}

// ════════════════════════════════════════════════════════
// Snapshot Tests — HTML Templates
// ════════════════════════════════════════════════════════

describe('HTML Template Snapshots', () => {
  const templateFiles = [
    'home.html',
    'test.html',
    'login.html',
    'register.html',
  ]

  for (const file of templateFiles) {
    it(`snapshot: ${file} structure is stable`, () => {
      const html = loadTemplate(file)
      const { doc, cleanup } = setupDom(html)

      // Snapshot key structural elements
      const head = doc.head
      const body = doc.body

      // Snapshot <head> children (scripts, styles, meta)
      const headSnapshot = Array.from(head.children).map((el: Element) => ({
        tag: el.tagName.toLowerCase(),
        src: el.getAttribute('src'),
        rel: el.getAttribute('rel'),
        href: el.getAttribute('href'),
        type: el.getAttribute('type'),
        id: el.id || undefined,
      }))
      expect(headSnapshot).toMatchSnapshot(`${file} <head>`)

      // Snapshot body structure (top-level sections only)
      const bodySnapshot = Array.from(body.children).map((el: Element) => ({
        tag: el.tagName.toLowerCase(),
        id: el.id || undefined,
        class: el.className || undefined,
        dataAttrs: Object.keys(el.dataset).length
          ? Object.fromEntries(Object.entries(el.dataset))
          : undefined,
      }))
      expect(bodySnapshot).toMatchSnapshot(`${file} <body> structure`)

      cleanup()
    })
  }
})

// ════════════════════════════════════════════════════════
// Snapshot Tests — PatchElements DOM mutations
// ════════════════════════════════════════════════════════

describe('Datastar PatchElements → DOM snapshots', () => {
  const BASE_HTML = `<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><title>Test</title></head>
<body>
  <div id="app">
    <div id="content">Initial content</div>
    <div id="sidebar">Sidebar</div>
    <ul id="list"></ul>
    <div id="test-results">
      <p>Click "Run Tests" to start.</p>
    </div>
  </div>
</body>
</html>`

  it('mode=inner: replaces children of selector target', () => {
    const { doc, cleanup } = setupDom(BASE_HTML)
    applyPatchElements(doc, '<span class="updated">New content</span>', {
      selector: '#content',
      mode: 'inner',
    })
    const content = doc.getElementById('content')!
    expect(content.innerHTML).toMatchSnapshot('inner mode result')
    cleanup()
  })

  it('mode=outer: replaces target element (matched by id)', () => {
    const { doc, cleanup } = setupDom(BASE_HTML)
    applyPatchElements(doc, '<div id="content" class="replaced"><h2>Replaced!</h2></div>', {
      mode: 'outer',
    })
    const app = doc.getElementById('app')!
    expect(app.innerHTML).toMatchSnapshot('outer mode result')
    cleanup()
  })

  it('mode=append: appends children to target', () => {
    const { doc, cleanup } = setupDom(BASE_HTML)
    applyPatchElements(doc, '<li>Item 1</li><li>Item 2</li>', {
      selector: '#list',
      mode: 'append',
    })
    const list = doc.getElementById('list')!
    expect(list.innerHTML).toMatchSnapshot('append mode result')
    cleanup()
  })

  it('mode=prepend: prepends children to target', () => {
    const { doc, cleanup } = setupDom(BASE_HTML)
    applyPatchElements(doc, '<li>First</li>', {
      selector: '#list',
      mode: 'prepend',
    })
    const list = doc.getElementById('list')!
    expect(list.innerHTML).toMatchSnapshot('prepend mode result')
    cleanup()
  })

  it('mode=remove: removes target element', () => {
    const { doc, cleanup } = setupDom(BASE_HTML)
    applyPatchElements(doc, '', {
      selector: '#sidebar',
      mode: 'remove',
    })
    const app = doc.getElementById('app')!
    expect(app.innerHTML).toMatchSnapshot('remove mode result')
    cleanup()
  })

  it('mode=before: inserts before target', () => {
    const { doc, cleanup } = setupDom(BASE_HTML)
    applyPatchElements(doc, '<div id="before-content">Before</div>', {
      selector: '#content',
      mode: 'before',
    })
    const app = doc.getElementById('app')!
    expect(app.innerHTML).toMatchSnapshot('before mode result')
    cleanup()
  })

  it('mode=after: inserts after target', () => {
    const { doc, cleanup } = setupDom(BASE_HTML)
    applyPatchElements(doc, '<div id="after-content">After</div>', {
      selector: '#content',
      mode: 'after',
    })
    const app = doc.getElementById('app')!
    expect(app.innerHTML).toMatchSnapshot('after mode result')
    cleanup()
  })
})

// ════════════════════════════════════════════════════════
// Snapshot Tests — Full test sequence (test.rs Phase A-G)
// ════════════════════════════════════════════════════════

describe('Test sequence DOM snapshots (mirrors Rust test.rs)', () => {
  const TEST_HTML = `<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><title>Test</title></head>
<body>
  <div id="test-results">
    <p class="text-muted">Click "Run Tests" to start.</p>
  </div>
  <div id="test-score" style="display:none">
    <span id="score-value">—</span>
  </div>
</body>
</html>`

  // Exact HTML payloads from test.rs Phase A
  const PHASE_A_EVENTS = [
    "<div id='test-1' class='test-pass' data-phase='A' data-type='fresh'>✅ Fresh A: Event 1 received</div>",
    "<div id='test-2' class='test-pass' data-phase='A' data-type='fresh'>✅ Fresh A: Event 2 received</div>",
    "<div id='test-3' class='test-pass' data-phase='A' data-type='fresh'>✅ Fresh A: Event 3 received</div>",
    "<div id='test-4' class='test-pass' data-phase='A' data-type='fresh'>✅ Fresh A: Event 4 received</div>",
  ]

  // Phase C: modified content
  const PHASE_C_EVENT = "<div id='test-1' class='test-pass' data-phase='C' data-type='modified'>✅ Phase C: Modified content — cache miss (different hash)</div>"

  // Phase D: out-of-order events
  const PHASE_D_EVENTS = [
    "<div id='ooo-3' class='test-pass' data-phase='D' data-type='out-of-order'>✅ Out-of-order: Event 3 (sent 1st)</div>",
    "<div id='ooo-1' class='test-pass' data-phase='D' data-type='out-of-order'>✅ Out-of-order: Event 1 (sent 2nd)</div>",
    "<div id='ooo-2' class='test-pass' data-phase='D' data-type='out-of-order'>✅ Out-of-order: Event 2 (sent 3rd)</div>",
  ]

  it('Phase A: 4 fresh events patch into #test-results', () => {
    const { doc, cleanup } = setupDom(TEST_HTML)
    const results = doc.getElementById('test-results')!

    for (const html of PHASE_A_EVENTS) {
      applyPatchElements(doc, html, { selector: '#test-results', mode: 'append' })
    }

    // Snapshot the final state of test-results
    expect(results.innerHTML).toMatchSnapshot('phase-a results')
    // Also snapshot structured data for programmatic assertions
    const passes = results.querySelectorAll('.test-pass')
    expect(passes.length).toBe(4)
    cleanup()
  })

  it('Phase C: modified event replaces test-1 by id (outer mode)', () => {
    const { doc, cleanup } = setupDom(TEST_HTML)
    const results = doc.getElementById('test-results')!

    // First, apply Phase A events
    for (const html of PHASE_A_EVENTS) {
      applyPatchElements(doc, html, { selector: '#test-results', mode: 'append' })
    }

    // Then apply Phase C — replaces #test-1 via outer mode (id match)
    applyPatchElements(doc, PHASE_C_EVENT, { mode: 'outer' })

    // Snapshot #test-1 after modification
    const test1 = doc.getElementById('test-1')!
    expect(test1.outerHTML).toMatchSnapshot('phase-c modified test-1')
    expect(test1.getAttribute('data-phase')).toBe('C')
    expect(test1.getAttribute('data-type')).toBe('modified')
    cleanup()
  })

  it('Phase D: out-of-order events append in send order', () => {
    const { doc, cleanup } = setupDom(TEST_HTML)
    const results = doc.getElementById('test-results')!

    for (const html of PHASE_D_EVENTS) {
      applyPatchElements(doc, html, { selector: '#test-results', mode: 'append' })
    }

    expect(results.innerHTML).toMatchSnapshot('phase-d out-of-order')
    // Verify all 3 are present
    expect(results.querySelectorAll('[data-phase="D"]').length).toBe(3)
    // Verify send order (ooo-3, ooo-1, ooo-2)
    const ids = Array.from(results.querySelectorAll('[data-phase="D"]')).map(
      (el) => el.id,
    )
    expect(ids).toEqual(['ooo-3', 'ooo-1', 'ooo-2'])
    cleanup()
  })

  it('Full sequence: Phase A → C → D combined snapshot', () => {
    const { doc, cleanup } = setupDom(TEST_HTML)
    const results = doc.getElementById('test-results')!

    // Phase A
    for (const html of PHASE_A_EVENTS) {
      applyPatchElements(doc, html, { selector: '#test-results', mode: 'append' })
    }
    // Phase C (replaces test-1)
    applyPatchElements(doc, PHASE_C_EVENT, { mode: 'outer' })
    // Phase D
    for (const html of PHASE_D_EVENTS) {
      applyPatchElements(doc, html, { selector: '#test-results', mode: 'append' })
    }

    // Full snapshot of test-results
    expect(results.innerHTML).toMatchSnapshot('full sequence A+C+D')

    // Verify Phase A: test-2, test-3, test-4 remain unchanged
    expect(doc.getElementById('test-2')!.getAttribute('data-phase')).toBe('A')
    expect(doc.getElementById('test-3')!.getAttribute('data-phase')).toBe('A')
    expect(doc.getElementById('test-4')!.getAttribute('data-phase')).toBe('A')
    // Verify Phase C: test-1 was updated
    expect(doc.getElementById('test-1')!.getAttribute('data-phase')).toBe('C')
    // Verify Phase D: 3 out-of-order events
    expect(doc.querySelectorAll('[data-phase="D"]').length).toBe(3)
    cleanup()
  })
})

// ════════════════════════════════════════════════════════
// Snapshot Tests — Partial templates (home_overview, etc.)
// ════════════════════════════════════════════════════════

describe('Partial template snapshots', () => {
  const PARTIALS_DIR = join(TEMPLATES)

  it('home_overview.html partial snapshot', () => {
    const html = readFileSync(join(PARTIALS_DIR, 'home_overview.html'), 'utf-8')
    const { doc, cleanup } = setupDom(`<!DOCTYPE html><html><body>${html}</body></html>`)
    const body = doc.body
    // Snapshot the card structure
    const cards = body.querySelectorAll('.card')
    expect(cards.length).toBeGreaterThan(0)
    const cardData = Array.from(cards).map((card: Element) => ({
      header: card.querySelector('.card-header')?.textContent?.trim(),
      body: card.querySelector('.card-body')?.textContent?.trim(),
    }))
    expect(cardData).toMatchSnapshot('home_overview cards')
    cleanup()
  })

  it('snapshot CSS files for unexpected changes', () => {
    const cssDir = join(ASSETS, 'css')
    const cssFiles = ['common.css', 'dark.css', 'light.css']
    for (const file of cssFiles) {
      const content = readFileSync(join(cssDir, file), 'utf-8')
      expect(content).toMatchSnapshot(`css/${file}`)
    }
  })
})
