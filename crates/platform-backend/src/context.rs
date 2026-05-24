use platform_core::{ClientContext, BufferedEvent, compute_content_hash};
use rama::http::body::sse::datastar::PatchElements;
use rama::http::body::sse::EventDataWrite;
use crate::elog;
use crate::crypto;

/// Extension trait that adds Rama-specific SSE methods to `ClientContext`.
///
/// Defined in `platform-backend` because `platform-core` has no Rama
/// dependency.  This trait bridges the gap by wrapping Rama's native
/// types (`PatchElements`, etc.) into a `BufferedEvent` with an HMAC
/// hash, then broadcasting it.
pub trait ClientContextSseExt {
    /// Create a `BufferedEvent` from Rama's `PatchElements`, compute its
    /// HMAC hash, broadcast it to all SSE clients, and optionally buffer
    /// it for replay on reconnect.
    fn emit_patch(&self, data_to_hash: &str, patch: PatchElements, should_cache: bool);
}

impl ClientContextSseExt for ClientContext {
    fn emit_patch(&self, _data_to_hash: &str, patch: PatchElements, should_cache: bool) {
        // 1. Serialise the PatchElements payload to its SSE wire format FIRST
        let mut buf = Vec::new();
        if patch.write_data(&mut buf).is_err() {
            elog!(Error, "Failed to serialize PatchElements");
            return;
        }
        let payload = match String::from_utf8(buf) {
            Ok(s) => s,
            Err(_) => {
                elog!(Error, "PatchElements produced non-UTF-8 output");
                return;
            }
        };

        // 2. Compute deterministic HMAC-SHA256 hash from the FULL payload
        //    (selector + mode + elements), not just the HTML content.
        //    This ensures the hash changes when the event format changes
        //    (e.g. adding mode:inner), forcing the SW to invalidate its cache.
        let hash = compute_content_hash(&payload, crypto::hmac_secret());

        // 3. Wrap in BufferedEvent
        let event = BufferedEvent {
            hash,
            payload,
            event_type: "datastar-patch-elements".to_string(),
        };

        // 4. Broadcast to all connected SSE clients
        let _ = self.sse_broadcaster.broadcast(event.clone());

        // 5. Buffer for replay on SSE reconnect (only if should_cache)
        if should_cache {
            self.event_emitter.buffer_event(event);
        }
    }
}
