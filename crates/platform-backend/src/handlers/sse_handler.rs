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
/// Volle Events: `id:N\nevent:T\ndata:...\n\n`
/// Dedup:       `id:N\n\n`  (SW replays aus FIFO-Cache)
pub async fn sse_endpoint(State(_state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    let client_seen = query_u64(&req, "s");

    elog!(
        Info,
        "SSE → connect client_id={} seen={}",
        ctx.client_id,
        client_seen,
    );

    let rx = ctx.event_emitter.connect(client_seen as usize);

    // Live-Events via mpsc → raw SSE strings
    let live_stream = UnboundedReceiverStream::new(rx).map(|event| {
        let ver = event.ver();
        let is_dedup = event.is_dedup();
        let raw = event.to_sse_raw_string();
        elog!(
            Info,
            "SSE → {} ver={} ({} bytes)",
            if is_dedup { "dedup" } else { "live" },
            ver,
            raw.len(),
        );
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
