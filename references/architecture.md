# Rama Platform — Architecture Reference

> **Stand:** Korrigiert gemäß `references/rama-api.md` (Ground Truth)
> **Canonical Spec:** `references/prompt.md`
> **Roadmap:** `references/todo.md`

---

## Systemübersicht

Fullstack-Webplattform mit:

| Schicht | Technologie |
|---------|------------|
| **Backend** | Rust + Rama 0.3.0-alpha.4 (HTTP/SSE) |
| **Frontend-Reaktivität** | Datastar SSE-only (PatchSignals, PatchElements, ExecuteScript) |
| **Caching** | Service Worker mit HMAC-SHA256 Hash-Sync (Push-Only, 16 Bytes / 128 Bit) |
| **Persistenz** | SeaORM 1.1 + SQLite |
| **Design** | Tokyo Night + GNOME/libadwaita |

---

## Crate-Struktur

```
platform/
├── Cargo.toml                    # Workspace-Root
├── Cargo.lock
├── crates/
│   ├── platform-core/            # Domäne: Typen, Config, I18n, Session
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs            # Nur: pub mod + Re-Exports
│   │       ├── client_id.rs      # UUID-Generierung + Parsing
│   │       ├── client_context.rs # ClientContext-Aggregat (inkl. EventEmitter + Arc<SseBroadcaster>)
│   │       ├── config.rs         # Singleton-Konfiguration
│   │       ├── event_emitter.rs  # EventEmitter (einfacher Buffer)
│   │       ├── buffered_event.rs # BufferedEvent (Hash + Rama-Payload)
│   │       ├── i18n.rs           # I18n-System (Lang, Lade-Logik)
│   │       └── session.rs        # SessionStorage + StorageMode
│   │
│   └── platform-backend/         # HTTP-Server + Layer + Handler
│       ├── Cargo.toml
│       ├── assets/
│       │   ├── css/              # Tokyo Night + Common CSS
│       │   │   ├── dark.css
│       │   │   ├── light.css
│       │   │   └── common.css
│       │   ├── i18n/             # de.json, en.json
│       │   └── js/               # datastar-core.js, sw.js
│       ├── pages/                # Statische HTML-Seiten (include_str!)
│       │   ├── login.html
│       │   ├── register.html
│       │   ├── home.html
│       │   ├── home_overview.html
│       │   ├── home_movies.html
│       │   ├── home_series.html
│       │   └── test.html
│       └── src/
│           ├── lib.rs            # Nur: pub mod
│           ├── main.rs           # Entrypoint (tracing init + server::run)
│           ├── server.rs         # Router<State>, SharedState, Layer-Stack via layer_fn
│           ├── context.rs        # extract_context() Utility + ClientContextSseExt Trait
│           ├── crypto.rs         # HMAC-SHA256 Hash (16 Bytes / 128 Bit)
│           ├── db/
│           │   └── mod.rs        # Datenbank-Initialisierung
│           ├── handlers/
│           │   ├── mod.rs        # Nur: pub mod + Re-Exports
│           │   ├── auth.rs       # POST /login, POST /register (Email-Login)
│           │   ├── icons.rs      # /icons/{name}.svg
│           │   ├── i18n.rs       # /api/i18n (PatchSignals)
│           │   ├── page.rs       # GET /login, /register, /home (HTML-Seiten)
│           │   ├── navigate.rs   # GET /home/movies, /home/series etc. (204 + SSE)
│           │   ├── sse.rs        # GET /sse (SSE-Stream mit Hash-Sync)
│           │   └── test.rs       # GET /test (E2E Hash-Sync Harness)
│           ├── layers/
│           │   ├── mod.rs        # Nur: pub mod + Re-Exports
│           │   ├── auth.rs       # AuthService: ClientId validieren/generieren
│           │   ├── client_context.rs  # ClientContextService: Aggregation + SseBroadcaster
│           │   └── session_storage.rs # SessionStorageService: Rehydrierung
│           └── sse/
│               ├── mod.rs        # Nur: pub mod + Re-Exports
│               └── broadcaster.rs # SseBroadcaster (tokio::sync::broadcast)
│
└── references/                   # Externe Dokumentation
    ├── prompt.md                 # Kanonische Systemspezifikation
    ├── todo.md                   # Phasen-basierte Roadmap
    ├── architecture.md           # Diese Datei
    ├── rama-api.md               # Rama-Crashkurs (Ground Truth)
    ├── rustdoc-rama-0.3.0-alpha.4/
    ├── sea-orm-1.1.19/
    ├── rustdoc-ring-0.17.14/
    ├── datastar/
    ├── icon-development-kit/
    ├── Tokyonight-dark.css
    └── Tokyonight-light.css
```

---

## Request-Lifecycle

```
Client
  │
  │  GET /login, /register, /icons/*, /assets/*, /sw.js
  ├──→ Router<State> (Public Routes)
  │      → Handler direkt (keine Layer)
  │
  │  GET /home, /home/movies, /sse, /api/i18n, /test
  └──→ Router<State> → Sub-Router (Protected Routes, via with_sub_router_make_fn)
         → CompressionLayer
           → AuthService (via layer_fn)
             → Extensions: ClientId (aus Cookie oder neu generiert)
             → Set-Cookie Header (wenn neu)
           → SessionStorageService (via layer_fn)
             → Extensions: SessionStorage (neu oder aus DB rehydriert)
           → ClientContextService (via layer_fn)
             → Extensions: ClientContext { client_id, session_storage, event_emitter, sse_broadcaster }
           → Handler (extrahiert ClientContext via extract_context(&req))
```

