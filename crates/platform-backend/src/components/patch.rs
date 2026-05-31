use rama::http::body::sse::datastar::PatchElements;

/// A single SSE patch: HTML data + Datastar patch config.
pub struct PatchEntry {
    pub elements: PatchElements,
}

/// Strategy interface for composable components.
/// Anything implementing `Patch` can be added to a `Shell`.
pub trait Patch {
    fn into_patches(self) -> Vec<PatchEntry>;
}
