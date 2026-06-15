use std::convert::Infallible;

use crate::context::SharedState;
use crate::elog;
use crate::utils::request::extract_context;
use rama::http::header;
use rama::http::service::web::extract::State;
use rama::http::{Body, Request, Response};
use tokio_stream::wrappers::UnboundedReceiverStream;
use tokio_stream::StreamExt;

fn query_u64(req: &Request, key: &str) -> u64 {
    req.uri()
        .query()
        .and_then(|q| {
            q.split('&')
                .find_map(|p| p.strip_prefix(&format!("{key}=")))
                .and_then(|v| v.parse().ok())
        })
        .unwrap_or(0)
}

/// GET /sse und POST /sse — SSE endpoint mit raw byte stream.
/// Volles Event: `id:N\nevent:T\ndata:...\n\n`
/// Dedup Event: `id:N\n\n` (SW liest aus Cache)
pub async fn sse_endpoint(State(_state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    let client_max_id = query_u64(&req, "v");

    elog!(
        Info,
        "SSE → connect client_id={} max_id={}",
        ctx.client_id,
        client_max_id,
    );

    let rx = ctx.event_emitter.connect(client_max_id);

    let live_stream = UnboundedReceiverStream::new(rx).map(|event| {
        let raw = event.to_sse_raw_string();
        Ok::<_, Infallible>(raw)
    });

    let response = Response::builder()
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache")
        .header(header::CONNECTION, "keep-alive")
        .body(Body::from_stream(live_stream))
        .unwrap();

    elog!(Info, "SSE → response built for client_id={}", ctx.client_id);

    response
}
