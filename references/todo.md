Hier ist ein umfassendes, strukturiertes TODO-List, das sich direkt aus den Vorgaben der `prompt.md` ableitet. Es ist in logische Implementierungsphasen und Komponenten unterteilt, damit nichts vergessen wird.

---

# 🚀 Rama Platform — Master TODO List

> **Referenz:** Alle Punkte beziehen sich auf die `PROJECT SPECIFICATION — Rama Platform (v1.1.0)`.

## Phase 0: Projekt-Setup & Infrastruktur

- [ ] **[Spec §1.3]** Cargo Workspace einrichten (`platform-core`, `platform-backend`).
- [ ] **[Spec §1.3]** Abhängigkeiten in `Cargo.toml` exakt gemäß Vorgabe eintragen (Rama 0.3.0-alpha.4, SeaORM 1.1.x, ring 0.17.14, etc.).
- [ ] **[Spec §1.3]** Sicherstellen, dass `platform-core` **KEINE** Abhängigkeit auf `rama` hat (Strict Separation of Concerns).
- [ ] **[Spec §2.7]** SQLite Datenbank-Datei konfigurieren (`DATABASE_URL="sqlite://platform.db?mode=rwc"`).
- [ ] **[Spec §2.7]** `sea-orm-cli` installieren und initialen Migrations-Ordner erstellen.
- [ ] **[Spec §❌0]** Durchgang: Sicherstellen, dass keine Logik in `mod.rs` Dateien liegt (nur `pub mod` + Re-Exports).

## Phase 1: Domain Core (`platform-core`)

- [ ] **[Spec §2.11]** `Config` Struct implementieren (Singleton via `std::sync::OnceLock`, Env-Vars: `DATABASE_URL`, `HOST`, `PORT`, `RUST_LOG`, `CLIENT_ID_TTL_DAYS`, `SSE_TTL_DAYS`, `HMAC_SECRET`).
- [ ] **[Spec §2.1]** `ClientId` Struct implementieren (Wrapper für `uuid::Uuid`), inkl. `generate()` und `parse()`.
- [ ] **[Spec §2.5]** Enum `StorageMode` erstellen (`FireAndForget`, `Volatile`, `Persistent`).
- [ ] **[Spec §2.5]** `SessionStorage` implementieren (JSON-basiert, Methoden: `set`, `get`, `set_persistent`, `set_volatile`, `set_fire_and_forget`, `emit_path`, etc.).
- [ ] **[Spec §2.4]** `ClientContext` Struct erstellen (Aggregat aus `ClientId` + `SessionStorage` + `EventEmitter` + `Arc<SseBroadcaster>`).
- [ ] **[Spec §2.6]** `BufferedEvent` Struct erstellen (Felder: `hash: String`, `payload: RamaDatastarType`).
- [ ] **[Spec §2.5]** `EventEmitter` implementieren (einfacher Event-Puffer, KEIN Namespace, KEIN Hash-Tracking: `buffer_event(event: BufferedEvent)`, `get_buffered_events()`, `drain_all_events()`).
- [ ] **[Spec §2.8]** `I18n` Struct implementieren (Lädt `assets/i18n/{de,en}.json`, `get(lang, key)`, `resolve()`).
- [ ] **[Spec §2.8]** Enum `Lang` erstellen (`De`, `En`, `from_header()`).

## Phase 2: Persistence (`platform-backend` + SeaORM)

- [ ] **[Spec §2.7]** SeaORM Entity `UserEntity` generieren/erstellen.
- [ ] **[Spec §2.7 / ADR-010]** `UserModel` sicherstellen: Enthält `email: String (UNIQUE)` für Login und `username: String (UNIQUE)` für Anzeige.
- [ ] **[Spec §2.7]** SeaORM Entity `SessionEntity` generieren/erstellen.
- [ ] **[Spec §2.7]** `SessionModel` sicherstellen: Enthält `data: Json` für `StorageMode::Persistent` Daten.
- [ ] **[Spec §2.7]** Migration ausführen und testen.

## Phase 3: HTTP Server, State & Layer-Stack (`platform-backend`)

- [ ] **[Spec §2.4]** `SharedState` Struct erstellen (enthält `Config`, `DatabaseConnection`, `I18n`, `Arc<SseBroadcaster>`).
- [ ] **[Spec §2.3 / ADR-008]** Rama `Router::new_with_state(shared_state)` initialisieren.
- [ ] **[Spec §2.3]** Public Routes direkt am Haupt-Router registrieren (`/login`, `/register`, `/icons/*`, `/assets/*`, `/sw.js`).
- [ ] **[Spec §2.3]** Sub-Router für Protected Routes via `with_sub_router_make_fn("/", ...)` anlegen.
- [ ] **[Spec §2.2 / ADR-009]** `AuthService` implementieren (Kein eigenes Layer-Struct, Nutzung von `rama::layer::layer_fn`). Logik: ClientId aus Cookie oder generieren, in Extensions injecten.
- [ ] **[Spec §2.2 / ADR-009]** `SessionStorageService` implementieren (via `layer_fn`). Logik: Liest ClientId, lädt/erstellt SessionStorage, in Extensions injecten.
- [ ] **[Spec §2.2 / ADR-009]** `ClientContextService` implementieren (via `layer_fn`). Logik: Aggregiert ClientId + SessionStorage + EventEmitter + Arc<SseBroadcaster>, in Extensions injecten.
- [ ] **[ADR-013]** `ClientContextSseExt` Extension Trait in `platform-backend` implementieren. Methode: `emit_patch(data_to_hash, patch: PatchElements, should_cache: bool)` — berechnet HMAC-Hash, erstellt `BufferedEvent`, sendet an `SseBroadcaster` und optional an `EventEmitter`.
- [ ] **[Spec §2.2]** `CompressionLayer` als äußerste Schicht im Sub-Router registrieren.
- [ ] **[Spec §2.2]** Reihenfolge im Sub-Router testen: `Compression -> Auth -> Session -> Context -> Handler`.
- [ ] **[Spec §2.4]** `extract_context(&req)` Utility-Funktion in `platform-backend` implementieren (liest `ClientContext` aus `req.extensions()`).

