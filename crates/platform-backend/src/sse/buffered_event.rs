use crate::crypto::compute_content_hash;
use rama::http::body::sse::datastar::EventData;
use rama::http::body::sse::EventDataWrite;

/// A single SSE event with content hash for SW dedup.
///
/// The hash is computed from the `write_data()` payload — the same
/// bytes that get serialized into the SSE `data:` lines.
/// The SW reads `id: <hash>` to populate its HASH_REGISTRY.
#[derive(Debug, Clone)]
pub struct BufferedEvent {
    pub hash: String,
    pub data: EventData,
}

impl BufferedEvent {
    pub fn new(data: EventData, secret: &str) -> Self {
        let payload = Self::serialize_payload(&data);
        let hash = compute_content_hash(&payload, secret);
        Self { hash, data }
    }

    fn serialize_payload(data: &EventData) -> String {
        let mut buf = Vec::new();
        match data {
            EventData::PatchElements(pe) => { let _ = pe.write_data(&mut buf); }
            EventData::ExecuteScript(es) => { let _ = es.write_data(&mut buf); }
            EventData::PatchSignals(ps) => { let _ = ps.write_data(&mut buf); }
        }
        String::from_utf8(buf).unwrap_or_default()
    }

    /// Build the SSE wire format using Datastar's `try_into_sse_event()`.
    pub fn to_sse_event(
        &self,
    ) -> Result<rama::http::sse::Event<EventData>, rama::http::sse::EventBuildError> {
        self.data.clone().try_into_sse_event()
    }
}
