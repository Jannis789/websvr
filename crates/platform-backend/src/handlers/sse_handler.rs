use crate::elog;

use crate::context::SharedState;
use crate::utils::request::extract_context;
use rama::http::service::web::extract::State;
use rama::http::service::web::response::{IntoResponse, Sse};
use rama::http::sse::server::KeepAlive;
use rama::http::body::sse::datastar::EventData;
use rama::http::sse::{Event, EventBuildError};
use rama::http::{Request, Response};
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

fn id_only_event(ver: u64) -> Result<Event<EventData>, EventBuildError> {
    Event::<EventData>::new().try_with_id(ver.to_string())
}

/// GET /sse — SSE endpoint mit Cache-Replay.
pub async fn sse_endpoint(State(_state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    let client_ver = query_u64(&req, "v");
    let client_epoch = query_u64(&req, "e");
    let server_epoch = ctx.event_emitter.epoch();

    elog!(
        Debug,
        "SSE → connected (client_id={}, v={}, e={})",
        ctx.client_id,
        client_ver,
        client_epoch
    );

    let (rx, plan) = ctx.event_emitter.connect(client_ver, client_epoch);
    let (id_only, full) = plan.into_parts();

    // Phase 1+2: id_only + full replay
    let replay_stream = tokio_stream::iter(
        id_only
            .into_iter()
            .filter_map(|ver| match id_only_event(ver) {
                Ok(e) => Some(Ok::<_, EventBuildError>(e)),
                Err(e) => {
                    elog!(Error, "SSE → id_only failed for ver={}: {}", ver, e);
                    None
                }
            })
            .chain(full.into_iter().filter_map(|event| match event.to_sse_event_with_id() {
                Ok(e) => Some(Ok::<_, EventBuildError>(e)),
                Err(e) => {
                    elog!(Error, "SSE → full event failed: {}", e);
                    None
                }
            })),
    );

    // Phase 3: Live-Events via mpsc
    let live_stream = UnboundedReceiverStream::new(rx).filter_map(|event| {
        elog!(Debug, "SSE → live event ver={}", event.ver());
        match event.to_sse_event_with_id() {
            Ok(sse_event) => Some(Ok(sse_event)),
            Err(e) => {
                elog!(Error, "SSE → serialization failed: {}", e);
                None
            }
        }
    });

    let stream = replay_stream.chain(live_stream);

    let mut response = Sse::new(stream).with_keep_alive(KeepAlive::new()).into_response();

    if let Ok(val) = server_epoch.to_string().parse() {
        response.headers_mut().insert("x-sse-epoch", val);
    }

    response
}
