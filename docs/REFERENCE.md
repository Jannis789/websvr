# Rama Platform — Synthetisierte Referenz

> KI-optimierte Konsolidierung aller Referenzmaterialien.
> Quellen: prompt.md, architecture.md, rama-api.md, todo.md, uml-klassendiagramm.md,
> rustdoc-rama-0.3.0-alpha.4/, sea-orm-1.1.19/, datastar/, rustdoc-ring-0.17.14/

---

## 1. Tech-Stack

| Schicht | Technologie | Version |
|---------|------------|---------|
| Backend | Rust + Rama HTTP | 0.3.0-alpha.4 |
| Frontend | Datastar SSE-only | 1.0.1 |
| ORM | SeaORM + SQLite | 1.1.x |
| Crypto | ring (HMAC-SHA256) | 0.17.14 |
| Runtime | tokio (broadcast) | 1.x |
| Design | Tokyo Night + GNOME/libadwaita | — |

---

## 2. Crate-Struktur

```
Cargo.toml (workspace)
├── platform-core/     — Domäne: Typen, Config, I18n, Session
│   └── KEINE rama-Abhängigkeit
└── platform-backend/  — HTTP, Router, Layer, Handler, SSE
    ├── assets/
    │   ├── css/        dark.css, light.css, common.css
    │   ├── i18n/       de.json, en.json
    │   ├── js/         datastar-core.js, sw.js
    │   └── templates/  *.html (include_str!)
    ├── migration/      SeaORM Migrationen
    └── src/
        ├── main.rs     tracing init + server::run
        ├── server.rs   Router<State>, SharedState, Layer-Stack
        ├── context.rs  extract_context() + ClientContextSseExt
        ├── crypto.rs   HMAC-SHA256 (16 Bytes / 128 Bit)
        ├── handlers/   page, navigate, auth, sse, test, i18n, icons
        ├── layers/     auth, session_storage, client_context
        └── sse/        broadcaster.rs (tokio::sync::broadcast)
```

---

## 3. Rama API — Essentials

### Server & Router

```rust
use rama::http::service::web::Router;
use rama::http::server::HttpServer;
use rama::layer::layer_fn;

let app = Router::new_with_state(shared_state)
    // Public: keine Layer
    .with_get("/login", handlers::page::login_page)
    .with_dir("/assets", "./assets")
    // Protected: Sub-Router mit Layer-Stack
    .with_sub_router_make_fn("/", |sub| {
        sub
            .with_get("/home", handlers::page::home_page)
            .with_get("/home/movies", handlers::navigate::get_home_movies)
            .with_get("/sse", handlers::sse::sse_endpoint)
            .layer(CompressionLayer::new())
            .layer(layer_fn(|i| AuthService::new(i)))
            .layer(layer_fn(|i| SessionStorageService::new(i)))
            .layer(layer_fn(|i| ClientContextService::new(i)))
    });

HttpServer::auto().listen("0.0.0.0:3000", app).await
```

### Handler-Signatur

```rust
use rama::http::service::web::extract::State;

pub async fn handler(State(state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    // ...
}
```

### Layer via layer_fn (keine eigenen Structs!)

```rust
use rama::service::Service;

pub struct AuthService<S> { inner: S }

impl<S: Service<Request, Output = Response, Error = Infallible>> Service<Request> for AuthService<S> {
    type Output = Response;
    type Error = Infallible;
    async fn serve(&self, req: Request) -> Result<Self::Output, Self::Error> {
        let mut req = req;
        req.extensions_mut().insert(/* ... */);
        self.inner.serve(req).await
    }
}
// Verwendung: .layer(layer_fn(|inner| AuthService::new(inner)))
```

### Rama-Crate-Pfade (Index)

