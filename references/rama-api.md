
---

# Rama HTTP Framework — API Crashkurs & Snippets

> **Framework Version:** `0.3.0-alpha.4`  
> **Vollständige Referenz:** Die komplette, offline verfügbare Rustdoc befindet sich unter `references/rustdoc-rama-0.3.0-alpha.4/`.  
> **Hinweis:** Diese Datei dient als Quick-Start für die in der Architektur definierten Patterns (State, Bifurcation, Layer-Fn, SSE/Datastar).

---

## 1. Server & Router Setup (Bifurcation)

Das Herzstück des Backends. Wir nutzen `Router<State>` für die automatische State-Injektion und `with_sub_router_make_fn` für die Trennung von öffentlichen und geschützten (Layer-Stack) Routen.

```rust
use rama::http::service::web::Router;
use rama::http::server::HttpServer;
use rama::http::layer::compression::CompressionLayer;
use rama::layer::layer_fn;

// SharedState wird beim Start erstellt
let shared_state = SharedState::new(/* ... */);

let app = Router::new_with_state(shared_state.clone())
    // === PUBLIC ROUTES (Keine Layer, direkter Zugriff) ===
    .with_get("/login", handlers::page::login_page)
    .with_get("/register", handlers::page::register_page)
    .with_dir("/assets", "./assets")
    .with_dir_embed("/icons", include_dir!("$CARGO_MANIFEST_DIR/../assets/icons"))

    // === PROTECTED ROUTES (Mit Layer-Stack & Bifurcation) ===
    .with_sub_router_make_fn("/", |sub_router| {
        sub_router
            // App-Routen
            .with_get("/home", handlers::page::home_page)
            .with_get("/home/movies", handlers::navigate::get_home_movies)
            .with_get("/sse", handlers::sse::sse_endpoint)
            .with_get("/test", handlers::test::test_page)
            
            // Layer-Stack (Reihenfolge ist kritisch: außen nach innen)
            .layer(CompressionLayer::new()) // 1. Äußerste Schicht
            .layer(layer_fn(|inner| AuthService::new(inner))) // 2. Auth
            .layer(layer_fn(|inner| SessionStorageService::new(inner))) // 3. Session
            .layer(layer_fn(|inner| ClientContextService::new(inner))) // 4. Context Aggregation
    });

// Server starten
HttpServer::auto().listen("0.0.0.0:3000", app).await.unwrap();
```

---

## 2. State & Extensions (Handler-Extraktion)

Handler in Rama sind einfache `async fn`. Wir nutzen Rama-Extractors, um uns den globalen `SharedState` und den per-Request `ClientContext` zu holen.

```rust
use rama::http::service::web::extract::State;
use rama::http::{Request, Response};

pub async fn home_page(
    State(state): State<SharedState>,       // Zwingend erforderlich von Rama
    req: Request,                           // Der gesamte Request (für Extensions)
) -> Response {
    
    // Utility-Funktion statt Boilerplate oder Makros
    let ctx = extract_context(&req); 

    // Zugriff auf Config, DB, etc. via State
    let config = &state.config;
    
    // ... Handler-Logik
}
```

---

## 3. Layer via `layer_fn` (Ohne eigene Structs)

Wir implementieren nur die *Services* (z.B. `AuthService`), nicht die *Layer-Structs*. Rama's `layer_fn` macht das automatisch.

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
        // 1. ClientId aus Cookie extrahieren oder generieren
        let client_id = extract_or_generate_client_id(&req);
        
        // 2. In Request-Extensions injizieren
        let mut req = req;
        req.extensions_mut().insert(client_id);
        
        // 3. An den nächsten Layer/Handler weiterreichen
        self.inner.serve(req).await
    }
}
```

---

## 4. SSE & Datastar Integration (Clean Architecture)

Hier kommt das durchdachte Zusammenspiel aus `ClientContext`, `SseBroadcaster`, Rama-Typen und unserem `BufferedEvent` zum Zug.

### 4.1 Der ClientContext (Core) & Das Extension Trait (Backend)

**Problem:** `platform-core` darf nicht von Rama abhängen, aber wir wollen `PatchElements` direkt übergeben können.  
**Lösung:** Der `ClientContext` in `platform-core` hält den `SseBroadcaster` und den `EventEmitter`. In `platform-backend` definieren wir ein Extension Trait, das die Logik für Rama-Typen hinzufügt!

**In `platform-core`:**
```rust
pub struct ClientContext {
    pub client_id: ClientId,
    pub session_storage: SessionStorage,
    pub event_emitter: EventEmitter,
    pub sse_broadcaster: Arc<SseBroadcaster>, // Bereits hier gesetzt!
}
```

**In `platform-backend` (Das Extension Trait):**
```rust
use platform_core::{ClientContext, BufferedEvent};
use rama::http::body::sse::datastar::PatchElements;
use crate::crypto;

pub trait ClientContextSseExt {
    fn emit_patch(&self, data_to_hash: &str, patch: PatchElements, should_cache: bool);
}

