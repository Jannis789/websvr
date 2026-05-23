# UML-Klassendiagramm — Rama Platform (UML 2.x)

> **Stand:** 2025-07-14  
> **Basis:** `references/architecture.md`, `references/prompt.md`, `references/todo.md`  
> **Notation:** UML 2.x — Jede Beziehung ist explizit mit ihrem UML-Typ benannt.  
> **Hinweis:** Dieses Diagramm folgt der konzeptionellen Architektur, nicht der exakten Implementierung.

---

## UML 2.x — Legende der Beziehungstypen in Mermaid.js

| UML 2.x Typ (DE) | UML 2.x Typ (EN) | Mermaid-Syntax | Semantik |
|-------------------|-------------------|----------------|----------|
| **Assoziation** | Association | `A --> B` | Strukturelle Beziehung: A kennt B, kommuniziert mit B. Keine Ownership. |
| **Aggregation** | Aggregation | `A --o B` | Schwache Ganzes-Teil-Beziehung: A enthält B, aber B kann unabhängig existieren. |
| **Komposition** | Composition | `A --* B` | Starke Ganzes-Teil-Beziehung: A ist aus B zusammengesetzt. B kann ohne A nicht existieren. |
| **Vererbung / Generalisierung** | Inheritance / Generalization | `A --\|> B` | „ist ein": A erbt von B (Subtyp ← Supertyp). |
| **Abhängigkeit** | Dependency | `A ..> B` | Nutzungsbeziehung: A benutzt B (temporär, z. B. als Parameter oder lokale Variable). |
| **Realisierung** | Realization | `A ..\|> B` | „erfüllt Vertrag": A implementiert Schnittstelle/Trait B. |

---

## 1. Gesamtarchitektur — Paketdiagramm

```mermaid
classDiagram
    direction TB

    namespace platform_core {
        class ClientId
        class Config
        class I18n
        class Lang
        class SessionStorage
        class StorageMode
        class ClientContext
        class EventEmitter
        class BufferedEvent
        class SseBroadcaster
        class AppState
        class PasswordUtil
    }

    namespace platform_backend__server {
        class SharedState
        class ClientContextSseExt
    }

    namespace platform_backend__layers {
        class AuthService~S~
        class SessionStorageService~S~
        class ClientContextService~S~
    }

    namespace platform_backend__sse {
    }

    namespace platform_backend__handlers {
        class AuthHandler
        class SseHandler
        class PageHandler
        class I18nHandler
        class IconHandler
        class NavigateHandler
    }

    namespace platform_backend__entities {
        class UserEntity
        class SessionEntity
    }

    namespace rama_http__traits {
        class Layer~S~
        class Service~Req~
    }
```

---

## 2. Domänen-Kern — platform-core (Komposition, Assoziation, Abhängigkeit)

