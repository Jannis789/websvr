use crate::context::SharedState;
use crate::elog;
use crate::utils::request::extract_context;
use crate::utils::response::{empty_response, html_response};
use rama::http::body::sse::datastar::{ElementPatchMode, PatchElements};
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
        let html = |id: &str, msg: &str| format!("<div class='test-info' data-marker='{}'>{}</div>", id, msg);

        // Marker via emit_element (proper patch_ver, cached per selector)
        let marker_patch =
            PatchElements::new(html("phase-a-start", "Phase A: Fresh events").try_into().unwrap())
                .with_selector("#content-slot".try_into().unwrap())
                .with_mode(ElementPatchMode::Inner);
        event_emitter.emit_element(marker_patch);

        for i in 1..=4 {
            let h = format!(
                "<div id='test-{}' class='test-pass' data-phase='A'>Fresh A: Event {}</div>",
                i, i
            );
            let patch = PatchElements::new(h.try_into().unwrap())
                .with_selector("#content-slot".try_into().unwrap())
                .with_mode(ElementPatchMode::Inner);
            event_emitter.emit_element(patch);
        }

        let end_patch = PatchElements::new(html("phase-a-end", "Phase A complete").try_into().unwrap())
            .with_selector("#content-slot".try_into().unwrap())
            .with_mode(ElementPatchMode::Inner);
        event_emitter.emit_element(end_patch);
    });

    empty_response(StatusCode::NO_CONTENT)
}

/// GET /test/1 — Emit a single PatchElement for replay protocol testing.
pub async fn test_action(State(_state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    let event_emitter = ctx.event_emitter.clone();

    let html = "<div data-marker='test-1'>Test Action 1</div>";
    let patch = PatchElements::new(html.try_into().unwrap())
        .with_selector("#content-slot".try_into().unwrap())
        .with_mode(ElementPatchMode::Inner);
    event_emitter.emit_element(patch);

    empty_response(StatusCode::NO_CONTENT)
}

/// GET /test/clear — No-op in new architecture (layer handles page generation automatically).
pub async fn test_clear(State(_state): State<SharedState>, req: Request) -> Response {
    let _ctx = extract_context(&req);
    // Layer inkrementiert page_gen bereits — kein manueller Aufruf nötig.
    empty_response(StatusCode::NO_CONTENT)
}

/// GET /test/stats — Cache diagnostics for memory leak detection.
/// Returns JSON with cached event count and current patch_ver counter.
pub async fn test_stats(State(_state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    let cached = ctx.event_emitter.cached_count();
    let ver = ctx.event_emitter.current_ver();
    let epoch = ctx.event_emitter.epoch();

    let json = serde_json::json!({
        "cached_count": cached,
        "current_ver": ver,
        "epoch": epoch,
    });

    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(json.to_string().into())
        .unwrap()
}
