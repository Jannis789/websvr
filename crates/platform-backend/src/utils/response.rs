use rama::http::header;
use rama::http::StatusCode;

use crate::sse::BufferedEvent;

/// Common response type for all handlers.
pub type Response = rama::http::Response<rama::http::Body>;

pub fn empty_response(status: StatusCode) -> Response {
    let mut resp = Response::new(rama::http::Body::empty());
    *resp.status_mut() = status;
    resp
}

pub fn redirect(url: &str) -> Response {
    let mut resp = Response::new(rama::http::Body::empty());
    *resp.status_mut() = StatusCode::SEE_OTHER;
    resp.headers_mut()
        .insert(header::LOCATION, url.parse().expect("invalid header value"));
    resp
}

pub fn html_response(html: &str) -> Response {
    let mut resp = Response::new(rama::http::Body::from(html.to_string()));
    resp.headers_mut().insert(
        header::CONTENT_TYPE,
        "text/html; charset=utf-8".parse().expect("invalid header value"),
    );
    resp
}

/// Build a `text/event-stream` response from buffered SSE events.
/// Uses the Rama `Sse` helper to get correct framing and headers.
pub fn sse_response(events: &[BufferedEvent]) -> Response {
    use rama::futures::stream;
    use rama::http::service::web::response::{IntoResponse, Sse};
    use std::convert::Infallible;

    let sse_events: Vec<_> = events
        .iter()
        .filter_map(|e| e.to_sse_event_with_id().ok())
        .collect();

    let stream = stream::iter(sse_events.into_iter().map(Ok::<_, Infallible>));
    Sse::new(stream).into_response()
}
