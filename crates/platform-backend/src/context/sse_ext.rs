use crate::sse::BufferedEvent;
use rama::http::body::sse::datastar::EventData;
use rama::http::service::web::response::{IntoResponse, Sse};
use rama::http::sse::{Event, EventBuildError};

/// Build an SSE `Response` from multiple `BufferedEvent`s.
pub fn sse_response(events: &[BufferedEvent]) -> rama::http::Response {
    let events = events.to_vec();
    Sse::new(async_stream::stream! {
        for event in events {
            match event.to_sse_event() {
                Ok(e) => yield Ok::<Event<EventData>, EventBuildError>(e),
                Err(_) => continue,
            }
        }
    })
    .into_response()
}