```mermaid
classDiagram
    direction TB

    class ClientId {
        +Uuid uuid
        +new() ClientId
        +generate() ClientId
        +parse(s: str) Option~ClientId~
        +as_bytes() &[u8]
    }

    class Config {
        +String database_url
        +String host
        +u16 port
        +String rust_log
        +u32 client_id_ttl_days
        +u32 sse_ttl_days
        +global() &Config
        -from_env() Config
    }

    class Lang {
        <<enumeration>>
        De
        En
        +from_header(Option~&str~) Lang
        +as_str() &str
    }

    class I18n {
        -JsonValue de
        -JsonValue en
        +new(de_json, en_json) I18n
        +get(lang: Lang) &JsonValue
        +resolve(lang, key, db_override) Option~String~
    }

    class StorageMode {
        <<enumeration>>
        FireAndForget
        Volatile
        Persistent
    }

    class SessionStorage {
        +ClientId client_id
        +JsonValue data
        +new(client_id) SessionStorage
        +set(path, value, mode: StorageMode)
        +get(path) Option~&JsonValue~
        +set_fire_and_forget(path, value)
        +set_volatile(path, value)
        +set_persistent(path, value)
        +emit_path(path) Option~JsonValue~
        +emit_all() &JsonValue
    }

    class ClientContext {
        +ClientId client_id
        +SessionStorage session_storage
        +EventEmitter event_emitter
        +Arc~SseBroadcaster~ sse_broadcaster
        +new(client_id) ClientContext
        +with_session(client_id, session) ClientContext
    }

    class BufferedEvent {
        +String hash
        +PatchElements | PatchSignals | ExecuteScript payload
    }

    class EventEmitter {
        -Vec~BufferedEvent~ buffer
        +new() EventEmitter
        +buffer_event(event: BufferedEvent)
        +get_buffered_events() Vec~&BufferedEvent~
        +drain_all_events() Vec~BufferedEvent~
    }

    class AppState {
        +Config config
        +I18n i18n
        +DatabaseConnection db
    }

    class PasswordUtil {
        <<utility>>
        +generate_salt() Vec~u8~
        +hash_password(password, salt) String
        +hash_new(password) String
        +verify_password(password, stored) bool
    }

    class RingHmac {
        <<external crate: ring>>
        +Context
        +Key
        +Tag
        +HMAC_SHA256
        +sign(key, data)
    }

    %% ── KOMPOSITION (strong ownership, Teil kann nicht ohne Ganzes existieren) ──
    ClientContext --* ClientId : «Komposition» ClientContext besteht aus ClientId (Lebenszyklus gekoppelt)
    ClientContext --* SessionStorage : «Komposition» ClientContext besteht aus SessionStorage (Lebenszyklus gekoppelt)
    ClientContext --* EventEmitter : «Komposition» ClientContext besitzt den EventEmitter (Lebenszyklus gekoppelt)
    SessionStorage --* ClientId : «Komposition» SessionStorage gehört zu einem ClientId (wird mit ClientId identifiziert)
    SessionStorage --* StorageMode : «Komposition» StorageMode ist fester Bestandteil der SessionStorage-Struktur
    EventEmitter --* BufferedEvent : «Komposition» EventEmitter puffert BufferedEvents
    BufferedEvent --* RamaPayload : «Komposition» BufferedEvent wrappt Rama-Payload (PatchElements etc.)

    %% ── AGGREGATION (weak ownership, Teil kann unabhängig existieren) ──
    AppState --o Config : «Aggregation» AppState nutzt Config, Config existiert als Singleton unabhängig
    AppState --o I18n : «Aggregation» AppState nutzt I18n, I18n kann in anderen Kontexten existieren

    %% ── ASSOZIATION (strukturelle Beziehung, keine Ownership) ──
    I18n --> Lang : «Assoziation» I18n verwendet Lang als Schlüssel für Übersetzungen
    ClientContext --> SseBroadcaster : «Assoziation» ClientContext hält Arc<SseBroadcaster> für SSE-Broadcasts
    EventEmitter --> BufferedEvent : «Assoziation» EventEmitter gibt Events zurück (iteriert, drained sie nicht)

    %% ── ABHÄNGIGKEIT (temporäre Nutzung) ──
    I18n ..> Lang : «Abhängigkeit» Lang wird als Parameter an I18n.get() und resolve() übergeben
    PasswordUtil ..> RingHmac : «Abhängigkeit» Nutzt ring.hmac zur Passwort-Hash-Generierung
```

---

## 3. Server & Layer-Stack — platform-backend (Realisierung, Assoziation, Komposition)

