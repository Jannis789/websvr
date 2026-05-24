use crate::elog;

use rama::http::{Request, Response};
use rama::http::service::web::response::{Sse, IntoResponse};
use rama::http::sse::{Event, EventBuildError};
use rama::http::service::web::extract::State;
use async_stream::stream;
use crate::sse::BufferedEvent;
use crate::server::SharedState;
use crate::utils::request::{extract_context};


/// GET /sse — SSE endpoint with buffered state replay
pub async fn sse_endpoint(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = extract_context(&req);

    // Parse known_hashes from query string
    let known_hashes = parse_known_hashes(&req);
    elog!(Debug, "SSE → connected (client_id={}, known={})", ctx.client_id, known_hashes.len());

    // 1. Subscribe to the broadcast channel FIRST
    let mut rx = ctx.sse_broadcaster.subscribe();

    // 2. Generate async SSE stream
    let stream = stream! {
        // Phase 1: Replay buffered events, skip known ones
        let buffered = ctx.event_emitter.get_buffered_events();
        let mut sent = 0;
        let mut skipped = 0;
        for event in &buffered {
            if known_hashes.contains(&event.hash) {
                skipped += 1;
                continue;
            }
            if let Ok(sse_event) = build_sse_event(event) {
                yield Ok::<Event<String>, EventBuildError>(sse_event);
                sent += 1;
            }
        }
        elog!(Debug, "SSE → Phase 1: sent {}, skipped {} (known)", sent, skipped);

        // Phase 2: Live events from broadcast channel
        while let Ok(event) = rx.recv().await {
            if known_hashes.contains(&event.hash) {
                continue;
            }
            if let Ok(sse_event) = build_sse_event(&event) {
                yield Ok::<Event<String>, EventBuildError>(sse_event);
            }
        }
    };

    // 3. Sse implements IntoResponse
    Sse::new(stream).into_response()
}

/// Parse `known_hashes` from the query string.
/// Format: `?known_hashes=hash1,hash2,hash3`
fn parse_known_hashes(req: &Request) -> std::collections::HashSet<String> {
    let mut set = std::collections::HashSet::new();
    if let Some(query) = req.uri().query() {
        for pair in query.split('&') {
            if let Some(value) = pair.strip_prefix("known_hashes=") {
                for hash in value.split(',') {
                    let h = hash.trim();
                    if !h.is_empty() {
                        set.insert(h.to_string());
                    }
                }
            }
        }
    }
    set
}

/// Build a Rama `Event<String>` from a `BufferedEvent`.
fn build_sse_event(event: &BufferedEvent) -> Result<Event<String>, EventBuildError> {
    Event::new()
        .with_data(event.payload.clone())
        .try_with_id(event.hash.clone())?
        .try_with_event(event.event_type.clone())
}