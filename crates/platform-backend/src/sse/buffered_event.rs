use rama::http::body::sse::datastar::EventData;

/// A single SSE event with patch_ver for reconnect.
///
/// `patch_ver` is monotonically increasing per client.
/// Live events are sent WITHOUT `id:` — the SW only needs id on replay.
/// Replay events are sent WITH `id:` so the SW can cache by patch_ver.
#[derive(Debug, Clone)]
pub struct BufferedEvent {
    ver: u64,
    pub(crate) data: EventData,
}

impl BufferedEvent {
    pub fn new(data: EventData, patch_ver: u64) -> Self {
        Self { ver: patch_ver, data }
    }

    /// The patch version of this event.
    pub fn ver(&self) -> u64 {
        self.ver
    }

    /// Content-equality check — compares `EventData` fields directly.
    /// `PatchElements` and `PatchSignals` both derive `PartialEq`.
    pub fn content_eq(&self, other: &EventData) -> bool {
        &self.data == other
    }

    /// SSE wire format WITHOUT `id:` — used for live broadcast events.
    /// The client receives these in real-time and doesn't need an id
    /// because it hasn't cached them yet.
    pub fn to_sse_event(
        &self,
    ) -> Result<rama::http::sse::Event<EventData>, rama::http::sse::EventBuildError> {
        self.data.clone().try_into_sse_event()
    }

    /// SSE wire format WITH `id:` — used for replay on reconnect.
    /// The SW uses the id (patch_ver) to cache events and deduplicate
    /// on subsequent reconnects.
    pub fn to_sse_event_with_id(
        &self,
    ) -> Result<rama::http::sse::Event<EventData>, rama::http::sse::EventBuildError> {
        self.data
            .clone()
            .try_into_sse_event()
            .and_then(|e| e.try_with_id(self.ver.to_string()))
    }
}