```mermaid
classDiagram
    direction TB

    class SharedState {
        <<Global App State>>
        +Config config
        +DatabaseConnection db
        +I18n i18n
        +Arc~SseBroadcaster~ sse_broadcaster
    }

    class Router~State~ {
        <<rama::http::service::web::Router>>
        +new_with_state(state) Router~State~
        +with_get(path, handler_fn)
        +with_sub_router_make_fn(prefix, configure_fn)
        +layer(L) Router~State~
    }

    class HandlerModule {
        <<Rama async fn Endpoints>>
        +home_page(State, Request) Response
        +sse_handler(State, Request) Response
        Hinweis: Keine eigenen Structs noetig
        ClientContext via extract_context(&req) Utility
    }

    class CompressionLayer {
        <<rama::http::layer::compression::CompressionLayer>>
        +layer(inner: S) CompressionService~S~
    }

    class AuthService~S~ {
        -S inner
        +serve(req) Future
        +Injiziert ClientId in Extensions
        Gewrapped via rama::layer::layer_fn
    }

    class SessionStorageService~S~ {
        -S inner
        +serve(req) Future
        +Injiziert SessionStorage in Extensions
        Gewrapped via rama::layer::layer_fn
    }

    class ClientContextService~S~ {
        -S inner
        +serve(req) Future
        +Injiziert ClientContext (inkl. SseBroadcaster, EventEmitter) in Extensions
        Gewrapped via rama::layer::layer_fn
    }

    class SseBroadcaster {
        -Sender sender
        -Mutex buffers
        +new() SseBroadcaster
        +subscribe() Receiver
        +broadcast(event) Result
    }

    class Layer~S~ {
        <<rama::Layer Trait>>
        type Service
        +layer(inner: S) Service
    }

    class Service~Req~ {
        <<rama::Service Trait>>
        type Output
        type Error
        +serve(req: Req) Future
    }

    %% ── STATE & BROADCASTER (SseBroadcaster ist in platform-core definiert) ──
    SharedState --* SseBroadcaster : Komposition (SharedState hält SseBroadcaster)
    ClientContext --> SseBroadcaster : Assoziation (ClientContext hält Arc<SseBroadcaster>)
    Router~State~ --o SharedState : Aggregation new_with_state

    %% ── LAYER-STACK KETTE ──
    Router~State~ --> CompressionLayer : Assoziation
    CompressionLayer --> AuthService~S~ : Assoziation
    AuthService~S~ --> SessionStorageService~S~ : Assoziation
    SessionStorageService~S~ --> ClientContextService~S~ : Assoziation
    ClientContextService~S~ --> HandlerModule : Assoziation Ende des Stacks

    %% ── REALISIERUNG ──
    CompressionLayer ..|> Layer~S~ : Realisierung
    AuthService~S~ ..|> Service~Req~ : Realisierung
    SessionStorageService~S~ ..|> Service~Req~ : Realisierung
    ClientContextService~S~ ..|> Service~Req~ : Realisierung
    Router~State~ ..|> Service~Req~ : Realisierung
    ClientContextSseExt ..|> ClientContext : Realisierung Extension Trait in platform-backend

    %% ── ABHÄNGIGKEITEN ──
    HandlerModule ..> SharedState : Abhaengigkeit State Extractor
    HandlerModule ..> extract_context : Abhaengigkeit Utility-Funktion fuer ClientContext
    ClientContextSseExt ..> BufferedEvent : Abhaengigkeit Erstellt BufferedEvent mit Hash
```

**Anmerkungen zur Rama-Implementierung:**
*   **Bifurkation:** Die Trennung von Public- und Protected-Routes erfolgt nativ im `Router<State>` via `with_sub_router_make_fn`. Öffentliche Routen umgehen den Layer-Stack komplett.
*   **Layer-Erstellung:** Die Services (`AuthService`, `SessionStorageService`, `ClientContextService`) werden per `rama::layer::layer_fn` als Layer registriert (z.B. `.layer(layer_fn(|inner| AuthService::new(inner)))`). **Keine eigenen Layer-Structs** (`AuthLayer`, etc.) — nur die `*Service`-Implementierungen existieren.
*   **State vs. Context:** `SharedState` ist der globale App-Zustand (via `new_with_state` injiziert). `ClientContext` ist der per-Request-Zustand (wird von den Layern in `req.extensions()` injiziert) und enthält bereits `EventEmitter` + `Arc<SseBroadcaster>`.
*   **Handler:** Endpunkte sind lose `async fn` Funktionen. Sie extrahieren `SharedState` via `State(state)` und `ClientContext` via Utility-Funktion `extract_context(&req)` (nicht via `Ext`-Extractor).
*   **ClientContextSseExt:** In `platform-backend` definiertes Extension Trait, das `emit_patch()` auf `ClientContext` bereitstellt — wrappt Rama-Typen in `BufferedEvent` und broadcastet sie.


---



### Sequenzdiagramm 1: Login (`POST /login`)

```mermaid
sequenceDiagram
    autonumber
    actor Client as Browser
    participant Router as Router (Rama)
    participant Handler as AuthHandler
    participant DB as UserEntity / DB
    participant Crypto as PasswordUtil

    Client->>Router: POST /login (username, password)
    Note over Router: Public Route!<br/>Bypass Layer-Stack
    Router->>Handler: dispatch(state, req)
    
    Handler->>Handler: Parse Form Body (AuthForm)
    Handler->>DB: Find user by username
    DB-->>Handler: UserModel (password_hash)
    
    Handler->>Crypto: verify_password(input, stored_hash)
    Crypto-->>Handler: bool (valid)
    
    alt Password Valid
        Handler->>DB: Create Session (user_id, client_id)
        Handler-->>Router: 303 See Other (Location: /home, Set-Cookie: client_id)
        Router-->>Client: Redirect zu /home
    else Password Invalid
        Handler-->>Router: 303 See Other (Location: /login?error)
        Router-->>Client: Redirect zu /login
    end
```

