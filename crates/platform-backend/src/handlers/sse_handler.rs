use rama::http::{Request, Response};
use rama::http::service::web::response::{Sse, IntoResponse};
use rama::http::sse::{Event, EventBuildError};
use rama::http::service::web::extract::State;
use async_stream::stream;
use platform_core::BufferedEvent;
use crate::server::SharedState;
use crate::common;

/// GET /sse — SSE endpoint with hash-sync replay
pub async fn sse_endpoint(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = common::extract_context(&req);

    // 1. Parse known_hashes from query string
    let known_hashes = parse_known_hashes(&req);

    // 2. Subscribe to the broadcast channel
    let mut rx = ctx.sse_broadcaster.subscribe();

    // 3. Generate async SSE stream of Result<Event<String>, ...>
    let stream = stream! {
        // Phase 1: Replay buffered events (iterate, NOT drain!)
        for event in ctx.event_emitter.get_buffered_events() {
            if !known_hashes.contains(&event.hash) {
                if let Ok(sse_event) = build_sse_event(&event) {
                    yield Ok::<Event<String>, EventBuildError>(sse_event);
                }
            }
        }

        // Phase 2: Live events from broadcast channel
        while let Ok(event) = rx.recv().await {
            if !known_hashes.contains(&event.hash) {
                if let Ok(sse_event) = build_sse_event(&event) {
                    yield Ok::<Event<String>, EventBuildError>(sse_event);
                }
            }
        }
    };

    // 4. Sse implements IntoResponse — sets correct headers automatically
    Sse::new(stream).into_response()
}

/// Parse known_hashes from the request query string.
fn parse_known_hashes(req: &Request) -> Vec<String> {
    let query = req.uri().query().unwrap_or("");
    query
        .split('&')
        .find_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            if parts.next()? == "known_hashes" {
                Some(parts.next()?.split(',').map(|s| s.to_string()).collect())
            } else {
                None
            }
        })
        .unwrap_or_default()
}

/// Build a Rama `Event<String>` from a `BufferedEvent`.
///
/// Embeds the HMAC hash as the SSE event ID so the Service Worker
/// can extract and register it for hash-sync deduplication.
fn build_sse_event(event: &BufferedEvent) -> Result<Event<String>, EventBuildError> {
    Event::new()
        .with_data(event.payload.clone())
        .try_with_id(event.hash.clone())?
        .try_with_event(event.event_type.clone())
}
