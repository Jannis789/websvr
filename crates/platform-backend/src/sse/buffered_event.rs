use serde::{Serialize, Deserialize};

/// Bridge between Rama's native SSE types and our HMAC hash for deduplication.
///
/// Rama's `PatchElements` / `PatchSignals` / `ExecuteScript` have no
/// field for a custom hash.  `BufferedEvent` wraps them together with
/// the deterministic HMAC-SHA256 hash (16 bytes / 128 bit, hex-encoded)
/// so the Service Worker can skip already-known content.
///
/// Defined in `platform-core` even though the payload is a Rama type;
/// the payload is stored as a generic `String` (the serialised SSE data)
/// to keep `platform-core` free of a Rama dependency.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedEvent {
    /// HMAC-SHA256 hash, truncated to 16 bytes (128 bit), hex-encoded.
    pub hash: String,
    /// The SSE payload (e.g. PatchElements data) as a raw string.
    pub payload: String,
    /// The Datastar event type as a string (e.g. "datastar-patch-elements").
    pub event_type: String,
}

impl BufferedEvent {
    /// Extract the CSS selector from the payload (e.g. "#content-body").
    /// The selector line looks like: `data: selector #content-body`
    pub fn extract_selector(&self) -> Option<String> {
        for line in self.payload.lines() {
            let trimmed = line.trim();
            // Line format: "data: selector #content-body"
            if trimmed.starts_with("data: selector ") {
                return Some(trimmed.strip_prefix("data: selector ")?.trim().to_string());
            }
        }
        None
    }
}
