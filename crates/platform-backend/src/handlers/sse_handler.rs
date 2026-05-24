use crate::elog;

use rama::http::{Request, Response};
use rama::http::service::web::response::{Sse, IntoResponse};
use rama::http::sse::{Event, EventBuildError};
use rama::http::service::web::extract::State;
use async_stream::stream;
use platform_core::BufferedEvent;
use crate::server::SharedState;
use crate::common;

/// GET /sse — SSE endpoint with buffered state replay
pub async fn sse_endpoint(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = common::extract_context(&req);
    elog!(Debug, "SSE → endpoint connected (client_id={})", ctx.client_id);

    // 1. Subscribe to the broadcast channel FIRST
    //    (so we don't miss events between Phase 1 and Phase 2)
    let mut rx = ctx.sse_broadcaster.subscribe();

    // 2. Generate async SSE stream
    let stream = stream! {
        // Phase 1: Replay buffered events (current state snapshot)
        let buffered = ctx.event_emitter.get_buffered_events();
        elog!(Debug, "SSE → replaying {} buffered events", buffered.len());
        for event in &buffered {
            if let Ok(sse_event) = build_sse_event(event) {
                yield Ok::<Event<String>, EventBuildError>(sse_event);
            }
        }
        elog!(Debug, "SSE → Phase 1 complete, entering live mode");

        // Phase 2: Live events from broadcast channel
        while let Ok(event) = rx.recv().await {
            if let Ok(sse_event) = build_sse_event(&event) {
                yield Ok::<Event<String>, EventBuildError>(sse_event);
            }
        }
    };

    // 3. Sse implements IntoResponse
    Sse::new(stream).into_response()
}

/// Build a Rama `Event<String>` from a `BufferedEvent`.
fn build_sse_event(event: &BufferedEvent) -> Result<Event<String>, EventBuildError> {
    Event::new()
        .with_data(event.payload.clone())
        .try_with_id(event.hash.clone())?
        .try_with_event(event.event_type.clone())
}
