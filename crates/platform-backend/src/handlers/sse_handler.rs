use crate::elog;

use crate::context::SharedState;
use crate::utils::request::extract_context;
use async_stream::stream;
use rama::http::body::sse::datastar::EventData;
use rama::http::service::web::extract::State;
use rama::http::service::web::response::{IntoResponse, Sse};
use rama::http::sse::{Event, EventBuildError};
use rama::http::{Request, Response};

/// GET /sse — SSE endpoint with state replay on reconnect
pub async fn sse_endpoint(State(_state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    elog!(Debug, "SSE → connected (client_id={})", ctx.client_id);

    let mut rx = ctx.event_emitter.subscribe();
    let state = ctx.event_emitter.get_state();

    let stream = stream! {
        // Phase 1: Replay current state (slots + signals)
        let sent = state.len();
        for event in &state {
            match event.to_sse_event() {
                Ok(sse_event) => {
                    yield Ok::<Event<EventData>, EventBuildError>(sse_event);
                }
                Err(e) => {
                    elog!(Error, "SSE → failed to serialize state: {}", e);
                }
            }
        }
        elog!(Debug, "SSE → replayed {} state events", sent);

        // Phase 2: Live events
        loop {
            match rx.recv().await {
                Ok(event) => match event.to_sse_event() {
                    Ok(sse_event) => {
                        yield Ok::<Event<EventData>, EventBuildError>(sse_event);
                    }
                    Err(e) => {
                        elog!(Error, "SSE → failed to serialize live event: {}", e);
                    }
                },
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    continue;
                }
                Err(_) => break,
            }
        }
    };

    Sse::new(stream).into_response()
}