| Konzept | Pfad |
|---------|------|
| Router mit State | `rama::http::service::web::Router<State>` |
| GET-Route | `Router::with_get("/path", handler)` |
| Sub-Router | `Router::with_sub_router_make_fn("/prefix", \|r\| r)` |
| State Extractor | `rama::http::service::web::extract::State` |
| Request-Extensions | `req.extensions().get::<T>()` / `req.extensions_mut().insert()` |
| Layer Trait | `rama::service::Layer` |
| Service Trait | `rama::service::Service` |
| layer_fn | `rama::layer::layer_fn` |
| Compression | `rama::http::layer::compression::CompressionLayer` |
| SSE Body | `rama::http::body::sse::Sse` |
| PatchElements | `rama::http::body::sse::datastar::PatchElements` |
| PatchSignals | `rama::http::body::sse::datastar::PatchSignals` |
| ExecuteScript | `rama::http::body::sse::datastar::ExecuteScript` |
| HTTP Status | `rama::http::StatusCode` |
| Response Builder | `rama::http::Response::builder()` |

---

## 4. SSE & Hash-Sync System

### Protokoll (Push-Only)

1. **SW intercepted** `fetch('/sse')` → hängt `?known_hashes=h1,h2,...` an URL
2. **Server Phase 1 (Replay):** Iteriert `event_emitter.get_buffered_events()`, skippt bekannte Hashes
3. **Server Phase 2 (Push):** `broadcast::Receiver` für Live-Events
4. **SW:** Speichert PatchElements-Hashes im In-Memory Set (TTL 24h)

### BufferedEvent

```rust
pub struct BufferedEvent {
    pub hash: String,    // HMAC-SHA256, auf 16 Bytes gekürzt, hex-enkodiert
    pub payload: /* Rama PatchElements/PatchSignals/ExecuteScript */,
}
```

### Navigation-Handler (204 + SSE)

```rust
static HTML: &str = include_str!("../../assets/templates/home_movies.html");

pub async fn get_home_movies(State(_): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    let patch = PatchElements::new(HTML.try_into().unwrap());
    ctx.emit_patch(HTML, patch, true);
    Response::builder().status(StatusCode::NO_CONTENT)
        .body(rama::http::Body::empty()).unwrap()
}
```

### HMAC-Hash (ring)

```rust
use ring::hmac;

pub fn compute_content_hash(data: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, HMAC_SECRET.as_bytes());
    let tag = hmac::sign(&key, data.as_bytes());
    hex::encode(&tag.as_ref()[..16])  // 128 Bit
}
```

WICHTIG: Niemals `std::hash::Hash` verwenden (randomisiert über Programstarts).

### Edge Cases

| Case | Verhalten |
|------|-----------|
| Hash Match | Event überspringen |
| Hash Mismatch | Event senden |
| Out-of-order | Jedes Event einzeln gegen known_hashes prüfen |
| SW verliert Hashes | Leere known_hashes → Server sendet alle |
| TTL > 24h | Hash gelöscht, Event wird neu verarbeitet |

---

## 5. State-Architektur

### SharedState (Global, Server-Scope)

```rust
pub struct SharedState {
    pub config: Config,
    pub db: DatabaseConnection,
    pub i18n: I18n,
    pub sse_broadcaster: Arc<SseBroadcaster>,
}
// Via Router::new_with_state() injiziert
```

### ClientContext (Per-Request)

```rust
pub struct ClientContext {
    pub client_id: ClientId,
    pub session_storage: SessionStorage,
    pub event_emitter: EventEmitter,
    pub sse_broadcaster: Arc<SseBroadcaster>,
}
// Via Layer-Stack in req.extensions() injiziert
```

### Layer-Reihenfolge (kritisch)

```
CompressionLayer → AuthService → SessionStorageService → ClientContextService → Handler
```

| Layer | Input | Output |
|-------|-------|--------|
| AuthService | Cookie-Header | `ClientId` in Extensions |
| SessionStorageService | `ClientId` | `SessionStorage` in Extensions |
| ClientContextService | `ClientId` + `SessionStorage` | `ClientContext` in Extensions |

---

## 6. SessionStorage & EventEmitter

### SessionStorage

