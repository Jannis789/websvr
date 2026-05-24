use crate::client_context::ClientContext;
use crate::sse::BufferedEvent;
use crate::crypto::compute_content_hash;
use rama::http::body::sse::datastar::PatchElements;
use rama::http::body::sse::EventDataWrite;
use crate::elog;

/// Extension trait that adds SSE methods to `ClientContext`.
pub trait ClientContextSseExt {
    fn emit_patch(&self, _data_to_hash: &str, patch: PatchElements, should_cache: bool);
}

impl ClientContextSseExt for ClientContext {
    fn emit_patch(&self, _data_to_hash: &str, patch: PatchElements, should_cache: bool) {
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

        let hash = compute_content_hash(&payload, crate::crypto::hmac_secret());

        let event = BufferedEvent {
            hash,
            payload,
            event_type: "datastar-patch-elements".to_string(),
        };

        let _ = self.sse_broadcaster.broadcast(event.clone());

        if should_cache {
            self.event_emitter.buffer_event(event);
        }
    }
}
