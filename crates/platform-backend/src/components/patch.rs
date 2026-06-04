use rama::http::body::sse::datastar::EventData;

/// A single Datastar SSE event produced by a component.
/// Wraps Rama's `EventData` enum for loose coupling — any component
/// can produce any event type (PatchElements, PatchSignals, ExecuteScript).
pub struct PatchEntry {
    pub data: EventData,
}

/// Strategy interface for composable components.
/// Anything implementing `Patch` can be added to a `Shell`.
pub trait Patch {
    fn into_patches(self) -> Vec<PatchEntry>;
}
