# Nachgelagerte TODOs (Post-Phase 8)

> Probleme und Verbesserungen, die während des Reviews und Tests aufgefallen sind.

## Bugs

- [ ] **Cookie wird bei jedem Request neu gesetzt** — `AuthService.is_new` prüft `req.extensions().get::<ClientId>().is_none()`, was immer `true` ist weil AuthService der Erste ist der injiziert. Fix: Prüfen ob Cookie im Request-Header vorhanden war. *(Fix in Arbeit)*

## Auth / Login

- [ ] **Password-Confirmation-Feld fehlt** — `register.html` hat kein Bestätigungsfeld für das Passwort. Der `auth::register` Handler validiert es auch nicht. Sollte `password_confirm` Feld haben und prüfen dass beide übereinstimmen.
- [ ] **Login-Handler ist ein Placeholder** — `auth::login` leitet aktuell immer nach `/home` weiter, ohne DB-Lookup oder Passwort-Verifikation. Braucht SeaORM User-Query + `PasswordUtil::verify_password`.
- [ ] **Logout löscht nur Cookie** — Die Server-Side Session in der DB wird nicht invalidated.

## Design

- [ ] **Design ist "Dreck"** — Das aktuelle UI ist rudimentär und nicht produktionsreif:
  - Floating-Window auf Login/Register wirkt nackt, kein Branding
  - Die Home-Page Sidebar hat keine aktive-State-Visualisierung
  - Karten (Overview) sehen aus wie Platzhalter
  - Responsive Design fehlt komplett (Mobile unbrauchbar)
  - Keine Lade-States, keine Transition-Animationen
  - Tokyo-Night-Palette wird nicht konsistent genutzt
  - Form-Inputs haben kein Error-Feedback
  - Header/Footer sind leer/optisch nicht ansprechend

## Architektur

- [ ] **Layer-Stack auf ALLE Routes angewendet** — Aktuell werden die Layer (Auth, Session, ClientContext) auf den gesamten `Router` angewendet, nicht nur auf den Protected Sub-Router. Public Routes wie `/login` durchlaufen unnötig den gesamten Stack.
- [ ] **SseBroadcaster hat keinen Replay-Buffer** — `SseBroadcaster::new(256)` erzeugt nur den Broadcast-Channel, aber es gibt keinen Replay-Buffer für neue SSE-Verbindungen. Phase 1 (Replay) im SSE-Handler greift auf `EventEmitter` zu, der per-Request leer ist.
- [ ] **`crypto` Modul ist nur ein Wrapper** — `crypto::hmac_secret()` delegiert nur an `Config::global()`. Könnte direkt in `compute_content_hash` integriert werden.

## Testing

- [ ] **Keine automatisierten Tests** — Jest für `sw.js` fehlt komplett. Rust-Unit-Tests nur für `password.rs`.
- [ ] **Edge Cases nicht getestet** — EC-1 bis EC-6 aus Spec §4.4 sind nicht automatisiert.
