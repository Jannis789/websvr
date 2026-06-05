# AGENTS.md — Rama Platform

AI agent context file. Read this before working on the codebase.

**This project follows the Seven Pillars of Stream Architecture** (defined in `~/.hermes/SOUL.md`).
Violations of these principles are bugs, not style choices.

---

## Tech Stack

| Layer | Technology | Version |
|-------|-----------|---------|
| Backend | Rust + Rama HTTP framework | 0.3.0-alpha.4 |
| Frontend Reactivity | Datastar (SSE-only) | v1.0.1 |
| Database | SeaORM + SQLite | 1.1.19 |
| Crypto | ring (HMAC-SHA256 für Passwort-Hashing) | 0.17.14 |
| Logging | Custom `elog!` macro (NOT tracing) | — |
| CSS | Tokyo Night + GNOME/libadwaita | Latest CSS only |
| Icons | GNOME Icon Development Kit | mask-image system |

---

## Project Structure

```
/home/jannis/Dokumente/test2/
├── Cargo.toml                          # Workspace root
├── crates/
│   ├── platform-core/                  # Pure domain types, NO async/HTTP deps
│   │   └── src/
│   │       ├── lib.rs                  # pub mod + re-exports only
│   │       ├── client_id.rs            # ClientId (UUID v4 wrapper)
│   │       ├── config.rs               # Config singleton (OnceLock + env vars)
│   │       ├── session.rs              # SessionStorage + StorageMode enum
│   │       ├── i18n.rs                 # I18n + Lang enum
│   │       └── password.rs             # PasswordUtil (ring-based hashing)
│   │
│   └── platform-backend/               # HTTP server, handlers, SSE, layers
│       ├── assets/
│       │   ├── css/
│       │   │   ├── common/
│       │   │   │   ├── base.css        # Reset + icon system (.icon-name mappings)
│       │   │   │   └── theme.css       # CSS custom properties (colors)
│       │   │   ├── features/           # Component CSS (one file each)
│       │   │   │   ├── sidebar.css, content.css, window.css, ...
│       │   │   │   ├── button.css, form.css, switch.css, popup.css, ...
│       │   │   │   └── settings.css, card.css, auth.css, utility.css
│       │   │   └── pages/              # Entry points with @import
│       │   │       ├── home.css, login.css, register.css, settings.css
│       │   ├── js/
│       │   │   ├── datastar.js         # Datastar core
│       │   │   └── sw.js               # Service Worker (ver+epoch Resume)
│       │   ├── icons/                  # GNOME SVG icons
│       │   ├── fragments/              # HTML fragments (loaded via include_str!)
│       │   │   ├── shell.html          # App shell (#sidebar-slot, #header-slot, #content-slot)
│       │   │   ├── sidebar/            # sidebar.html, menu.html, header.html, footer.html
│       │   │   ├── content/            # overview.html, movies.html, series.html
│       │   │   ├── main/               # header.html
│       │   │   ├── auth/               # login-form.html, register-form.html, header.html
│       │   │   └── settings/           # account.html, email.html, password.html, ...
│       │   └── templates/              # Full page HTML (login.html, test.html)
│       │
│       └── src/
│           ├── lib.rs                  # pub mod + elog! macro
│           ├── main.rs                 # Entrypoint
│           ├── routes.rs               # Router setup, run()
│           ├── db.rs                   # Database init
│           ├── entities/               # SeaORM entities (users.rs, sessions.rs)
│           ├── context/
│           │   ├── shared_state.rs     # SharedState (global: Config, DB, I18n, SseBroadcaster)
│           │   ├── client_context.rs   # ClientContext (per-request aggregate)
│           │   └── session_storage.rs  # SessionStorageService + SessionMap
│           ├── components/
│           │   ├── patch.rs            # Patch trait + PatchEntry struct
│           │   ├── fragment.rs         # Fragment (Patch impl for slot patches)
│           │   ├── shell.rs            # Shell builder (collects patches, emits SSE)
│           │   └── sidebar.rs          # Sidebar component
│           ├── handlers/
│           │   ├── auth.rs             # POST /login, /register, /logout
│           │   ├── navigate.rs         # GET /home/overview, /home/movies, /home/series
│           │   ├── sse_handler.rs      # GET /sse (SSE stream mit ver+epoch Resume)
│           │   ├── i18n_handler.rs     # GET /i18n/{lang}.json
│           │   ├── test.rs             # GET /test, /test/auth, /test/run
│           │   └── pages/              # Full-page handlers (login, register, home, settings)
│           ├── layers/
│           │   ├── auth.rs             # AuthService + require_auth()
│           │   ├── client_context.rs   # ClientContextService
│           │   └── session_stack.rs    # Session stack (session_layer + broadcaster)
│           ├── sse/
│           │   ├── buffered_event.rs   # BufferedEvent (patch_ver + payload)
│           │   └── sse_broadcaster.rs  # SseBroadcaster (tokio broadcast channel)
│           └── utils/
│               ├── request.rs          # extract_context() utility
│               ├── response.rs         # Response type, redirect(), html_response()
│               └── log.rs              # Custom logging backend for elog!
│
└── references/                         # Offline documentation (GROUND TRUTH)
    ├── prompt.md                       # Canonical project specification
    ├── architecture.md                 # Architecture reference
    ├── rama-api.md                     # Rama API crash course
    ├── todo.md                         # Phase-based roadmap
    ├── component-system.md             # Component system design
    ├── uml-klassendiagramm-vollstaendig.md  # UML class diagram
    ├── rustdoc-rama-0.3.0-alpha.4/     # Full Rama rustdoc
    ├── sea-orm-1.1.19/                 # SeaORM docs
    ├── rustdoc-ring-0.17.14/           # Ring rustdoc
    ├── datastar/                       # Datastar SDK + examples
    └── icon-development-kit/           # GNOME Icon Development Kit SVGs
```