**Kernpunkte:**
*   **Public Route:** Umgeht den Auth/Session-Layer-Stack komplett.
*   **Validierung:** Strikte Trennung von DB-Lookup und Hash-Verifikation (via `PasswordUtil`).
*   **Antwort:** Immer `303 See Other` (Post/Redirect/Get-Pattern).
    *   *Erfolg:* Redirect zu `/home` + Setzen des Session-Cookies.
    *   *Fehler:* Redirect zurück zum Login-Formular mit Error-Query.

---

### Sequenzdiagramm 2: `/home` & Sidebar-Navigation (SSE & Datastar)

```mermaid
sequenceDiagram
    autonumber
    actor User
    participant Browser as Browser / Datastar
    participant SW as ServiceWorker
    participant Router as Router (Rama)
    participant Stack as Layer-Stack<br/>(Auth -> Session -> Context)
    participant Handler as Handler<br/>(z.B. get_home_movies)
    participant Broadcaster as SseBroadcaster
    participant SseConn as SseHandler (Aktiver Stream)

    rect rgb(240, 248, 255)
    Note over SW, SseConn: Phase 1: SSE Verbindung & Hash-Sync
    SW->>Router: GET /sse?known_hashes=h1,h2,h3
    Router->>Stack: Protected Route -> Layer-Stack
    Stack->>SseConn: dispatch(state, ClientContext)
    SseConn->>SseConn: Parse known_hashes aus Query
    SseConn->>Broadcaster: subscribe() + get_buffered()
    SseConn->>SseConn: Filtere Replay-Events: skippe wenn Hash in known_hashes
    SseConn-->>SW: SSE Stream (Replay gefiltert + zukünftige Live-Events)
    end

    rect rgb(255, 250, 240)
    Note over User, SseConn: Phase 2: Dynamischer Content-Replace
    User->>Browser: Klickt "Movies" in Sidebar
    Note over Browser: Datastar feuert GET Request
    Browser->>Router: GET /home/movies
    Router->>Stack: Protected Route -> Layer-Stack
    Stack->>Handler: dispatch(state, ClientContext)
    
    Handler->>Handler: Lookup statischer HTML-String für Movies
    Note over Handler: Kein HTML generieren!<br/>Nur statischen String laden.
    
    Handler->>Broadcaster: broadcast(DatastarEvent: PatchElements, selector: #content, data: static_html)
    
    Handler-->>Router: 204 No Content
    Router-->>Browser: 204 No Content (Request ist beendet)
    
    Broadcaster-->>SseConn: Event an Receiver leiten
    SseConn-->>SW: SSE Event: datastar-patch-elements
    SW-->>Browser: Event an Datastar-Core leiten
    Note over Browser: Datastar patcht das DOM<br/>Ersetzt #content mit statischem Movies-HTML
    Browser-->>User: Sieht neuen Content (ohne Page-Reload!)
    end
```

**Kernpunkte:**
*   **Phase 1 (SSE Connect):**
    *   SW sendet `known_hashes` beim Verbindungsaufbau.
    *   Backend filtert Replay-Events exakt auf diese Hashes (verhindert redundante DOM-Patches).
*   **Phase 2 (Navigation):**
    *   GET-Request (z.B. `/home/movies`) durchläuft den Layer-Stack.
    *   Handler lädt **statisches HTML** (keine String-Generierung zur Laufzeit).
    *   Handler broadcastet `DatastarEvent` (PatchElements) an den `SseBroadcaster`.
    *   **Sofortige Antwort:** `204 No Content` (non-blocking, Request ist beendet).
    *   Asynchroner Push: Content-Update läuft isoliert über den bestehenden SSE-Stream -> Datastar patched den `#content` Selektor.
---


## 6. SeaORM-Entities — Persistenzschicht

