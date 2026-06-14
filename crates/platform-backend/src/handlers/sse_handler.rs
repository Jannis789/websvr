use crate::elog;

use crate::context::SharedState;
use crate::utils::request::extract_context;
use rama::http::service::web::extract::State;
use rama::http::service::web::response::{IntoResponse, Sse};
use rama::http::body::sse::datastar::EventData;
use rama::http::sse::{Event, EventBuildError};
use rama::http::{BodyExtractExt, Request, Response};
use serde::Deserialize;
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

#[derive(Deserialize, Default)]
struct SseInit {
    #[serde(default)]
    seen: u64,
}

fn id_only_event(ver: u64) -> Result<Event<EventData>, EventBuildError> {
    Event::<EventData>::new().try_with_id(ver.to_string())
}

/// GET /sse und POST /sse — SSE endpoint mit Cache-Replay.
pub async fn sse_endpoint(State(_state): State<SharedState>, req: Request) -> Response {
    let ctx = extract_context(&req);
    let is_post = req.method() == "POST";
    let client_gen = query_u64(&req, "g");

    let client_ver = if is_post {
        let (_, body) = req.into_parts();
        body.try_into_json::<SseInit>().await.unwrap_or_default().seen
    } else {
        query_u64(&req, "v")
    };

    elog!(
        Info,
        "SSE → connect client_id={} method={} v={} g={}",
        ctx.client_id,
        if is_post { "POST" } else { "GET" },
        client_ver,
        client_gen,
    );

    let (rx, plan) = ctx.event_emitter.connect(client_ver, client_gen);
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
        let ver = event.ver();
        elog!(Info, "SSE → live event ver={} arriving", ver);
        match event.to_sse_event_with_id() {
            Ok(sse_event) => {
                elog!(Info, "SSE → live event ver={} SERIALIZED OK", ver);
                Some(Ok::<_, EventBuildError>(sse_event))
            }
            Err(e) => {
                elog!(Error, "SSE → live event ver={} serialization failed: {}", ver, e);
                None
            }
        }
    });

    let stream = replay_stream.chain(live_stream);

    let response = Sse::new(stream).into_response();

    elog!(Info, "SSE → response built for client_id={}", ctx.client_id);

    response
}