---

## Conventions & Rules (HARD-WON)

### The Seven Pillars (Summary — full definition in SOUL.md)

| # | Principle | TL;DR |
|---|-----------|-------|
| 1 | **Fetch-SSE Contract** | Client fetches, server streams. SSE response, not HTML pages. |
| 2 | **Webcomponent Stream** | Fragments = webcomponents. PatchElements stream to slots. |
| 3 | **Patch Integrity** | No string concatenation with PatchElements. Rama types serialize themselves. |
| 4 | **Signal Laziness** | Unknown Datastar keys = `""`. No initialization needed. |
| 5 | **Declarative Reactivity** | Datastar IS the frontend. No JS files, no build step. |
| 6 | **Single Stream Topology** | One SSE endpoint (`GET /sse`). All events flow through it. |
| 7 | **Framework-First** | Built-in Rama/Datastar types before custom code. Always. |

### General

- **SIMPELSTE LOESUNG. IMMER.** User solution is final. No overengineering.
- **Framework-first**: Use built-in Rama/Datastar paths before custom code.
- **Krebsgeschwuer-Regel**: Remove dead code immediately. Never leave commented-out code.
- **Don't repeatedly ask for permission** after approval was given.
- **Code ZUERST zeigen** (show code before writing to files).
- **Language**: User communicates in German. Respond in German when user writes German.
- **ALWAYS run `cargo test` AND `npm/vitest`** before declaring work done.

### Rama Router

- **NO `.layer()` on sub-routers in the old style.** Current pattern uses nested `with_sub_service`:
  ```rust
  Router::new_with_state(state)
      .with_sub_service("/", session_stack::session_layer(broadcaster,
          Router::new_with_state(state)
              .with_sub_service("/", auth::require_auth(
                  Router::new_with_state(state)
                      .with_get("/home", handler)
              ))
              .with_get("/sse", sse_handler)
      ))
  ```
- Sub-router + `layer.layer(router)` + `with_sub_service` pattern.
- `mod.rs` files: ONLY `pub mod` + re-exports. NEVER logic.

### Datastar v1.0.1 (CRITICAL)

- **COLONS, not hyphens**: `data-on:click`, `data-bind:signal`, `data-class:className="expr"`
- **NO `data-datastar-*` prefix**: Use `data-on:click`, NOT `data-datastar-on-click`
- **SSE events**: `datastar-patch-elements`, `datastar-patch-signals` (event types)
- **ExecuteScript**: Rama's `ExecuteScript` type, serializes as `datastar-patch-elements` with `<script>` tag. NOT a separate event type.
- **Errors**: Nested errors object. Reset with `$errors=''` (NOT `$errors={}`).