- JSON-basiert pro Client
- 3 Modi: `FireAndForget`, `Volatile`, `Persistent`
- Nur `Persistent` wird via SeaORM in DB geschrieben
- Selektives Senden via `emit_path` (JSON-Path)

### EventEmitter

- Einfacher Puffer: `buffer_event()`, `get_buffered_events()`, `drain_all_events()`
- Kein Namespace, kein Hash-Tracking

---

## 7. SeaORM — DB-Schema & Patterns

### Schema

**users:**
| Spalte | Typ | Constraints |
|--------|-----|------------|
| id | Integer | PK AUTOINCREMENT |
| username | String | NOT NULL, UNIQUE (nur Anzeige) |
| email | String | NOT NULL, UNIQUE (Login-Identifier) |
| password_hash | String | NOT NULL |
| created_at | DateTime | DEFAULT CURRENT_TIMESTAMP |
| updated_at | DateTime | DEFAULT CURRENT_TIMESTAMP |

**sessions:**
| Spalte | Typ | Constraints |
|--------|-----|------------|
| id | String (UUID) | PK |
| user_id | Integer | FK → users.id |
| client_id | String (UUID) | NOT NULL |
| data | Json | DEFAULT '{}' |
| expires_at | DateTime | NOT NULL |
| created_at | DateTime | DEFAULT CURRENT_TIMESTAMP |

### Wichtige SeaORM-Pfade

```rust
// DB-Verbindung
sea_orm::Database::connect("sqlite://platform.db?mode=rwc").await

// Entity Traits
sea_orm::EntityTrait   // = Entity
sea_orm::ModelTrait    // = Model (gelesen)
sea_orm::ActiveModelTrait  // = ActiveModel (schreiben)

// Query-Beispiele
Entity::find_by_id(id).one(&db).await
Entity::insert(active_model).exec(&db).await
Entity::update(active_model).exec(&db).await
Entity::delete_by_id(id).exec(&db).await

// Conditions
Entity::find().filter(Column::Email.eq(email)).one(&db).await
```

### Migrations-Workflow

```bash
sea-orm-cli migrate init          # Verzeichnis erstellen
sea-orm-cli migrate generate <n>  # Neue Migration
sea-orm-cli migrate up            # Ausführen
sea-orm-cli generate entity -o src/entities  # Entities generieren
```

---

## 8. Datastar — Frontend-Integration

### SSE-Event-Typen

| Typ | Rama-Typ | Zweck |
|-----|----------|-------|
| PatchElements | `PatchElements::new(html)` | DOM-Fragment ersetzen |
| PatchSignals | `PatchSignals::new(json)` | Reactive State updaten |
| ExecuteScript | `ExecuteScript::new(js)` | JS ausführen |

### HTML-Attribute (Datastar v1.0.1)

```html
<input data-bind:title />
<div data-text="$title.toUpperCase()"></div>
<button data-on:click="@post('/endpoint')">Save</button>
<div data-on-signal-patch="console.log($patch)"></div>
<div data-show="$isLoggedIn"></div>
<div data-class="{'active': $isActive}"></div>
```

### Signal-System

- `JSONPatch` = `Record<string, any>` (Typed als JS Object, nicht JSON-Patch-ops)
- `DATASTAR_SIGNAL_PATCH_EVENT` wird bei jeder Signal-Änderung gefeuert
- Batch-Modus: `beginBatch()` / `endBatch()` verhindert mehrfaches Triggern

---

## 9. Config / Environment

Singleton via `std::sync::OnceLock`:

| Variable | Default | Zweck |
|----------|---------|-------|
| DATABASE_URL | `sqlite://platform.db?mode=rwc` | SQLite-Pfad |
| HOST | `0.0.0.0` | Server-Host |
| PORT | `3000` | Server-Port |
| RUST_LOG | `info` | Tracing-Level |
| CLIENT_ID_TTL_DAYS | `30` | ClientId-Cookie-TTL |
| SSE_TTL_DAYS | `—` | SSE-TTL |
| HMAC_SECRET | — | HMAC-Signing-Key |