```mermaid
classDiagram
    direction TB

    class UserModel {
        +i32 id PK
        +String username UNIQUE
        +String email UNIQUE
        +String password_hash
        +DateTime created_at
        +DateTime updated_at
    }

    class UserEntity {
        <<SeaORM EntityTrait>>
        +find() Select
        +insert() Insert
        +update() Update
        +delete() Delete
    }

    class SessionModel {
        +String id PK
        +i32 user_id FK
        +String client_id
        +Json data
        +DateTime expires_at
        +DateTime created_at
    }

    class SessionEntity {
        <<SeaORM EntityTrait>>
        +find() Select
        +insert() Insert
        +update() Update
        +delete() Delete
    }

    class DatabaseConnection {
        <<SeaORM DatabaseConnection>>
        +execute(query)
        +transaction(callback)
    }

    class DbModule {
        <<module>>
        +init(config) DatabaseConnection
    }

    class StorageMode {
        <<enumeration>>
        FireAndForget
        Volatile
        Persistent
    }

    %% ── ASSOZIATION ──
    UserEntity --> UserModel : Gibt Models zurueck
    SessionEntity --> SessionModel : Gibt Models zurueck
    UserModel --> SessionModel : 1 User hat N Sessions

    %% ── ABHÄNGIGKEIT ──
    DbModule ..> DatabaseConnection : init Factory
    UserEntity ..> DatabaseConnection : Queries
    SessionEntity ..> DatabaseConnection : Queries
    SessionModel ..> StorageMode : Speichert nur Persistent Daten im Json data Feld
```

**Anmerkungen zur Persistenzschicht:**

*   **Login-Strategie:** Der Login erfolgt exklusiv über das `email` Feld. Der `username` dient rein der Anzeige im UI.
*   **StorageMode Mapping:** Das `Json data` Feld in `SessionModel` persistiert *nur* die Werte aus dem `SessionStorage`, die mit `StorageMode::Persistent` gesetzt wurden. Werte mit `StorageMode::Volatile` (im RAM gehalten, weg bei Serverneustart) oder `FireAndForget` (einmalig emittiert) werden nicht in dieses Feld geschrieben.
*   **Keine ActiveModels:** Das ORM-spezifische `ActiveModel`-Pattern (für Change-Tracking bei Updates) wurde aus dem Architektur-Diagramm entfernt.

---



## 7. SSE / Hash-Sync — Datenfluss

```mermaid
classDiagram
    direction TB

    class ServiceWorker {
        <<Client Boundary>>
        +fetch intercept /sse
        +HashRegistry In-Memory Set
        +Haengt known_hashes an URL
    }

    class SseEndpoint {
        <<Rama async fn Endpoint>>
        +Stellt GET /sse Stream bereit
        +Phase 0: Initial Events
        +Phase 1: Replay Buffered iter NOT drain
        +Phase 2: Push via broadcast Receiver
        +Konvertiert BufferedEvent zu SSE Body
    }

    class SseBroadcaster {
        <<platform-core / Global State>>
        +tokio::sync::broadcast channel
        +subscribe() Receiver
        +broadcast(event: BufferedEvent) Result
        Hinweis: Wird in SharedState gehalten
        und als Arc in ClientContext injiziert
    }

    class BufferedEvent {
        <<platform-core>>
        +String hash HMAC_SHA256 (16 Bytes / 128 Bit)
        +Payload: Rama PatchElements oder PatchSignals oder ExecuteScript
    }

    class RingHmac {
        <<external crate: ring>>
        +HMAC_SHA256
        +sign(key, data)
    }

    %% ── ASSOZIATION ──
    ServiceWorker --> SseEndpoint : GET /sse?known_hashes
    SseEndpoint --> SseBroadcaster : subscribe und get_buffered
    SseEndpoint --> BufferedEvent : Hash-Vergleich mit known_hashes

    %% ── ABHÄNGIGKEIT ──
    SseEndpoint ..> RingHmac : compute_hash fuer HTML Strings
    SseBroadcaster ..> BufferedEvent : Verteilt und puffert Events
    NavigateHandler ..> BufferedEvent : Erstellt Event mit PatchElements und Hash
```

**Anmerkungen zur SSE / Hash-Sync Architektur:**

