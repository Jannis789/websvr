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
    crate::elog!(Info, "Test → test page requested (client_id={})", _ctx.client_id);
    html_response(include_str!("../../assets/templates/test.html"))
}

/// GET /test/run — Execute the automated hash-sync test sequence
///
/// Fires a comprehensive sequence of `BufferedEvent`s through the `SseBroadcaster`
/// and the `EventEmitter` buffer.  The test.html frontend receives these via SSE
/// and computes a score.
///
/// ## Test Phases
///
/// **Phase A – Fresh events (cache miss):**
///   Send 4 new PatchElements events with fresh hashes.
///   SW learns them; client should receive all 4.
///
/// **Phase B – Replayed events (same content, same hashes):**
///   Send the same 4 events again with same hashes.
///   Within the same SSE connection, these will be received (known_hashes
///   are fixed at connect time).  On the NEXT connection (after page reload),
///   the SW will send these hashes as known_hashes and the server will skip them.
///
/// **Phase C – Modified event (content changed, new hash):**
///   Same logical event (id=test-1), different content → different hash.
///   Should be received as a cache miss.
///
/// **Phase D – Out-of-order events:**
///   Events arrive in different order than buffered; each checked independently.
///
/// **Phase E – Buffer replay verification:**
///   Events buffered in EventEmitter should be available for Phase 1 replay.
///
/// **Phase F – Score marker:**
///   Final event with test-complete marker.
pub async fn test_run(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = common::extract_context(&req);
    let sse_broadcaster = ctx.sse_broadcaster.clone();
    let event_emitter = ctx.event_emitter.clone();
    crate::elog!(Ok, "Test → running comprehensive hash-sync test sequence");

    // Spawn the test sequence in the background; respond immediately with 204
    tokio::spawn(async move {
        let secret = platform_core::Config::global().hmac_secret.clone();
        let sleep = tokio::time::sleep;
        let ms = |n| tokio::time::Duration::from_millis(n);

        // ────────────────────────────────────────
        // Phase A: Fresh events (4 events)
        // ────────────────────────────────────────
        let _ = sse_broadcaster.broadcast(marker_event("phase-a-start", "Phase A: Fresh events (should all be cache misses)"));
        sleep(ms(100)).await;

        let fresh_events: Vec<(&str, &str)> = vec![
            ("<div id='test-1' class='test-pass' data-phase='A' data-type='fresh'>✅ Fresh A: Event 1 received</div>", "fresh-a-1"),
            ("<div id='test-2' class='test-pass' data-phase='A' data-type='fresh'>✅ Fresh A: Event 2 received</div>", "fresh-a-2"),
            ("<div id='test-3' class='test-pass' data-phase='A' data-type='fresh'>✅ Fresh A: Event 3 received</div>", "fresh-a-3"),
            ("<div id='test-4' class='test-pass' data-phase='A' data-type='fresh'>✅ Fresh A: Event 4 received</div>", "fresh-a-4"),
        ];

        for (html, _tag) in &fresh_events {
            let event = buffered_patch(html, &secret);
            // Buffer for replay AND broadcast live
            event_emitter.buffer_event(event.clone());
            let _ = sse_broadcaster.broadcast(event);
            sleep(ms(150)).await;
        }

        let _ = sse_broadcaster.broadcast(marker_event("phase-a-end", "Phase A complete — 4 fresh events sent, buffered in EventEmitter"));
        sleep(ms(300)).await;

        // ────────────────────────────────────────
        // Phase B: Replayed events (same content, same hashes)
        // These events have the same hashes as Phase A. Within this SSE
        // connection, known_hashes are fixed from the initial request,
        // so the server will still send them.  On the NEXT connection
        // (after the user clicks "Reload"), the SW will include these
        // hashes in known_hashes and the server's Phase 1 will skip them.
        // ────────────────────────────────────────
        let _ = sse_broadcaster.broadcast(marker_event("phase-b-start", "Phase B: Replayed events (should be skipped — already known)"));
        sleep(ms(100)).await;

        for (html, _tag) in &fresh_events {
            let event = buffered_patch(html, &secret);
            // Do NOT buffer again — these are duplicates
            let _ = sse_broadcaster.broadcast(event);
            sleep(ms(100)).await;
        }

        let _ = sse_broadcaster.broadcast(marker_event("phase-b-end", "Phase B complete — duplicates broadcast (SW should skip them)"));
        sleep(ms(300)).await;

        // ────────────────────────────────────────
        // Phase C: Modified event (same ID, different content → different hash)
        // ────────────────────────────────────────
        let _ = sse_broadcaster.broadcast(marker_event("phase-c-start", "Phase C: Modified event (content changed → cache miss)"));
        sleep(ms(100)).await;

        let modified_html = "<div id='test-1' class='test-pass' data-phase='C' data-type='modified'>✅ Phase C: Modified content — cache miss (different hash)</div>";
        let mod_event = buffered_patch(modified_html, &secret);
        event_emitter.buffer_event(mod_event.clone());
        let _ = sse_broadcaster.broadcast(mod_event);
        sleep(ms(150)).await;

        let _ = sse_broadcaster.broadcast(marker_event("phase-c-end", "Phase C complete — modified event sent"));
        sleep(ms(300)).await;

        // ────────────────────────────────────────
        // Phase D: Out-of-order events
        // ────────────────────────────────────────
        let _ = sse_broadcaster.broadcast(marker_event("phase-d-start", "Phase D: Out-of-order events (each checked independently)"));
        sleep(ms(100)).await;

        let ooo_events: Vec<&str> = vec![
            "<div id='ooo-3' class='test-pass' data-phase='D' data-type='out-of-order'>✅ Out-of-order: Event 3 (sent 1st)</div>",
            "<div id='ooo-1' class='test-pass' data-phase='D' data-type='out-of-order'>✅ Out-of-order: Event 1 (sent 2nd)</div>",
            "<div id='ooo-2' class='test-pass' data-phase='D' data-type='out-of-order'>✅ Out-of-order: Event 2 (sent 3rd)</div>",
        ];

        for html in &ooo_events {
            let event = buffered_patch(html, &secret);
            event_emitter.buffer_event(event.clone());
            let _ = sse_broadcaster.broadcast(event);
            sleep(ms(150)).await;
        }

        let _ = sse_broadcaster.broadcast(marker_event("phase-d-end", "Phase D complete — out-of-order events sent"));
        sleep(ms(300)).await;

        // ────────────────────────────────────────
        // Phase E: Buffer replay verification
        // ────────────────────────────────────────
        let buffered_count = event_emitter.get_buffered_events().len();
        let _ = sse_broadcaster.broadcast(marker_event(
            "phase-e-info",
            &format!("Phase E: EventEmitter buffer has {} events (for Phase 1 replay on SSE reconnect)", buffered_count),
        ));
        sleep(ms(100)).await;

        // ────────────────────────────────────────
        // Phase F: Non-PatchElements events
        // PatchSignals events — the SW only caches PatchElements hashes.
        // The server-side known_hashes check applies to all event types.
        // Verified by SW unit tests (non-PatchElements events not registered).
        // ────────────────────────────────────────
        let _ = sse_broadcaster.broadcast(marker_event("phase-f-start", "Phase F: Non-PatchElements (PatchSignals — SW should NOT cache its hash)"));
        sleep(ms(100)).await;

        // Send a PatchSignals event — SW should NOT register its hash
        let signals_event = platform_core::BufferedEvent {
            hash: compute_content_hash("{\"test_signal\": true}", &secret),
            payload: "{\"test_signal\": true}".to_string(),
            event_type: "datastar-patch-signals".to_string(),
        };
        let _ = sse_broadcaster.broadcast(signals_event);
        sleep(ms(100)).await;

        let _ = sse_broadcaster.broadcast(marker_event("phase-f-end", "Phase F complete — PatchSignals sent (SW should NOT cache)"));
        sleep(ms(300)).await;

        // ────────────────────────────────────────
        // Phase G: Completion marker + score trigger
        // ────────────────────────────────────────
        let _ = sse_broadcaster.broadcast(platform_core::BufferedEvent {
            hash: "test-complete".to_string(),
            payload: "<div id='test-complete' class='test-info' style='display:none'></div>".to_string(),
            event_type: "datastar-patch-elements".to_string(),
        });
        sleep(ms(100)).await;

        let _ = sse_broadcaster.broadcast(marker_event(
            "test-score",
            &format!(
                "✅ Test sequence complete. {} events buffered for replay. Check SW console for hash cache stats.",
                buffered_count
            ),
        ));
    });

    empty_response(StatusCode::NO_CONTENT)
}

/// Helper: create a `BufferedEvent` from an HTML snippet for PatchElements.
fn buffered_patch(html: &str, secret: &str) -> platform_core::BufferedEvent {
    let hash = compute_content_hash(html, secret);
    platform_core::BufferedEvent {
        hash,
        payload: html.to_string(),
        event_type: "datastar-patch-elements".to_string(),
    }
}

/// Helper: create a marker/info event (visible in test results).
fn marker_event(hash_suffix: &str, message: &str) -> platform_core::BufferedEvent {
    platform_core::BufferedEvent {
        hash: format!("marker-{}", hash_suffix),
        payload: format!(
            "<div class='test-info test-marker' data-marker='{}'>📋 {}</div>",
            hash_suffix, message
        ),
        event_type: "datastar-patch-elements".to_string(),
    }
}