## Phase 4: SSE, Broadcaster & Hash-Sync (Kritisch)

- [ ] **[Spec §2.6]** `SseBroadcaster` implementieren (Wrappt `tokio::sync::broadcast`, Methoden: `subscribe()`, `broadcast()`, Puffer-Logik).
- [ ] **[Spec §2.6 / ADR-004 / ADR-014]** Hash-Generierung mit `ring::hmac::HMAC_SHA256` implementieren. Tag auf 16 Bytes (128 Bit) kürzen, hex-enkodieren. **[WICHTIG: NIEMALS `std::hash::Hash` nutzen!]**
- [ ] **[Spec §2.6]** `SseEndpoint` (async fn) für `GET /sse` implementieren.
- [ ] **[Spec §2.6]** SSE Phase 1 implementieren: `known_hashes` aus Query String parsen, gebufferte Events iterieren (NOT drain!), Hashes vergleichen, überspringen wenn bekannt.
- [ ] **[Spec §2.6]** SSE Phase 2 implementieren: Live-Events via `broadcast::Receiver` empfangen und in SSE-Stream schreiben.
- [ ] **[Spec §2.6]** Konvertierung von internem `BufferedEvent` in Rama SSE Body (`PatchElements`, etc.) implementieren.

## Phase 5: Handler & UI Logik

- [ ] **[Spec §2.7]** `AuthHandler` (Login via Email, POST /login, Passwort via `ring` verifizieren, Session erstellen, 303 Redirect).
- [ ] **[Spec §2.9]** Statische HTML-Seiten als `include_str!` anlegen (`login.html`, `register.html`, `home.html`, etc.).
- [ ] **[Spec §2.9 / ADR-011]** `NavigateHandler` für `/home/overview`, `/home/movies`, `/home/series` erstellen. Logik: Statisches HTML laden -> `BufferedEvent(PatchElements)` erstellen -> an `SseBroadcaster` senden -> **`204 No Content`** returnen.
- [ ] **[Spec §2.8]** `I18nHandler` erstellen (liefert Übersetzungen via PatchSignals).
- [ ] **[Spec §2.10]** `IconHandler` für `/icons/{name}.svg` erstellen (SVG via `include_str!`).
- [ ] **[Spec §❌1 / ❌2]** Durchgang: Sicherstellen, dass nirgendwo String-Concatenation für UI oder Datastar-Events verwendet wird.

## Phase 6: Frontend (Service Worker & Datastar)

- [ ] **[Spec §2.6]** `sw.js` schreiben: Intercepted `fetch('/sse')`.
- [ ] **[Spec §2.6]** Logik in `sw.js`: `known_hashes` aus In-Memory Set an URL hängen.
- [ ] **[Spec §2.6]** Hash Registry in `sw.js`: Nur Hashes von `PatchElements` Events speichern (TTL 24h).
- [ ] **[Spec §2.9]** CSS-Architektur aufbauen: `dark.css`, `light.css`, `common.css` (Tokyo Night, libadwaita Spacing).
- [ ] **[Spec §2.9]** Datastar Core integrieren und Signal-Store initialisieren.
- [ ] **[Spec §2.9]** Sidebar + Main Content Layout in `home.html` umsetzen.

## Phase 7: Testing & E2E Harness

- [ ] **[Spec §2.12]** Jest-Testumgebung für `sw.js` aufsetzen.
- [ ] **[Spec §2.12]** Unit Tests für SW Hash Registry schreiben (TTL, Deduplizierung).
- [ ] **[Spec §2.12]** E2E Test-Route `/test` im Backend anlegen (Protected Sub-Router).
- [ ] **[Spec §2.12]** Test-Seite (`test.html`) mit Datastar-Signals für Test-Ergebnisse bauen.
- [ ] **[Spec §2.12]** Backend-Logik für `/test`: Sequentielles Feuern von `BufferedEvents` (Edge Cases: neue, bekannte, out-of-order Events) in den Broadcaster.
- [ ] **[Spec §4.4]** Edge Cases implementieren & testen: EC-1 (Hash Match), EC-2 (Mismatch), EC-5 (SW verliert Hashes), EC-6 (TTL Überschreitung).
- [ ] **[Spec §2.12]** Score-Berechnung im Frontend umsetzen (z.B. `10/10 Caching-Kombinationen`).

## Phase 8: Final Review & Restriktionen-Check

- [ ] **[Spec §❌3]** Side-Effect Check: Werden State-Changes immer über EventEmitter/SSE abgewickelt?
- [ ] **[Spec §❌4]** CSS Check: Gibt es page-spezifisches CSS? Wenn ja, nach `common.css` mit Custom Properties verschieben.
- [ ] **[Spec §❌5]** JS Check: Gibt es eigenes JS für UI-State? Entfernen, nur Datastar SSE nutzen.
- [ ] **[Spec §❌7]** Response-Check: Returnieren SSE-Trigger-GETs konsequent `204 No Content`?