*   **Das `BufferedEvent` als Brücke:** Ramas native Typen (`PatchElements`, etc.) haben kein Feld für unseren Custom-Hash. Das `BufferedEvent` (in `platform-core`) ist unser architektonischer Adapter: Es verpackt den Rama-Payload zusammen mit dem HMAC-Hash (16 Bytes / 128 Bit, hex-enkodiert). Erst ganz am Ende (im `SseEndpoint`) wird es in das finale SSE-Format übersetzt.
*   **Warum HMAC und nicht `impl Hash`?:** Rusts Standard-Hasher ist randomisiert (Schutz vor Hash-DoS). Der Hash wäre nach einem Server-Neustart anders, was den Sync mit den `known_hashes` des ServiceWorkers unbrauchbar machen würde. Daher zwingend deterministischer Hash via `ring::hmac::HMAC_SHA256`.
*   **`SseBroadcaster` Definition:** Liegt in `platform-core`, da `ClientContext` (in core) einen `Arc<SseBroadcaster>` hält. Wird von `SharedState` (in backend) einmalig erstellt und als `Arc` sowohl in `SharedState` als auch in jeden `ClientContext` (via `ClientContextService`) injiziert. Nutzt `tokio::sync::broadcast` für Multi-Client-SSE.
*   **`SseEndpoint` statt Handler:** Der Name verdeutlicht: Hier geht es um eine langelebige, offene Stream-Verbindung, nicht um einen kurzen Request/Response-Cycle. Er ist der einzige Konsument, der Ramas `DatastarEvent`-Stream tatsächlich über die Leitung schreibt.
*   **Trennung von Zuständigkeit:** Der `SseBroadcaster` ist "dumm" und verteilt Events nur an alle Receiver. Die Intelligenz (Vergleich: "Kennt der SW diesen Hash schon?") liegt ausschließlich im `SseEndpoint`, bevor dieser das Event auf die Leitung legt.



---

## 8. CSS-Architektur (Abhängigkeit)

```mermaid
classDiagram
    direction TB

    class DarkCSS {
        <<dark.css>>
        @import tokyonight-dark.css
        @import common.css
    }

    class LightCSS {
        <<light.css>>
        @import tokyonight-light.css
        @import common.css
    }

    class CommonCSS {
        <<common.css>>
        Alle Komponenten-Styles
        CSS Custom Properties
        GNOME/libadwaita Spacing
        Soft Elevation (box-shadow)
    }

    class TokyoNightDark {
        <<tokyonight-dark.css>>
        Nur Farb-Palette (Dark)
    }

    class TokyoNightLight {
        <<tokyonight-light.css>>
        Nur Farb-Palette (Light)
    }

    class Browser {
        <<Client>>
        prefers-color-scheme Media Query
    }

    %% ── ABHÄNGIGKEIT (CSS @import = Dependency) ──
    DarkCSS ..> TokyoNightDark : «Abhängigkeit» @import tokyonight-dark.css (Farbpalette wird geladen)
    DarkCSS ..> CommonCSS : «Abhängigkeit» @import common.css (Komponenten-Styles)
    LightCSS ..> TokyoNightLight : «Abhängigkeit» @import tokyonight-light.css (Farbpalette wird geladen)
    LightCSS ..> CommonCSS : «Abhängigkeit» @import common.css (Komponenten-Styles)

    %% ── ASSOZIATION ──
    Browser --> DarkCSS : «Assoziation» prefers-color-scheme dark -> lädt dark.css
    Browser --> LightCSS : «Assoziation» prefers-color-scheme light -> lädt light.css
```

---


## 9. Crate-Abhängigkeitsgraph (Abhängigkeit zwischen Crates)