**Wichtig:** Keine eigenen Layer-Structs — Services werden per `rama::layer::layer_fn` als Layer gewrapped:
```rust
.layer(layer_fn(|inner| AuthService::new(inner)))
.layer(layer_fn(|inner| SessionStorageService::new(inner)))
.layer(layer_fn(|inner| ClientContextService::new(inner)))
```

---

## Layer-Implementierung (via `layer_fn`)

Jeder Service implementiert das Rama-`Service`-Trait:

```rust
use rama::service::Service;
use rama::http::{Request, Response, StatusCode};
use rama::extensions::ExtensionsMut;

pub struct AuthService<S> {
    inner: S,
}

impl<S> Service<Request> for AuthService<S>
where
    S: Service<Request, Output = Response, Error = Infallible>,
{
    type Output = Response;
    type Error = Infallible;

    async fn serve(&self, req: Request) -> Result<Self::Output, Self::Error> {
        let client_id = extract_or_generate_client_id(&req);
        let mut req = req;
        req.extensions_mut().insert(client_id);
        self.inner.serve(req).await
    }
}
```

**Komposition im Sub-Router:**
```rust
sub_router
    .with_get("/home", handlers::page::home_page)
    .with_get("/home/movies", handlers::navigate::get_home_movies)
    .with_get("/sse", handlers::sse::sse_endpoint)
    .layer(CompressionLayer::new())
    .layer(layer_fn(|inner| AuthService::new(inner)))
    .layer(layer_fn(|inner| SessionStorageService::new(inner)))
    .layer(layer_fn(|inner| ClientContextService::new(inner)))
```

---

## Datenfluss SSE / Hash-Sync

```
┌─────────────────────────────────────────────────────────────────┐
│  Service Worker (sw.js)                                          │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ fetch('/sse') intercept → URL + ?known_hashes=hash1,hash2 │  │
│  │ PatchElements Hash Registry (In-Memory Set, TTL 24h)     │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────────────┐
│  Server (handlers/sse.rs)                                        │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │ Phase 1: Replay buffered events (iter, NOT drain!)        │  │
│  │   → skip if event.hash ∈ known_hashes                     │  │
│  │ Phase 2: Push loop via broadcast::Receiver                │  │
│  │   → ring::hmac::HMAC_SHA256, 16 Bytes (128 Bit), hex      │  │
│  └───────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

**BufferedEvent (in `platform-core`):**
```rust
pub struct BufferedEvent {
    pub hash: String,  // HMAC-SHA256, 16 Bytes (128 Bit), hex-enkodiert
    pub payload: /* Rama PatchElements / PatchSignals / ExecuteScript */,
}
```

**ClientContextSseExt (in `platform-backend`):**
```rust
pub trait ClientContextSseExt {
    fn emit_patch(&self, data_to_hash: &str, patch: PatchElements, should_cache: bool);
}
```
Berechnet den HMAC-Hash, wrappt in `BufferedEvent`, sendet an `SseBroadcaster` und optional an `EventEmitter`.

---

## SeaORM-Integration

**DB-Schema (Migrationen via `sea-orm-cli`):**

### Tabelle: `users`
| Spalte | Typ | Constraints |
|--------|-----|------------|
| `id` | `Integer` | PRIMARY KEY AUTOINCREMENT |
| `username` | `String` | NOT NULL, UNIQUE |
| `email` | `String` | NOT NULL, UNIQUE |
| `password_hash` | `String` | NOT NULL |
| `created_at` | `DateTime` | NOT NULL, DEFAULT CURRENT_TIMESTAMP |
| `updated_at` | `DateTime` | NOT NULL, DEFAULT CURRENT_TIMESTAMP |

### Tabelle: `sessions`
| Spalte | Typ | Constraints |
|--------|-----|------------|
| `id` | `String` (UUID) | PRIMARY KEY |
| `user_id` | `Integer` | NOT NULL, FOREIGN KEY → users.id |
| `client_id` | `String` (UUID) | NOT NULL |
| `data` | `Json` | NOT NULL, DEFAULT '{}' |
| `expires_at` | `DateTime` | NOT NULL |
| `created_at` | `DateTime` | NOT NULL, DEFAULT CURRENT_TIMESTAMP |

**Login-Strategie:** Login erfolgt exklusiv über das `email` Feld. `username` dient nur der Anzeige im UI.

**StorageMode Mapping:** Nur `StorageMode::Persistent`-Daten werden via SeaORM in das `Json data` Feld geschrieben. `Volatile` und `FireAndForget` verbleiben im RAM.

**Migrations-Workflow:**
1. `sea-orm-cli migrate init` → Migrationen-Verzeichnis
2. `sea-orm-cli migrate generate <name>` → Neue Migration
3. `sea-orm-cli migrate up` → Migration ausführen
4. `sea-orm-cli generate entity -o src/entities` → Entity-Dateien generieren

---

## CSS-Architektur

```
dark.css          → @import tokyonight-dark.css + common.css
light.css         → @import tokyonight-light.css + common.css
common.css        → Alle Komponenten-Styles via CSS Custom Properties
tokyonight-*.css  → Nur Farb-Palette (kanonische Referenz)
```

**Design-Prinzipien:**
- CSS Custom Properties für Theming (keine CSS-in-JS)
- Media Query `prefers-color-scheme` als Default
- Uniform CSS: Ein Regelsatz pro Komponententyp
- GNOME/libadwaita Spacing-Scale
- Soft Elevation (box-shadow, kein neon)
- Floating centered window (NICHT fullscreen)
- Layout: Sidebar + Main Content + Header + Footer
