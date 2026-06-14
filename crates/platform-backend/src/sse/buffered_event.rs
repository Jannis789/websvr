use std::sync::Arc;

use rama::http::body::sse::datastar::EventData;

/// A single SSE event with patch_ver for reconnect.
///
/// `data` is wrapped in `Arc<EventData>` so that clones (cache insert,
/// broadcast send, return to handler) share the underlying strings
/// instead of deep-copying. Wire serialization (`to_sse_event_*`) clones
/// out of the Arc — that only happens once per connected SSE client.
#[derive(Debug, Clone)]
pub struct BufferedEvent {
    ver: u64,
    pub(crate) data: Arc<EventData>,
}

impl BufferedEvent {
    pub fn new(data: EventData, patch_ver: u64) -> Self {
        Self {
            ver: patch_ver,
            data: Arc::new(data),
        }
    }

    /// The patch version of this event.
    pub fn ver(&self) -> u64 {
        self.ver
    }

    /// Content-equality check — compares `EventData` fields directly.
    /// `PatchElements` and `PatchSignals` both derive `PartialEq`.
    pub fn content_eq(&self, other: &EventData) -> bool {
        self.data.as_ref() == other
    }

    /// SSE wire format WITHOUT `id:` — used for live broadcast events.
    pub fn to_sse_event(
        &self,
    ) -> Result<rama::http::sse::Event<EventData>, rama::http::sse::EventBuildError> {
        self.data.as_ref().clone().try_into_sse_event()
    }

    /// SSE wire format WITH `id:` — used for replay on reconnect.
    pub fn to_sse_event_with_id(
        &self,
    ) -> Result<rama::http::sse::Event<EventData>, rama::http::sse::EventBuildError> {
        self.data
            .as_ref()
            .clone()
            .try_into_sse_event()
            .and_then(|e| e.try_with_id(self.ver.to_string()))
    }
}