impl ClientContextSseExt for ClientContext {
    fn emit_patch(&self, data_to_hash: &str, patch: PatchElements, should_cache: bool) {
        // 1. Hash berechnen (16 Bytes / 128 Bit)
        let hash = crypto::compute_content_hash(data_to_hash);

        // 2. In BufferedEvent wrappen
        let event = BufferedEvent { 
            hash, 
            payload: patch.into() 
        };

        // 3. An Broadcaster senden (der auf dem Context liegt)
        self.sse_broadcaster.broadcast(event).unwrap();
        
        // 4. Nur cachen, wenn explizit gewünscht (meistens true)
        if should_cache {
            self.event_emitter.buffer_event(event);
        }
    }
}
```

### 4.2 Navigation: Der ultimativ saubere Handler

Der Handler baut das `PatchElements`, übergibt es an den Context, und returniert sofort `204`.

```rust
use rama::http::body::sse::datastar::PatchElements;
use rama::http::{Response, StatusCode};
use crate::sse_ext::ClientContextSseExt; // Trait importieren!

static HOME_MOVIES_HTML: &str = include_str!("../../pages/home_movies.html");

pub async fn get_home_movies(
    State(_state): State<SharedState>, // Zwingend von Rama gefordert
    req: Request,
) -> Response {
    let ctx = extract_context(&req);

    // 1. PatchElements erstellen (Selector ist OPTIONAL!)
    let mut patch = PatchElements::new(HOME_MOVIES_HTML.try_into().unwrap());
    
    // Selector nur setzen, wenn nötig (Datastar default ist meist outer)
    // patch = patch.with_selector("#content".try_into().unwrap());

    // 2. Facade aufrufen: Übergibt das HTML für den Hash und das PatchElements
    // should_cache = true für Seiteninhalte, false für z.B. einmalige Signale
    ctx.emit_patch(HOME_MOVIES_HTML, patch, true);

    // 3. SOFORT 204 No Content zurückgeben
    Response::builder()
        .status(StatusCode::NO_CONTENT)
        .body(rama::http::Body::empty())
        .unwrap()
}
```

### 4.3 SSE Endpoint: Stream mit Hash-Filterung

```rust
use rama::http::body::sse::Sse;
use async_stream::stream;

pub async fn sse_endpoint(
    State(state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = extract_context(&req);

    // 1. known_hashes aus Query parsen
    let known_hashes = parse_known_hashes(&req);

    // 2. Receiver für Broadcast-Channel holen
    let mut rx = ctx.sse_broadcaster.subscribe();

    // 3. Asynchronen Stream generieren
    let stream = stream! {
        // Phase 1: Replay buffered (Iterieren, NICHT drainen!)
        for event in ctx.event_emitter.get_buffered_events() {
            if !known_hashes.contains(&event.hash) {
                yield Ok(event.try_into_sse_event());
            }
        }

        // Phase 2: Live-Events vom Channel empfangen
        while let Ok(event) = rx.recv().await {
            if !known_hashes.contains(&event.hash) {
                yield Ok(event.try_into_sse_event());
            }
        }
    };

    // 4. Response mit SSE-Stream bauen
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "text/event-stream")
        .header("cache-control", "no-cache")
        .body(Sse::new(stream).into_body())
        .unwrap()
}
```

---

## 5. Utilities & Common Patterns

### 5.1 HMAC-Hash Berechnung (16 Bytes / 128 Bit)
**WICHTIG:** Nutze *niemals* `std::hash::Hash` für Hash-Sync, da dieser randomisiert ist!

```rust
use ring::hmac;

pub fn compute_content_hash(data: &str) -> String {
    let key = hmac::Key::new(hmac::HMAC_SHA256, b"your_hmac_secret_from_env");
    let tag = hmac::sign(&key, data.as_bytes());
    
    // Auf 16 Bytes (128 Bit) kürzen für effizienteren Hash-Sync im ServiceWorker
    hex::encode(&tag.as_ref()[..16])
}
```

### 5.2 Context Extraction Utility
Kein Makro, einfach eine saubere Utility-Funktion.

```rust
use rama::http::Request;
use platform_core::ClientContext;

#[inline]
pub fn extract_context(req: &Request) -> ClientContext {
    req.extensions()
        .get::<ClientContext>()
        .cloned()
        .expect("ClientContext must be injected by Layer-Stack")
}
```

### 5.3 Redirect (303 See Other)
Wird beim Login benötigt (Post/Redirect/Get-Pattern).

```rust
use rama::http::{Response, StatusCode, header};

pub fn redirect(url: &str) -> Response {
    Response::builder()
        .status(StatusCode::SEE_OTHER) // 303
        .header(header::LOCATION, url)
        .body(rama::http::Body::empty())
        .unwrap()
}
```

### 5.4 Cookie auslesen
```rust
use rama::http::header::COOKIE;

fn get_cookie_value(req: &Request, cookie_name: &str) -> Option<String> {
    req.headers()
        .get(COOKIE)?
        .to_str().ok()?
        .split(';')
        .find_map(|c| {
            let parts: Vec<&str> = c.trim().splitn(2, '=').collect();
            if parts.len() == 2 && parts[0] == cookie_name {
                Some(parts[1].to_string())
            } else {
                None
            }
        })
}
```