---

## 10. Implementierungs-Regeln (ADR)

| # | Regel | Begründung |
|---|-------|-----------|
| ❌0 | KEINE Logik in mod.rs | Nur `pub mod` + Re-Exports |
| ❌1 | KEINE String-Concatenation für UI | `include_str!` + PatchElements |
| ❌2 | KEINE eigenen Layer-Structs | `layer_fn` nutzen |
| ❌3 | KEINE undokumentierten Side Effects | Alles via EventEmitter/SSE |
| ❌4 | KEIN page-spezifisches CSS | Global `common.css` + Custom Properties |
| ❌5 | KEIN eigenes JS für UI-State | Nur Datastar SSE |
| ❌6 | KEIN `std::hash::Hash` für Hash-Sync | `ring::hmac` für Determinismus |
| ❌7 | KEINE synchronen Wartezeiten bei SSE | Sofort `204 No Content` |
| ADR-010 | Login via Email | Email = Login-Identifier, Username = Anzeige |
| ADR-013 | `ClientContextSseExt` in Backend | Trennt Rama-Abhängigkeit von Core |
| ADR-014 | Hash auf 128 Bit kürzen | Effizienter Hash-Sync im SW |

---

## 11. ClientId-System

```rust
pub struct ClientId(pub uuid::Uuid);  // UUID v4
pub const CLIENT_ID_COOKIE: &str = "platform_cid";
// Cookie: HttpOnly, SameSite=Lax, Path=/
// TTL: 30 Tage (Remember-Me: 365 Tage)
// Niemals Request ablehnen → immer generieren
```

---

## 12. I18N

- Dateien: `assets/i18n/{de,en}.json`
- Signal: `$i18n.<key>` via PatchSignals
- Erkennung: `Accept-Language` Header → `Lang::from_header()`
- Override: DB-basiert (spätere Phase)

---

## 13. CSS-Architektur

```
dark.css  → @import tokyonight-dark.css + common.css
light.css → @import tokyonight-light.css + common.css
common.css → Alle Komponenten via CSS Custom Properties
```

- `prefers-color-scheme` als Default
- Soft Contrast (kein Neon, kein Cyberpunk)
- GNOME/libadwaita Spacing-Scale
- Floating centered window (nicht fullscreen)
- Layout: Sidebar + Main + Header + Footer

---

## 14. Utilities

### Context Extraction

```rust
pub fn extract_context(req: &Request) -> ClientContext {
    req.extensions().get::<ClientContext>().cloned()
        .expect("ClientContext must be injected by Layer-Stack")
}
```

### Redirect (303)

```rust
pub fn redirect(url: &str) -> Response {
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header(header::LOCATION, url)
        .body(rama::http::Body::empty()).unwrap()
}
```

### Cookie auslesen

```rust
fn get_cookie_value(req: &Request, name: &str) -> Option<String> {
    req.headers().get(COOKIE)?.to_str().ok()?
        .split(';').find_map(|c| {
            let p: Vec<&str> = c.trim().splitn(2, '=').collect();
            if p.len() == 2 && p[0] == name { Some(p[1].to_string()) } else { None }
        })
}
```

---

## 15. Implementierungs-Status

Phase 0 (Setup): ✅ komplett
Phase 1 (Core): ✅ komplett (Config, ClientId, SessionStorage, EventEmitter, BufferedEvent, I18n)
Phase 2 (Persistence): ✅ komplett (SeaORM Entities, Migrationen)
Phase 3 (Server + Layer): ✅ komplett (Router, Layer-Stack, SSE, Broadcaster)
Phase 4 (Handler): ✅ komplett (Page, Navigate, Auth, SSE, Test, I18n, Icons)
Phase 5 (Frontend): 🔄 in Arbeit (HTML-Seiten, SW, Datastar-Integration)
Phase 6 (Testing): 🔄 in Arbeit (E2E Hash-Sync Verification)
