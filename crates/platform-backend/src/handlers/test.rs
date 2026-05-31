use crate::context::SharedState;
use crate::elog;
use crate::sse::BufferedEvent;
use crate::utils::request::extract_context;
use crate::utils::response::{empty_response, html_response};
use rama::http::body::sse::datastar::{EventData, PatchElements};
use rama::http::service::web::extract::State;
use rama::http::{Request, Response, StatusCode};

/// GET /test — E2E test harness page
pub async fn test_page(State(_state): State<SharedState>, req: Request) -> Response {
    let _ctx = extract_context(&req);
    html_response(include_str!("../../assets/templates/test.html"))
}

/// GET /test/auth — Mark the session as authenticated (for E2E tests).
pub async fn test_auth(State(_state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    {
        let mut session = ctx.session_storage.lock().await;
        session.set_volatile("authenticated", serde_json::Value::Bool(true));
    }
    elog!(Info, "Test → authenticated session for {}", ctx.client_id);
    empty_response(StatusCode::NO_CONTENT)
}

/// GET /test/run — Execute the automated state replay test
pub async fn test_run(State(_state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    let event_emitter = ctx.event_emitter.clone();
    elog!(Info, "Test → running state replay test");

    tokio::spawn(async move {
        let secret = event_emitter.secret().to_string();

        // Phase A: Fresh events
        let _ = event_emitter.broadcast(marker_event("phase-a-start", "Phase A: Fresh events", &secret));

        for i in 1..=4 {
            let html = format!(
                "<div id='test-{}' class='test-pass' data-phase='A'>Fresh A: Event {}</div>",
                i, i
            );
            let patch = PatchElements::new(html.try_into().unwrap())
                .with_selector("#content-slot".try_into().unwrap());
            event_emitter.emit_element(patch);
        }

        let _ = event_emitter.broadcast(marker_event("phase-a-end", "Phase A complete", &secret));

        // Phase B: State replay (get_state returns last event per slot)
        let _ = event_emitter.broadcast(marker_event("phase-b-start", "Phase B: State replay", &secret));
        for event in event_emitter.get_state() {
            let _ = event_emitter.broadcast(event);
        }
        let _ = event_emitter.broadcast(marker_event("phase-b-end", "Phase B complete", &secret));

        // Completion
        let _ = event_emitter.broadcast(marker_event(
            "test-done",
            &format!("Done. {} events in state cache.", event_emitter.cached_count()),
            &secret,
        ));
    });

    empty_response(StatusCode::NO_CONTENT)
}

fn marker_event(id: &str, msg: &str, secret: &str) -> BufferedEvent {
    let html = format!("<div class='test-info' data-marker='{}'>{}</div>", id, msg);
    let patch =
        PatchElements::new(html.try_into().unwrap()).with_selector("#content-slot".try_into().unwrap());
    BufferedEvent::new(EventData::PatchElements(patch), secret)
}
