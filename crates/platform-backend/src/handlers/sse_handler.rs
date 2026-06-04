use crate::elog;

use crate::context::SharedState;
use crate::utils::request::extract_context;
use async_stream::stream;
use rama::http::body::sse::datastar::EventData;
use rama::http::service::web::extract::State;
use rama::http::service::web::response::{IntoResponse, Sse};
use rama::http::sse::server::KeepAlive;
use rama::http::sse::{Event, EventBuildError};
use rama::http::{Request, Response};

type SseResult = Result<Event<EventData>, EventBuildError>;

/// Extract a query parameter by key. Returns 0 if absent/invalid.
fn query_u64(req: &Request, key: &str) -> u64 {
    req.uri()
        .query()
        .and_then(|q| q.split('&').find_map(|p| p.strip_prefix(&format!("{key}="))))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// Build an id-only SSE event (no data, no event type — just `id: <ver>`).
/// The SW recognizes this and replays from its local cache.
fn id_only_event(ver: u64) -> SseResult {
    Event::new().try_with_id(ver.to_string())
}

/// GET /sse — SSE endpoint with state replay on reconnect.
///
/// Client sends `?v=N&e=E` (highest patch_ver + epoch seen by SW).
/// - Epoch mismatch (server restart) → all events sent as full WITH `id:`.
/// - Events the SW already has cached → send `id: <ver>` only.
/// - Events with new content → send full SSE event WITH `id:`.
/// - Live events (new broadcasts) → send WITHOUT `id:`.
pub async fn sse_endpoint(State(_state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    let client_ver = query_u64(&req, "v");
    let client_epoch = query_u64(&req, "e");
    let server_epoch = ctx.event_emitter.epoch();

    elog!(
        Debug,
        "SSE → connected (client_id={}, ver={}, epoch={}/{})",
        ctx.client_id,
        client_ver,
        client_epoch,
        server_epoch
    );

    // Atomically subscribe + build replay plan to prevent race condition
    let (mut rx, plan) = ctx.event_emitter.subscribe_and_plan(client_ver, client_epoch);
    let (id_only, full) = plan.into_parts();

    let stream = stream! {
        if !id_only.is_empty() {
            elog!(Debug, "SSE → {} id-only replays since ver={}", id_only.len(), client_ver);
        }
        for ver in &id_only {
            match id_only_event(*ver) {
                Ok(sse_event) => yield Ok::<_, EventBuildError>(sse_event),
                Err(e) => elog!(Error, "SSE → failed to build id-only event: {}", e),
            }
        }

        if !full.is_empty() {
            elog!(Debug, "SSE → {} full replays since ver={}", full.len(), client_ver);
        }
        for event in &full {
            // Replay events: WITH id (SW caches by patch_ver)
            match event.to_sse_event_with_id() {
                Ok(sse_event) => yield Ok::<_, EventBuildError>(sse_event),
                Err(e) => elog!(Error, "SSE → failed to serialize replay: {}", e),
            }
        }

        loop {
            match rx.recv().await {
                // Live events: MIT id:, damit SW alles cached.
                // Der Replay filtert trotzdem per request_gen.
                Ok(event) => match event.to_sse_event_with_id() {
                    Ok(sse_event) => yield Ok::<_, EventBuildError>(sse_event),
                    Err(e) => elog!(Error, "SSE → failed to serialize live event: {}", e),
                },
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    elog!(Warn, "SSE → consumer lagged, skipped {} events", n);
                    continue;
                }
                Err(_) => break,
            }
        }
    };

    let mut response = Sse::new(stream).with_keep_alive(KeepAlive::new()).into_response();

    if let Ok(val) = server_epoch.to_string().parse() {
        response.headers_mut().insert("x-sse-epoch", val);
    }

    response
}