### SSE & Navigation

- **Dreifaltigkeit**: Die drei SSE-Event-Typen, mit denen der Server den Client steuert:
  1. `PatchElements` -- DOM-Fragmente in Slots einsetzen
  2. `PatchSignals` -- Datastar-Signale setzen/aendern
  3. `ExecuteScript` -- JavaScript ueber SSE ausfuehren
- **POST-Handler** (Login, Register, etc.): **303 See Other → `/sse`**. Der Client folgt dem Redirect und erhaelt die Dreifaltigkeit als SSE-Stream.
- **GET-Handler** (Navigation): Direkte SSE-Response mit der Dreifaltigkeit, kein Redirect noetig.
- **Redirects**: Via ExecuteScript ueber SSE fuer SSE-verbundene Clients, NICHT HTTP 3xx (ausser POST → 303 → /sse).
- **ver+epoch Resume**: Resume via `?v=N&e=E` Query-Parameter. Kein HMAC, keine known_hashes.

### CSS

- **Feature-based** in `css/features/`, one file per component.
- **Page CSS** (`css/pages/`) only contains `@import` statements for features.
- **One `<link>` per page** template (e.g., `home.css` for the home shell).
- **Latest CSS only**: range syntax, nesting — no vendor prefixes beyond `-webkit-mask-*`.
- **NO page-specific CSS** in feature files.
- **NO inline styles**.

### Icons

- GNOME Icon Development Kit under `references/icon-development-kit/icons/`.
- **NEVER invent placeholder icons.** If no icon exists, say so.
- Register in `base.css` as `.icon-name` with `mask-image: url("/assets/icons/name.svg")`.
- Use in HTML as: `<span class="icon icon-name"></span>`

### Shell / Layout

- App shell: `#sidebar-slot`, `#header-slot`, `#content-slot`
- **NO `data-signals` in shell.html** — signals set via SSE.
- Shell loads Datastar + registers Service Worker.
- SSE connection: `<div data-init="@get('/sse')"></div>`

### Component System

- Each component = 1 CSS file (`features/`) + 1 HTML snippet (`fragments/`).
- Behavior exclusively via Datastar attributes. No JS, no build step.
- **BEM-like naming**: `.component`, `.component--variant`, `.component.is-state`, `.component__element`
- Components nest, never merge. Container = layout, child = appearance.
- State classes via Datastar: `data-class:is-active="expr"`, `data-class:is-open="expr"`

### Logging

- **`elog!` macro** (NOT `tracing`): `elog!(Info, "message {}", arg)`, `elog!(Error, "oops")`
- Custom macro over crate `utils::log`.

---

## Key Patterns

### Fetch-SSE Contract (Navigation Handler)

Every handler follows the same pattern: extract context, build shell, emit SSE.

```rust
use crate::components::Shell;
use crate::utils::request::extract_context;

static HOME_MOVIES_HTML: &str = include_str!("../../assets/fragments/content/movies.html");

pub async fn get_home_movies(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = extract_context(&req);
    // Client fetches → Server responds with SSE PatchElements
    Shell::empty().content(HOME_MOVIES_HTML).emit_response(&ctx)
}
```

No HTML pages. No JSON. POSTs → 303 → /sse (Dreifaltigkeit). GETs → direkte SSE-Response.

### Webcomponent Stream (Fragment → Slot → PatchElements)

```rust
// A Fragment targets a DOM slot and streams as PatchElements:
Fragment::new("#content-slot", html_content)
// Fragment implements Patch trait → generates PatchElements events
// Multiple fragments compose into a Shell:
Shell::empty()
    .sidebar(Sidebar::new("overview"))   // Fragment → #sidebar-slot
    .header(header_html)                  // Fragment → #header-slot
    .content(HOME_OVERVIEW_HTML)          // Fragment → #content-slot
    .signals(r#"{"activePage":"overview"}"#)
    .emit_response(&ctx)                  // Broadcast via SSE
```

### Patch Integrity (Rama Types, Never String Concat)