```mermaid
classDiagram
    direction TB

    class platform_core {
        <<crate>>
        Reine Domänentypen & Utilities
        Keine I/O
        Deps: uuid, serde_json, ring, sea-orm, chrono
    }

    class platform_backend {
        <<crate>>
        HTTP-Server + Layer + Handler + SSE
        Deps: platform-core, rama, tokio, async-stream
    }

    class rama {
        <<external crate 0.3.0-alpha.4>>
        HTTP-Server-Framework
        Router~State~, Layer, Service Traits
        Native Datastar SSE Types (PatchElements etc.)
    }

    class tokio {
        <<external crate 1.x>>
        Async Runtime
        sync::broadcast (Multi-Client-SSE via SseBroadcaster)
    }

    class ring {
        <<external crate 0.17.14>>
        HMAC-SHA256 (Hash-Sync für BufferedEvent)
        Secure Random
    }

    class sea_orm {
        <<external crate 1.1.x>>
        ORM (SQLite via sqlx-sqlite)
        Entity / Model / Migration
    }

    class serde_json {
        <<external crate 1.x>>
        JSON Serialisierung
        SessionStorage Daten (Volatile / Persistent)
    }

    class uuid_crate {
        <<external crate 1.x>>
        UUID v4 Generierung
        ClientId
    }

    class async_stream {
        <<external crate 0.3.x>>
        SSE-Stream-Generierung im SseEndpoint
    }

    %% ── ABHÄNGIGKEIT (Crate-Abhängigkeiten) ──
    platform_backend ..> platform_core : «Abhängigkeit» platform-backend hängt von platform-core ab (Domänentypen, BufferedEvent, SharedState)
    platform_backend ..> rama : «Abhängigkeit» HTTP-Server, Router~State~, Layer, native Datastar Types
    platform_backend ..> tokio : «Abhängigkeit» Async Runtime, broadcast.channel für SseBroadcaster
    platform_backend ..> async_stream : «Abhängigkeit» SSE-Stream-Erzeugung
    platform_core ..> uuid_crate : «Abhängigkeit» UUID v4 für ClientId
    platform_core ..> serde_json : «Abhängigkeit» SessionStorage JSON
    platform_core ..> ring : «Abhängigkeit» HMAC-SHA256 für BufferedEvent Hashes
    platform_core ..> sea_orm : «Abhängigkeit» DatabaseConnection Typ in AppState/SharedState
```

---

## 10. Zusammenfassung der verwendeten UML 2.x Beziehungstypen

| Beziehungstyp | Mermaid | Vorkommen im Diagramm (aktualisiert auf Rama-Native Architektur) |
|---------------|---------|-----------------------|
| **Komposition** | `A --* B` | `ClientContext --* ClientId`, `ClientContext --* SessionStorage`, `ClientContext --* EventEmitter`, `SessionStorage --* ClientId`, `SessionStorage --* StorageMode`, `EventEmitter --* BufferedEvent`, `BufferedEvent --* RamaPayload` (PatchElements etc.), `SharedState --* SseBroadcaster`, `UserModel --> SessionModel` (1 zu n DB Zusammensetzung) |
| **Aggregation** | `A --o B` | `SharedState --o Config`, `SharedState --o I18n`, `Router~State~ --o SharedState` (via `new_with_state`) |
| **Assoziation** | `A --> B` | `I18n --> Lang`, `Router~State~ --> CompressionLayer`, `CompressionLayer --> AuthService`, `AuthService --> SessionStorageService`, `SessionStorageService --> ClientContextService`, `ClientContextService --> HandlerModule`, `ServiceWorker --> SseEndpoint`, `SseEndpoint --> SseBroadcaster`, `ClientContext --> SseBroadcaster` (Arc), `UserEntity --> UserModel`, `SessionEntity --> SessionModel` |
| **Vererbung / Generalisierung** | `A --\|> B` | *(Im Projekt nicht verwendet — Rust setzt auf Traits statt Klassenvererbung)* |
| **Abhängigkeit** | `A ..> B` | `I18n ..> Lang`, `PasswordUtil ..> RingHmac`, `HandlerModule ..> SharedState` (State Extractor), `HandlerModule ..> extract_context` (Utility-Funktion), `ClientContextSseExt ..> BufferedEvent`, `NavigateHandler ..> SseBroadcaster`, `NavigateHandler ..> BufferedEvent`, `SseEndpoint ..> RingHmac`, `SseEndpoint ..> BufferedEvent`, `SseBroadcaster ..> BufferedEvent`, `DbModule ..> DatabaseConnection`, `UserEntity ..> DatabaseConnection`, `SessionEntity ..> DatabaseConnection`, `DarkCSS ..> TokyoNightDark`, `DarkCSS ..> CommonCSS`, `LightCSS ..> TokyoNightLight`, `LightCSS ..> CommonCSS`, `platform_backend ..> platform_core`, etc. |
| **Realisierung** | `A ..\|> B` | `CompressionLayer ..\|> Layer`, `AuthService ..\|> Service`, `SessionStorageService ..\|> Service`, `ClientContextService ..\|> Service`, `Router~State~ ..\|> Service` (Native Rama Implementierung), `ClientContextSseExt ..\|> ClientContext` (Extension Trait) |
