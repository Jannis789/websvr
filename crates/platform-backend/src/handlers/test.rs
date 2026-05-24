use crate::elog;
use rama::http::{Request, Response, StatusCode};
use rama::http::service::web::extract::State;
use crate::crypto::compute_content_hash;
use crate::server::SharedState;
use crate::utils::request::{extract_context};
use crate::utils::response::{empty_response, html_response};

/// GET /test — E2E test harness page
pub async fn test_page(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let _ctx = extract_context(&req);
    elog!(Debug, "Test → test page requested (client_id={})", _ctx.client_id);
    html_response(include_str!("../../assets/templates/test.html"))
}

/// GET /test/run — Execute the automated hash-sync test sequence
///
/// Fires all events immediately (no artificial delays).
/// The broadcast channel guarantees ordering.
/// The TS test harness connects to SSE first, then triggers this endpoint,
/// and collects events until the "test-complete" marker arrives.
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
/// **Phase F – Non-PatchElements events:**
///   PatchSignals event — SW should NOT register its hash.
///
/// **Phase G – Completion marker:**
///   Final event with "test-complete" hash, used by TS tests as Promise signal.
pub async fn test_run(
    State(_state): State<SharedState>,
    req: Request,
) -> Response {
    let ctx = extract_context(&req);
    let sse_broadcaster = ctx.sse_broadcaster.clone();
    let event_emitter = ctx.event_emitter.clone();
    elog!(Info, "Test → running hash-sync test sequence");

    // Spawn the test sequence in the background; respond immediately with 204
    tokio::spawn(async move {
        let secret = platform_core::Config::global().hmac_secret.clone();

        // ────────────────────────────────────────
        // Phase A: Fresh events (4 events)
        // ────────────────────────────────────────
        let _ = sse_broadcaster.broadcast(marker_event("phase-a-start", "Phase A: Fresh events"));

        let fresh_events: Vec<&str> = vec![
            "<div id='test-1' class='test-pass' data-phase='A' data-type='fresh'>✅ Fresh A: Event 1 received</div>",
            "<div id='test-2' class='test-pass' data-phase='A' data-type='fresh'>✅ Fresh A: Event 2 received</div>",
            "<div id='test-3' class='test-pass' data-phase='A' data-type='fresh'>✅ Fresh A: Event 3 received</div>",
            "<div id='test-4' class='test-pass' data-phase='A' data-type='fresh'>✅ Fresh A: Event 4 received</div>",
        ];

        for html in &fresh_events {
            let event = buffered_patch(html, &secret);
            event_emitter.buffer_event(event.clone());
            let _ = sse_broadcaster.broadcast(event);
        }

        let _ = sse_broadcaster.broadcast(marker_event("phase-a-end", "Phase A complete"));

        // ────────────────────────────────────────
        // Phase B: Replayed events (same content, same hashes)
        // ────────────────────────────────────────
        let _ = sse_broadcaster.broadcast(marker_event("phase-b-start", "Phase B: Replayed events"));

        for html in &fresh_events {
            let event = buffered_patch(html, &secret);
            let _ = sse_broadcaster.broadcast(event);
        }

        let _ = sse_broadcaster.broadcast(marker_event("phase-b-end", "Phase B complete"));

        // ────────────────────────────────────────
        // Phase C: Modified event (same ID, different content → different hash)
        // ────────────────────────────────────────
        let _ = sse_broadcaster.broadcast(marker_event("phase-c-start", "Phase C: Modified event"));

        let modified_html = "<div id='test-1' class='test-pass' data-phase='C' data-type='modified'>✅ Phase C: Modified content — cache miss (different hash)</div>";
        let mod_event = buffered_patch(modified_html, &secret);
        event_emitter.buffer_event(mod_event.clone());
        let _ = sse_broadcaster.broadcast(mod_event);

        let _ = sse_broadcaster.broadcast(marker_event("phase-c-end", "Phase C complete"));

        // ────────────────────────────────────────
        // Phase D: Out-of-order events
        // ────────────────────────────────────────
        let _ = sse_broadcaster.broadcast(marker_event("phase-d-start", "Phase D: Out-of-order events"));

        let ooo_events: Vec<&str> = vec![
            "<div id='ooo-3' class='test-pass' data-phase='D' data-type='out-of-order'>✅ Out-of-order: Event 3 (sent 1st)</div>",
            "<div id='ooo-1' class='test-pass' data-phase='D' data-type='out-of-order'>✅ Out-of-order: Event 1 (sent 2nd)</div>",
            "<div id='ooo-2' class='test-pass' data-phase='D' data-type='out-of-order'>✅ Out-of-order: Event 2 (sent 3rd)</div>",
        ];

        for html in &ooo_events {
            let event = buffered_patch(html, &secret);
            event_emitter.buffer_event(event.clone());
            let _ = sse_broadcaster.broadcast(event);
        }

        let _ = sse_broadcaster.broadcast(marker_event("phase-d-end", "Phase D complete"));

        // ────────────────────────────────────────
        // Phase E: Buffer replay verification
        // ────────────────────────────────────────
        let buffered_count = event_emitter.get_buffered_events().len();
        let _ = sse_broadcaster.broadcast(marker_event(
            "phase-e-info",
            &format!("Phase E: EventEmitter buffer has {} events", buffered_count),
        ));

        // ────────────────────────────────────────
        // Phase F: Non-PatchElements events
        // ────────────────────────────────────────
        let _ = sse_broadcaster.broadcast(marker_event("phase-f-start", "Phase F: PatchSignals"));

        let signals_event = crate::sse::BufferedEvent {
            hash: compute_content_hash("{\"test_signal\": true}", &secret),
            payload: "{\"test_signal\": true}".to_string(),
            event_type: "datastar-patch-signals".to_string(),
        };
        let _ = sse_broadcaster.broadcast(signals_event);

        let _ = sse_broadcaster.broadcast(marker_event("phase-f-end", "Phase F complete"));

        // ────────────────────────────────────────
        // Phase G: Completion marker — TS tests await this hash
        // ────────────────────────────────────────
        let _ = sse_broadcaster.broadcast(crate::sse::BufferedEvent {
            hash: "test-complete".to_string(),
            payload: "<div id='test-complete' class='test-info' style='display:none'></div>".to_string(),
            event_type: "datastar-patch-elements".to_string(),
        });

        let _ = sse_broadcaster.broadcast(marker_event(
            "test-score",
            &format!(
                "✅ Test sequence complete. {} events buffered for replay.",
                buffered_count
            ),
        ));
    });

    empty_response(StatusCode::NO_CONTENT)
}

/// Helper: create a `BufferedEvent` from an HTML snippet for PatchElements.
fn buffered_patch(html: &str, secret: &str) -> crate::sse::BufferedEvent {
    let hash = compute_content_hash(html, secret);
    crate::sse::BufferedEvent {
        hash,
        payload: html.to_string(),
        event_type: "datastar-patch-elements".to_string(),
    }
}

/// Helper: create a marker/info event (visible in test results).
fn marker_event(hash_suffix: &str, message: &str) -> crate::sse::BufferedEvent {
    crate::sse::BufferedEvent {
        hash: format!("marker-{}", hash_suffix),
        payload: format!(
            "<div class='test-info test-marker' data-marker='{}'>📋 {}</div>",
            hash_suffix, message
        ),
        event_type: "datastar-patch-elements".to_string(),
    }
}