```rust
// VERBOTEN — never hand-format SSE:
let bad = format!("selector {}\nmode inner\nelements {}", sel, html);

// GEBOTEN — Rama types serialize correctly:
let patch = PatchElements::new(html.try_into().unwrap())
    .with_selector(selector.try_into().unwrap());
patch.write_data(&mut buf);  // Correct serialization

// ExecuteScript — same principle:
let exec = ExecuteScript::new(NonEmptyStr::try_from(script).unwrap());
exec.write_data(&mut buf);  // NOT manual data: lines
```

### Context Extraction

```rust
// From any handler with a Request:
let ctx = extract_context(&req);
// ctx is ClientContext (per-request, from extensions)
```

---

## Pitfalls (What NOT to Do)

| Don't | Do |
|-------|-----|
| Use `tracing::info!()` / `tracing` macros | Use `elog!(Info, ...)` |
| Send full `known_hashes` list for resume | Send `?v=N&e=E` Query-Parameter (ver+epoch) |
| Put logic in `mod.rs` files | `mod.rs` = `pub mod` + re-exports only |
| Use hyphens in Datastar attrs (`data-on-click`) | Use colons (`data-on:click`) |
| Use `data-datastar-*` prefix | Use `data-on:click`, `data-bind:signal` directly |
| Reset errors with `$errors={}` | Reset with `$errors=''` |
| Invent placeholder icons | Use only GNOME Icon Kit SVGs |
| Add `data-signals` to shell.html | Set signals via SSE |
| Use HTTP redirects for SSE clients | Use ExecuteScript ueber SSE (ausser POST → 303 → /sse) |
| Generate HTML via string concatenation | Use `include_str!` for static HTML fragments |
| Use `.layer()` directly on sub-routers | Use `with_sub_service` + layer wrappers |
| Overengineer solutions | SIMPELSTE LOESUNG. IMMER. |
| Leave commented-out code | Remove immediately (Krebsgeschwuer-Regel) |
| Ask for permission repeatedly | Once approved, just do it |

---

## Architecture Notes

### Crate Separation

- **`platform-core`**: Pure domain types, NO async runtime, NO HTTP, NO Rama dependency.
- **`platform-backend`**: All HTTP/I/O, handlers, layers, SSE. Depends on `platform-core`.

### State Model

- **SharedState** (global, singleton): Config, DatabaseConnection, I18n, Arc<SseBroadcaster>. Injected via `Router::new_with_state()`.
- **ClientContext** (per-request): ClientId, SessionStorage, EventEmitter, Arc<SseBroadcaster>. Injected by layers into `req.extensions()`.

### Layer Stack Order (outer to inner)

```
CompressionLayer -> SessionStack -> AuthService -> ClientContextService -> Handler
```

### SSE Data Flow

1. Client connects: `GET /sse` -> long-lived stream
2. Navigation: `GET /home/movies` -> handler creates PatchElements -> broadcast via SseBroadcaster
3. Service Worker intercepts `/sse`, sends `?v=N&e=E` (ver+epoch) for resume
4. Server replays buffered events ab patch_ver (iter, NOT drain), liefert verpasste Events
5. Server pushes live events via `tokio::sync::broadcast` channel

### Login Flow

- Login via **email** (UNIQUE field). Username is display-only.
- POST /login -> verify password -> create session -> **303 See Other → /sse** (Dreifaltigkeit streamt den neuen State)
- Public routes bypass the auth layer stack entirely.

---

## Reference Files

All offline documentation is in `/home/jannis/Dokumente/test2/references/`:

| File | Content |
|------|---------|
| `prompt.md` | Canonical project specification (GROUND TRUTH) |
| `architecture.md` | Full architecture reference |
| `rama-api.md` | Rama 0.3.0-alpha.4 API crash course |
| `todo.md` | Phase-based implementation roadmap |
| `component-system.md` | Component system design |
| `uml-klassendiagramm-vollstaendig.md` | UML class diagram with relationships |
| `rustdoc-rama-0.3.0-alpha.4/` | Full Rama rustdoc (offline) |
| `sea-orm-1.1.19/` | SeaORM documentation |
| `rustdoc-ring-0.17.14/` | Ring crypto rustdoc |
| `datastar/` | Datastar SDK + examples |
| `icon-development-kit/` | GNOME Icon Development Kit SVGs |

**Rule**: Treat local reference material as the ONLY source of truth. If something doesn't exist: say so, don't approximate, don't substitute. No invented APIs. No assumed frameworks.
