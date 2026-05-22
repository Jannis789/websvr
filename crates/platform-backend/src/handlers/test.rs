use rama::http::{Request, Response, StatusCode};
use rama::http::service::web::extract::State;
use platform_core::compute_content_hash;
use crate::server::SharedState;
use crate::common::{self, html_response, empty_response};

/// GET /test — E2E test harness page
pub async fn test_page(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let _ctx = common::extract_context(&req);
    html_response(include_str!("../../pages/test.html"))
}

/// GET /test/run — Execute the automated hash-sync test sequence
///
/// Fires a defined sequence of `BufferedEvents` through the `SseBroadcaster`.
/// The test.html frontend receives these via SSE and computes a score.
///
/// Sequence:
///  1. Send 3 new events (fresh content) → SW should cache hashes
///  2. Send the same 3 events again → SW should deduplicate via known_hashes
///  3. Send 1 event with a modified hash → SW should detect mismatch
pub async fn test_run(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = common::extract_context(&req);
    let sse_broadcaster = ctx.sse_broadcaster.clone();

    // Spawn the test sequence in the background; respond immediately with 204
    tokio::spawn(async move {
        let secret = platform_core::Config::global().hmac_secret.clone();

        // ── Phase A: Fresh events ──
        let events_a = vec![
            ("<div id='test-1' class='test-pass'>Test 1: Fresh event A</div>", "test-a"),
            ("<div id='test-2' class='test-pass'>Test 2: Fresh event B</div>", "test-b"),
            ("<div id='test-3' class='test-pass'>Test 3: Fresh event C</div>", "test-c"),
        ];

        for (html, _tag) in &events_a {
            let hash = compute_content_hash(html, &secret);
            let event = platform_core::BufferedEvent {
                hash,
                payload: html.to_string(),
                event_type: "datastar-patch-elements".to_string(),
            };
            let _ = sse_broadcaster.broadcast(event);
            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
        }

        // ── Phase B: Replayed events (same content, same hashes) ──
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        let _ = sse_broadcaster.broadcast(platform_core::BufferedEvent {
            hash: "phase-marker".to_string(),
            payload: "<div id='test-phase' class='test-info'>Phase B: Replayed events (should be cached)</div>".to_string(),
            event_type: "datastar-patch-elements".to_string(),
        });

        for (html, _tag) in &events_a {
            let hash = compute_content_hash(html, &secret);
            let event = platform_core::BufferedEvent {
                hash,
                payload: html.to_string(),
                event_type: "datastar-patch-elements".to_string(),
            };
            let _ = sse_broadcaster.broadcast(event);
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // ── Phase C: Modified event (different content, new hash) ──
        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        let _ = sse_broadcaster.broadcast(platform_core::BufferedEvent {
            hash: "phase-marker-c".to_string(),
            payload: "<div id='test-phase' class='test-info'>Phase C: Modified event (should be a cache miss)</div>".to_string(),
            event_type: "datastar-patch-elements".to_string(),
        });

        let modified_html = "<div id='test-1' class='test-pass'>Test 1: MODIFIED content — cache miss!</div>";
        let modified_hash = compute_content_hash(modified_html, &secret);
        let _ = sse_broadcaster.broadcast(platform_core::BufferedEvent {
            hash: modified_hash,
            payload: modified_html.to_string(),
            event_type: "datastar-patch-elements".to_string(),
        });

        // ── Completion marker ──
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        let _ = sse_broadcaster.broadcast(platform_core::BufferedEvent {
            hash: "test-complete".to_string(),
            payload: "<div id='test-complete' class='test-info'>✅ Test sequence complete — check the SW console for hash cache stats.</div>".to_string(),
            event_type: "datastar-patch-elements".to_string(),
        });
    });

    empty_response(StatusCode::NO_CONTENT)
}
